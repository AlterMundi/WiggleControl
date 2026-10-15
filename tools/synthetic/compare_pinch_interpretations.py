#!/usr/bin/env python3
"""Compare pinch metrics on retained predictions through the actual Rust controller.

No inference, camera, rendering or OS input. World or optional pixel palm-length
trials encode a unitless ratio as ratio*0.1 in the existing replay distance field,
with correspondingly encoded thresholds. Those values are NOT corrected physical meters. This is an
offline interpretation probe, not a production metric or threshold selection.
"""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import subprocess
import tempfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def matched_geometry(reference, measured):
    """Reject a replacement skin reference whose full marker trajectory differs."""
    for key in ('asset_sha256', 'pose_recipe', 'target_joint_angles_rad',
                'macro_abduction_target_rad', 'camera_world_matrix', 'pinch_pose_sha256'):
        if reference.get(key) != measured.get(key):
            raise ValueError('Geometry metadata mismatch: '+key)
    if len(reference['frames']) != len(measured['frames']):
        raise ValueError('Geometry frame count mismatch')
    maximum = 0.0
    for first, second in zip(reference['frames'], measured['frames']):
        for key in ('frame', 'source_us', 'closure_interpolation'):
            if first[key] != second[key]:
                raise ValueError('Geometry frame mismatch: '+key)
        if len(first['world_m']) != len(second['world_m']):
            raise ValueError('Geometry marker count mismatch')
        errors = [math.dist(a, b) for a, b in zip(first['world_m'], second['world_m'])]
        if not all(math.isfinite(e) and e <= 1e-6 for e in errors):
            raise ValueError('Geometry markers changed')
        maximum = max(maximum, *errors)
        first_hands = first.get('reference_hands', [])
        second_hands = second.get('reference_hands', [])
        if len(first_hands) != len(second_hands):
            raise ValueError('Paired geometry hand count changed')
        for original_hand, measured_hand in zip(first_hands, second_hands):
            if original_hand['id'] != measured_hand['id']:
                raise ValueError('Paired geometry hand identity changed')
            a, b = original_hand['world_m'], measured_hand['world_m']
            if len(a) != len(b):
                raise ValueError('Paired geometry marker count changed')
            errors = [math.dist(p,q) for p,q in zip(a,b)]
            if not errors or not all(math.isfinite(e) and e <= 1e-6 for e in errors):
                raise ValueError('Paired geometry markers changed')
            if 'distal_skin' not in measured_hand:
                raise ValueError('Missing independently measured paired skin')
            maximum = max(maximum,*errors)
        if 'distal_skin' not in second:
            raise ValueError('Missing independently measured skin')
    return measured, maximum


def encoded_hands(row, normalized, onset_screen_ratio=None):
    hands = copy.deepcopy(row['hands'])
    if normalized or onset_screen_ratio is not None:
        if normalized and normalized is not True and normalized != 'screen':
            raise ValueError('Unknown normalized metric')
        raw = {h['id']: h for h in row['perception']}
        for hand in hands:
            points = raw[hand['id']]['landmarks' if normalized == 'screen' or onset_screen_ratio is not None
                                      else 'world_landmarks']
            palm = math.dist(points[0], points[9])
            gap = math.dist(points[4], points[8])
            if not math.isfinite(palm) or palm <= 1e-6 or not math.isfinite(gap):
                raise ValueError('Unqualified palm/gap')
            if onset_screen_ratio is not None:
                hand['pinch_onset_allowed'] = gap/palm <= onset_screen_ratio
            else:
                hand['pinch_distance_m'] = gap/palm*0.1
    return hands


def replay(binary, settings, rows, normalized, close, open_, stride, offset=0, confirm_us=40000,
           onset_screen_ratio=None):
    selected = rows[offset::stride]
    config = copy.deepcopy(settings)
    config['gestures'].update(close_m=close, open_m=open_, confirm_us=confirm_us)
    commands = [dict(type='start')]
    for row in selected:
        commands.append(dict(type='frame', now_us=row['source_us'], observation=dict(
            version=1, source_us=row['source_us'], ambiguous=row['ambiguous'],
            hands=encoded_hands(row, normalized, onset_screen_ratio))))
    commands.append(dict(type='stop'))
    with tempfile.TemporaryDirectory(prefix='wigglecontrol-pinch-probe-') as directory:
        config_path = Path(directory)/'settings.json'
        config_path.write_text(json.dumps(config, allow_nan=False))
        result = subprocess.run([str(binary), 'mode-replay', str(config_path)],
                                input=''.join(json.dumps(c, allow_nan=False)+'\n' for c in commands),
                                text=True, capture_output=True, check=True, timeout=30)
    output = [json.loads(line) for line in result.stdout.splitlines()]
    # mode-replay emits one receipt per command and a final release at EOF.
    if len(output) != len(commands)+1 or any(
            x['camera_opened'] or x['system_input'] for x in output):
        raise ValueError('Unexpected replay output contract')
    if any(o['output']['contacts'] or o['output']['button_down'] for o in output[-2:]):
        raise ValueError('Stop did not release input')
    return [(r, o['output']) for r, o in zip(selected, output[1:-2])]


