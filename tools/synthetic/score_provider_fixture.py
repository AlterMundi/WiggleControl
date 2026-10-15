#!/usr/bin/env python3
"""Score a finite real-clock worker journal against independent source geometry."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import statistics

from compare_pinch_interpretations import score
from summarize_pair_closure import summarize


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate(events):
    require(bool(events), 'Empty journal')
    terminal = events[-1]
    require(terminal.get('event') == 'stop' and terminal.get('error') is None,
            'Producer did not finish successfully')
    require(terminal.get('worker_closed') is True, 'Worker close was not acknowledged')
    require(not terminal['controller']['contacts'] and not terminal['controller']['button_down'],
            'Final controller state is held')
    require(all(e.get('camera_opened') is False and e.get('system_input') is False
                for e in events), 'Journal is not a camera/device-free fixture')
    captures = [e for e in events if e['event'] == 'capture']
    by_sequence = {e['sequence']: e for e in captures}
    rows = [e for e in events if e['event'] == 'frame']
    require(bool(rows), 'No accepted observations')
    require(len(by_sequence) == len(captures), 'Duplicate capture sequence')
    require(all(a['sequence'] < b['sequence'] and a['source_us'] < b['source_us']
                for a, b in zip(captures, captures[1:])), 'Capture publication is not ordered')
    require(len({e['sequence'] for e in rows}) == len(rows), 'Duplicate accepted sequence')
    require(terminal['captured'] == len(captures), 'Capture count mismatch')
    require(terminal['issued'] == len(rows) + terminal['expired'], 'Lease count mismatch')
    require(terminal['captured'] == terminal['issued'] + terminal['skipped'],
            'Skipped source count mismatch')
    previous_sequence = previous_meaning = -1
    for row in rows:
        capture = by_sequence.get(row['sequence'])
        require(capture is not None, 'Accepted frame has no capture')
        require((row['capture_sequence'], row['source_us']) ==
                (capture['capture_sequence'], capture['source_us']), 'Capture identity mismatch')
        require(row['source_us'] <= row['issued_us'] <= row['received_us'] <=
                row['now_us'] <= row['meaning_us'], 'Invalid host clock order')
        require(row['sequence'] > previous_sequence and row['meaning_us'] > previous_meaning,
                'Accepted observations are not ordered')
        previous_sequence, previous_meaning = row['sequence'], row['meaning_us']
    return rows, terminal


def stale_releases(events):
    """Return independently ticked releases whose latest accepted input was held."""
    latest = None
    releases = []
    for event in events:
        if event['event'] == 'frame':
            latest = event
        elif event['event'] == 'intent_transition' and latest is not None:
            before, after = latest['controller'], event['controller']
            if (before['contacts'] or before['button_down']) and not (
                    after['contacts'] or after['button_down']) and after.get('reason') == 'stale_input':
                require(event['last_source_us'] == latest['source_us'], 'Release source mismatch')
                releases.append(dict(now_us=event['now_us'], source_us=latest['source_us'],
                                     source_age_ms=(event['now_us']-latest['source_us'])/1000))
    return releases


def wall_time(events, reference):
    """Compare all stable source intervals with actual semantic output, including skips."""
    captures = sorted((e for e in events if e['event'] == 'capture'), key=lambda e: e['source_us'])
    outputs = sorted((e.get('meaning_us', e.get('now_us')), len(e['controller']['contacts']))
                     for e in events if e['event'] in ('frame', 'intent_transition'))
    totals = dict(stable_hold_us=0, matching_count_us=0, near_hold_us=0,
                  matching_near_us=0, open_hold_contact_us=0)
    cursor, count = 0, 0
    for capture, following in zip(captures, captures[1:]):
        label = reference['frames'][capture['capture_sequence']]
        hands = label.get('reference_hands', [label])
        states = []
        for hand in hands:
            gap = hand['distal_skin']['surface_gap_m']
            closure = hand['closure_interpolation']
            states.append(0 if closure == 0 else 1 if gap <= .003 and closure > .999 else None)
        if any(state is None for state in states):
            continue
        expected = sum(states)
        start, end = capture['source_us'], following['source_us']
        while cursor < len(outputs) and outputs[cursor][0] <= start:
            _, count = outputs[cursor]
            cursor += 1
        while start < end:
            boundary = min(end, outputs[cursor][0]) if cursor < len(outputs) else end
            duration = boundary-start
            totals['stable_hold_us'] += duration
            totals['matching_count_us'] += duration if count == expected else 0
            if expected:
                totals['near_hold_us'] += duration
                totals['matching_near_us'] += duration if count == expected else 0
            elif count:
                totals['open_hold_contact_us'] += duration
            start = boundary
            if cursor < len(outputs) and outputs[cursor][0] <= start:
                _, count = outputs[cursor]
                cursor += 1
    return dict(**{k.removesuffix('_us')+'_ms': v/1000 for k, v in totals.items()},
                limits='All captured stable geometry intervals through last publication; includes '
                       'skips, perception delay and confirmation. Counts do not prove ownership. '
                       'Geometry proximity is not measured human intent or OS delivery.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('journal', type=Path)
    parser.add_argument('reference', type=Path, help='Independent geometry, indexed by capture_sequence')
    parser.add_argument('session', type=Path)
    parser.add_argument('output', type=Path, help='New directory for matched geometry and scores')
    args = parser.parse_args()
    events = [json.loads(line) for line in args.journal.read_text().splitlines()]
    rows, terminal = validate(events)
    reference = json.loads(args.reference.read_text())
    require(all(0 <= e['capture_sequence'] < len(reference['frames'])
                for e in events if e['event'] == 'capture'), 'Capture exceeds reference geometry')
    wall = wall_time(events, reference)
    labels = reference['frames']
    selected, predictions = [], []
    for index, row in enumerate(rows):
        require(0 <= row['capture_sequence'] < len(labels), 'Capture exceeds reference geometry')
        label = copy.deepcopy(labels[row['capture_sequence']])
        label.update(frame=index, source_us=row['source_us'], source_fixture_frame=row['capture_sequence'])
        selected.append(label)
        predictions.append(dict(row, frame=index, contacts=row['controller']['contacts']))
    reference['frames'] = selected
    matched = dict(camera_opened=False, system_input=False, reference=reference, predictions=predictions,
                   config_sha256=hashlib.sha256(args.session.read_bytes()).hexdigest(),
                   label_mapping='Independent geometry selected by capture_sequence; only frame/time '
                                 'labels reindexed to host capture. Skipped source frames scored separately.')
    args.output.mkdir(mode=0o700, exist_ok=False)
    matched_path = args.output/'matched.json'
    matched_path.write_text(json.dumps(matched, indent=2)+'\n')
    quality = (summarize(matched_path, args.session) if reference.get('paired_closure') else
               score(reference, [(r, r['controller']) for r in predictions]))
    ages = sorted((r['meaning_us']-r['source_us'])/1000 for r in rows)
    result = dict(schema='wigglecontrol.provider-fixture-score/v1', terminal=terminal,
                  frames=len(rows), quality=quality, wall_time=wall,
                  stale_releases=stale_releases(events),
                  journal_sha256=hashlib.sha256(args.journal.read_bytes()).hexdigest(),
                  reference_sha256=hashlib.sha256(args.reference.read_bytes()).hexdigest(),
                  session_sha256=matched['config_sha256'],
                  encoded_frame_to_meaning_ms=dict(median=statistics.median(ages),
                      p95=ages[int((len(ages)-1)*.95)], maximum=max(ages)),
                  accepted_hz=(len(rows)-1)*1e6/(rows[-1]['meaning_us']-rows[0]['meaning_us'])
                      if len(rows)>1 else None,
                  limits='Synthetic encoded publication to meaning on one host. Excludes camera '
                         'exposure/acquisition and OS/app delivery. Accepted-frame hold coverage '
                         'does not measure uninterrupted wall-time contact coverage.')
    (args.output/'score.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'quality'}))


if __name__ == '__main__':
    main()
