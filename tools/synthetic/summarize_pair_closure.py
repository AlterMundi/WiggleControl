#!/usr/bin/env python3
"""Score paired closure holds and observable ownership against independent rig geometry."""
import argparse
import hashlib
import itertools
import json
import math
from pathlib import Path


def associate(centers, references):
    """Associate only with spatial margin; screen overlap remains unscored."""
    if not centers or len(centers) > 2 or len(references) != 2:
        return {}
    if (any(r['width'] <= 2 or not math.isfinite(r['width']) for r in references)
            or any(not math.isfinite(v) for c in centers+[r['center'] for r in references] for v in c)):
        return {}
    scale = max(r['width'] for r in references)
    if scale <= 2 or math.dist(references[0]['center'], references[1]['center']) < 0.8*scale:
        return {}
    options = []
    for targets in itertools.permutations(range(2), len(centers)):
        distances = [math.dist(c, references[t]['center'])/references[t]['width']
                     for c, t in zip(centers, targets)]
        options.append((sum(distances), max(distances), targets))
    options.sort()
    if options[0][1] > 0.75 or options[1][0]-options[0][0] <= 0.5:
        return {}
    return {i: references[t]['id'] for i, t in enumerate(options[0][2])}


def summarize(path, config_path):
    data = json.loads(path.read_text())
    if data['camera_opened'] or data['system_input'] or not data['reference'].get('paired_closure'):
        raise ValueError('Expected an offline paired-closure experiment')
    config_bytes = config_path.read_bytes()
    if data.get('config_sha256') != hashlib.sha256(config_bytes).hexdigest():
        raise ValueError('Configuration digest differs from the experiment')
    workspace = json.loads(config_bytes).get('touch_workspace')
    if workspace is not None and workspace != {'min': {'x': 0, 'y': 0}, 'max': {'x': 1, 'y': 1}}:
        raise ValueError('Contact association requires the full uncalibrated workspace')
    labels, predictions = data['reference']['frames'], data['predictions']
    if not labels or len(labels) != len(predictions):
        raise ValueError('Reference/prediction frame counts differ')
    width, height = data['reference']['image_size']
    reference_ids = {h['id'] for h in labels[0]['reference_hands']}
    if len(reference_ids) != 2:
        raise ValueError('Expected two distinct reference hands')
    maximum = {h['id']: max(ref['closure_interpolation'] for f in labels
                            for ref in f['reference_hands'] if ref['id'] == h['id'])
               for h in labels[0]['reference_hands']}
    holds, trace, track_changes, owner_changes = {}, [], [], []
    last_tracks, contact_owners = {}, {}
    last_contacts, onsets, releases = set(), [], []
    unscored_tracks, unscored_contacts, open_reference_contacts = [], [], []
    for index, (label, prediction) in enumerate(zip(labels, predictions)):
        if (label['frame'] != index or prediction['frame'] != index
                or label['source_us'] != prediction['source_us']):
            raise ValueError('Reference/prediction frame order or time differs')
        if (len(label['reference_hands']) != 2
                or {h['id'] for h in label['reference_hands']} != reference_ids):
            raise ValueError('Reference hand identities changed')
        references = []
        for hand in label['reference_hands']:
            p = hand['image_px']
            references.append(dict(id=hand['id'], center=[(p[5][j]+p[17][j])/2 for j in (0, 1)],
                                   width=math.dist(p[5], p[17])))
        raw = prediction['perception']
        if len({h['id'] for h in raw}) != len(raw):
            raise ValueError('Duplicate model track IDs')
        centers = [[(h['landmarks'][5][j]+h['landmarks'][17][j])/2 for j in (0, 1)] for h in raw]
        matches = associate(centers, references)
        if raw and not matches:
            unscored_tracks.append(index)
        for i, reference in matches.items():
            track = raw[i]['id']
            previous = last_tracks.get(reference)
            if previous and previous['id'] != track:
                track_changes.append(dict(frame=index, reference=reference, before=previous['id'],
                                          after=track, across_gap=previous['frame'] != index-1))
            last_tracks[reference] = dict(id=track, frame=index)
        contacts = prediction['contacts']
        if len({c['id'] for c in contacts}) != len(contacts):
            raise ValueError('Duplicate contact IDs')
        # This benchmark uses the explicit full workspace, with the native
        # mirrored-X convention. It must not interpret calibrated coordinates.
        contact_centers = [[(1-c['x'])*width, c['y']*height] for c in contacts]
        contact_matches = associate(contact_centers, references)
        if contacts and not contact_matches:
            unscored_contacts.append(index)
        for i, reference in contact_matches.items():
            cid = contacts[i]['id']
            previous = contact_owners.get(cid)
            if previous and previous['reference'] != reference:
                owner_changes.append(dict(frame=index, contact_id=cid, before=previous['reference'],
                                          after=reference, across_gap=previous['frame'] != index-1))
            contact_owners[cid] = dict(reference=reference, frame=index)
            hand = next(h for h in label['reference_hands'] if h['id'] == reference)
            if hand['closure_interpolation'] <= 1e-9:
                open_reference_contacts.append(dict(frame=index, reference=reference, contact_id=cid))
        active = {c['id'] for c in contacts}
        for i, contact in enumerate(contacts):
            if contact['id'] in last_contacts:
                continue
            reference = contact_matches.get(i)
            hand = next((h for h in label['reference_hands'] if h['id'] == reference), None)
            onsets.append(dict(frame=index, source_us=label['source_us'], contact_id=contact['id'],
                               reference=reference,
                               distal_skin_gap_mm=hand['distal_skin']['surface_gap_m']*1000 if hand else None))
        for cid in sorted(last_contacts-active):
            releases.append(dict(frame=index, contact_id=cid,
                                 reference=contact_owners.get(cid, {}).get('reference'),
                                 ambiguous=prediction['ambiguous'],
                                 ambiguity_reason=prediction.get('ambiguity_reason')))
        last_contacts = active
        pose = [h['closure_interpolation'] for h in label['reference_hands']]
        states = [('open' if x <= 1e-9 else 'near' if math.isclose(x, maximum[h['id']], abs_tol=1e-9)
                   else 'transition') for x, h in zip(pose, label['reference_hands'])]
        stable = 'transition' not in states
        if stable:
            key = '/'.join(states)
            stats = holds.setdefault(key, dict(frames=0, matching_count=0, two_hands=0,
                                               expected_contacts=states.count('near')))
            stats['frames'] += 1
            stats['matching_count'] += len(contacts) == stats['expected_contacts']
            stats['two_hands'] += len(raw) == 2
        trace.append(dict(frame=index, reference_states=states, stable_hold=stable,
                          track_assignments={reference: raw[i]['id'] for i, reference in matches.items()},
                          contact_assignments={reference: contacts[i]['id'] for i, reference in contact_matches.items()},
                          contacts=len(contacts), ambiguous=prediction['ambiguous']))
    return dict(layout=data['reference']['pair_layout'], frames=len(labels), holds=holds,
        two_hand_frames=sum(len(p['hands']) == 2 for p in predictions),
        ambiguous_frames=sum(p['ambiguous'] for p in predictions),
        track_ids=sorted({h['id'] for p in predictions for h in p['perception']}),
        skin_in_frame=all(f['skin_in_frame'] for f in labels),
        separated_skin_depth_ranges=all(f['separated_skin_depth_ranges'] for f in labels),
        selected_patch_intersections=[dict(frame=f['frame'], hand=h['id']) for f in labels
                                      for h in f['reference_hands'] if h['distal_skin']['surface_intersection']],
        track_changes=track_changes, contact_owner_changes=owner_changes,
        contact_onsets=onsets, contact_releases=releases,
        open_reference_contacts=open_reference_contacts,
        unscored_track_association_frames=unscored_tracks,
        unscored_contact_association_frames=unscored_contacts,
        report_sha256=hashlib.sha256(path.read_bytes()).hexdigest(), trace=trace)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', type=Path, nargs='+')
    parser.add_argument('--config', type=Path, required=True,
                        help='Exact offline config, checked against each report digest')
    args = parser.parse_args()
    print(json.dumps(dict(schema='wigglecontrol.pair-closure-summary/v1',
        rows=[summarize(p, args.config) for p in args.reports], camera_opened=False, system_input=False,
        association='Independent projected palm centers; <=0.75 reference-palm widths, best/second cost margin>0.5; centers closer than0.8 widths remain unscored.',
        workspace='Full normalized workspace only; native mirrored X. Calibrated contact positions unsupported.',
        limits='Articulation holds are reference poses, not human intent. Occluded/overlapping associations are unscored rather than treated as identity correctness.'), indent=2))


if __name__ == '__main__':
    main()