def score(reference, replayed):
    frames = {f['frame']: f for f in reference['frames']}
    measured = []
    was_contact = False
    onsets = []
    for row, output in replayed:
        label = frames[row['frame']]
        if label['source_us'] != row['source_us'] or len(row['hands']) > 1:
            raise ValueError('Probe requires matching single-hand source frames')
        skin = label['distal_skin']
        gap = skin['surface_gap_m']
        if skin['surface_intersection'] or not math.isfinite(gap) or gap < 0:
            raise ValueError('Skin geometry is unqualified')
        contact = bool(output['contacts'])
        if contact and not was_contact:
            onsets.append(dict(frame=row['frame'], source_us=row['source_us'], skin_gap_m=gap))
        was_contact = contact
        measured.append(dict(frame=row['frame'], source_us=row['source_us'],
                             skin_gap_m=gap, contact=contact,
                             closure=label['closure_interpolation']))
    near = [f for f in measured if f['skin_gap_m'] <= 0.003 and f['closure'] > 0.999]
    open_ = [f for f in measured if f['closure'] == 0]
    separated = [f for f in measured if f['skin_gap_m'] >= 0.050]
    return dict(frames=len(measured), contact_frames=sum(f['contact'] for f in measured),
                close_hold_frames=len(near), close_hold_contacts=sum(f['contact'] for f in near),
                open_hold_frames=len(open_), open_hold_contacts=sum(f['contact'] for f in open_),
                separated_50mm_frames=len(separated),
                separated_50mm_contacts=sum(f['contact'] for f in separated),
                contact_onsets=onsets, stop_released=True, trajectory=measured)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('results', type=Path, nargs='+')
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--settings', type=Path, required=True)
    parser.add_argument('--geometry', type=Path,
                        help='Replacement labels for ONE result only; full geometry must match')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--screen-ratio', type=float, nargs=2, metavar=('CLOSE', 'OPEN'),
                        help='Compare raw vs one fixed pixel-gap/pixel-palm ratio, instead of the world grid')
    parser.add_argument('--onset-screen-ratio', type=float,
                        help='Veto new presses above this pixel ratio; keep raw confirmation/release/rearm')
    args = parser.parse_args()
    if args.onset_screen_ratio is not None and (
            args.screen_ratio or not 0 < args.onset_screen_ratio <= 2):
        parser.error('Onset ratio requires0<ratio<=2 and is separate from --screen-ratio')
    if args.screen_ratio and not (0 < args.screen_ratio[0] < args.screen_ratio[1] <= 2):
        parser.error('Screen ratios require0<CLOSE<OPEN<=2')
    if args.geometry and len(args.results) != 1:
        parser.error('--geometry requires exactly one results file')
    inputs = [args.binary, args.settings, *args.results, *([args.geometry] if args.geometry else [])]
    hashes = {str(p): digest(p) for p in inputs}
    settings = json.loads(args.settings.read_text())
    reports = []
    for path in args.results:
        data = json.loads(path.read_text())
        if data['camera_opened'] or data['system_input']:
            raise ValueError('Expected camera-free, device-free source')
        reference, error = data['reference'], None
        if args.geometry:
            reference, error = matched_geometry(reference, json.loads(args.geometry.read_text()))
        rows = data['predictions']
        if any(not h.get('pinch_onset_allowed', True) for row in rows for h in row['hands']):
            raise ValueError('Expected ungated source observations for the raw baseline')
        if ([f['frame'] for f in reference['frames']] != [r['frame'] for r in rows]
                or not all(b['source_us'] > a['source_us'] for a, b in zip(rows, rows[1:]))):
            raise ValueError('Expected complete ordered prediction trajectory')
        cases = []
        original_confirm = settings['gestures'].get('confirm_us', 40000)
        for stride, offset in ((1, 0), (2, 0), (2, 1)):
            candidates = [('raw_meters', False, settings['gestures']['close_m'],
                           settings['gestures']['open_m'], original_confirm)]
            if args.onset_screen_ratio is not None:
                candidates.append(('screen_onset_veto', False, settings['gestures']['close_m'],
                                   settings['gestures']['open_m'], original_confirm))
            elif args.screen_ratio:
                candidates.append(('screen_palm_ratio', 'screen', args.screen_ratio[0]*0.1,
                                   args.screen_ratio[1]*0.1, original_confirm))
            else:
                candidates += [('palm_length_ratio', True, close*0.1, close*1.5*0.1, confirm)
                               for close in (0.30, 0.35, 0.38, 0.40, 0.42, 0.45)
                               for confirm in sorted({original_confirm, 80000, 120000})]
            for metric, normalized, close, open_, confirm in candidates:
                scored = score(reference, replay(args.binary.resolve(), settings, rows,
                                                normalized, close, open_, stride, offset, confirm,
                                                args.onset_screen_ratio if metric == 'screen_onset_veto'
                                                else None))
                if metric == 'raw_meters' and stride == 1:
                    baseline = [bool(x['contacts']) for x in rows]
                    if baseline != [x['contact'] for x in scored['trajectory']]:
                        raise ValueError('Baseline controller does not reproduce retained contacts')
                cases.append(dict(metric=metric, close=close/(0.1 if normalized else 1),
                                  open=open_/(0.1 if normalized else 1), stride=stride,
                                  offset=offset, confirm_us=confirm,
                                  onset_screen_ratio=(args.onset_screen_ratio
                                                      if metric == 'screen_onset_veto' else None), **scored))
        reports.append(dict(source=str(path), geometry_marker_max_error_m=error, cases=cases))
    if hashes != {str(p): digest(p) for p in inputs}:
        raise ValueError('Source/binary/settings changed during comparison')
    report = dict(schema='wigglecontrol.pinch-interpretation-probe/v1',
                  camera_opened=False, system_input=False, deployed=False,
                  hashes=hashes, reports=reports,
                  limits='Development controls only; ratio encoding is not physical meters; '
                         'no threshold selection, unseen-view or human acceptance')
    with args.output.open('x') as out:
        out.write(json.dumps(report, indent=2, allow_nan=False)+'\n')


if __name__ == '__main__':
    main()
