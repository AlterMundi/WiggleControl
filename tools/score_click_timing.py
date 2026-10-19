#!/usr/bin/env python3
"""Replay known synthetic double pinches across source sampling phases.

No camera/models/devices/application. The supplied delay is not a latency result.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess

from one_hand_baseline import observation_hand, SCREEN


WAVES = ((60, 80), (80, 80), (100, 100), (140, 100), (200, 100), (140, 40))


def fixture(close_ms, open_ms, phase_us, period_us, delay_us, open_ratio=0.6,
            right_click=False, pinch_drift=0.0):
    close, opening = close_ms * 1000, open_ms * 1000
    start = 400_000
    second = start + close + opening
    end = second + close + 400_000
    commands = [(0, {'type': 'start'})]
    observed = [0, 0]
    gap_open = 0
    for source in range(1 + phase_us, end + 1, period_us):
        first = start <= source < start + close
        last = second <= source < second + close
        observed[0] += first
        observed[1] += last
        gap_open += start + close <= source < second
        now = source + delay_us
        hand = observation_hand(0.5, first or last)
        if not first and not last:
            hand['pinch_distance_ratio'] = open_ratio
        if first or last:
            hand['cursor']['x'] += pinch_drift if first else -pinch_drift
        commands.append((now, {'type': 'frame', 'now_us': now, 'observation': {
            'version': 1, 'source_us': source, 'ambiguous': False,
            'hands': [hand]}}))
        if right_click:
            commands[-1][1]['middle_pinch'] = {
                'hand_id': hand['id'], 'source_us': source,
                'middle_tip': {'x': 0.52, 'y': 0.5}, 'pinch_distance_ratio': open_ratio}
    last = commands[-1][0]
    commands.extend((t, {'type': 'tick', 'now_us': t}) for t in range(5000, last + 1, 5000))
    commands.sort(key=lambda item: item[0])
    commands.append((last + 1, {'type': 'stop'}))
    return commands, observed, gap_open


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output-dir', required=True, type=Path)
    parser.add_argument('--period-ms', type=float, default=68)
    parser.add_argument('--delay-ms', type=float, default=65)
    parser.add_argument('--confirm-ms', type=float, help='Explicit override; otherwise preserve settings or use40ms')
    parser.add_argument('--settings', type=Path, help='Replay selected controller settings instead of gesture-only defaults')
    parser.add_argument('--pinch-drift', type=float, default=0, help='Opposite synthetic cursor offsets for the two closures, normalized source units')
    parser.add_argument('--release-confirm-ms', type=float)
    parser.add_argument('--release-open-factor', type=float, default=1.25)
    parser.add_argument('--open-ratio', type=float, default=0.6)
    parser.add_argument('--phases', type=int, default=17)
    args = parser.parse_args()
    period, delay = (round(t * 1000) for t in (args.period_ms, args.delay_ms))
    if not math.isfinite(args.pinch_drift) or not 0 <= args.pinch_drift <= 0.05:
        parser.error('Require normalized pinch drift0..0.05')
    input_settings = args.settings.read_bytes() if args.settings else None
    settings = json.loads(input_settings) if input_settings is not None else {
        'gestures': {'min_confidence': 0.5, 'close_ratio': 0.38,
                     'open_ratio': 0.4, 'confirm_us': 40_000, 'stale_us': 300_000}}
    if args.confirm_ms is not None:
        settings.setdefault('gestures', {})['confirm_us'] = round(args.confirm_ms * 1000)
    confirm = settings.get('gestures', {}).get('confirm_us', 40_000)
    if not 10_000 <= period <= 200_000 or not 0 <= delay < 100_000 or not 1 <= args.phases <= 100:
        parser.error('Require period10..200ms, delay0..<100ms and1..100 phases')
    if not 0 <= confirm < 300_000:
        parser.error('Require confirmation0..<300ms')
    if not 0.4 <= args.open_ratio <= 3:
        parser.error('Require observed open ratio0.4..3')
    binary = args.binary.resolve(strict=True)
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    root = args.output_dir
    root.mkdir(parents=True, exist_ok=False)
    config = root / 'settings.json'
    if args.release_confirm_ms is not None:
        settings.setdefault('gestures', {})['release_confirmation'] = {
            'confirm_us': round(args.release_confirm_ms*1000),
            'immediate_open_factor': args.release_open_factor}
    config.write_text(json.dumps(settings) + '\n')
    cases = []
    for close, opening in WAVES:
        for phase in range(args.phases):
            offset = round(phase * period / args.phases)
            commands, observed, gap_open = fixture(close, opening, offset, period, delay, args.open_ratio,
                                                       settings.get('right_click', False), args.pinch_drift)
            raw = ''.join(json.dumps(c) + '\n' for _, c in commands)
            name = f'close{close}-open{opening}-phase{offset}'
            (root / (name + '-commands.jsonl')).write_text(raw)
            run = subprocess.run([str(binary), 'pipeline-replay', str(config.resolve()),
                                  '2000', '300', '--absolute'], input=raw, text=True,
                                 capture_output=True, timeout=10, check=True)
            (root / (name + '-receipts.jsonl')).write_text(run.stdout)
            rows = [json.loads(line) for line in run.stdout.splitlines()]
            if len(rows) != len(commands) + 1 or any(r['camera_opened'] or r['system_input'] for r in rows):
                raise ValueError('Expected one device-free receipt per command and EOF')
            downs, ups, down_positions, held_travel = [], [], [], []
            position, held_anchor = [None, None], None
            for (now, _), row in zip(commands, rows):
                for e in row['mouse']:
                    if e['kind'] == 3 and e['code'] in (0, 1):
                        position[e['code']] = e['value'] / 65535 * SCREEN[e['code']]
                    if held_anchor is not None and None not in position:
                        held_travel.append(math.dist(position, held_anchor))
                    if e['kind'] == 1 and e['code'] == 272:
                        (downs if e['value'] else ups).append(now)
                        if e['value']:
                            held_anchor = tuple(position) if None not in position else None
                            down_positions.append(held_anchor)
                        else:
                            held_anchor = None
            first=[d for d in downs if 400_000<=d-delay<400_000+close*1000]
            cases.append({'close_ms': close, 'open_ms': opening, 'phase_us': offset,
                          'observed_closed_samples': observed, 'observed_open_between': gap_open,
                          'downs_us': downs, 'ups_us': ups,
                          'press_distance_px': math.dist(*down_positions) if len(down_positions) == 2 and all(down_positions) else None,
                          'max_held_travel_px': max(held_travel, default=0),
                          'two_complete_pairs': len(downs) == len(ups) == 2,
                          'press_interval_ms': (downs[1] - downs[0]) / 1000 if len(downs) == 2 else None,
                          'first_down_after_intent_ms': (first[0] - 400_000) / 1000 if first else None,
                          'eof_neutral': not rows[-1]['controller']['button_down'],
                          'input_sha256': hashlib.sha256(raw.encode()).hexdigest()})
    summaries = []
    for close, opening in WAVES:
        selected = [c for c in cases if c['close_ms'] == close and c['open_ms'] == opening]
        times = [c['first_down_after_intent_ms'] for c in selected if c['first_down_after_intent_ms'] is not None]
        summaries.append({'close_ms': close, 'open_ms': opening, 'phases': len(selected),
                          'two_complete_pairs': sum(c['two_complete_pairs'] for c in selected),
                          'first_gesture_detected':len(times),
                          'median_first_down_after_intent_ms': statistics.median(times) if times else None,
                          'first_down_range_ms': [min(times), max(times)] if times else None,
                          'max_press_distance_px': max((c['press_distance_px'] for c in selected if c['press_distance_px'] is not None), default=None),
                          'max_held_travel_px': max(c['max_held_travel_px'] for c in selected)})
    if hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
        raise ValueError('Binary changed during the comparison')
    report = {'schema': 'wigglecontrol.synthetic-click-timing/v1', 'binary_sha256': digest,
              'tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'hand_fixture_tool_sha256':hashlib.sha256(Path(__file__).with_name('one_hand_baseline.py').read_bytes()).hexdigest(),
              'period_us': period, 'delay_us': delay, 'confirm_us': confirm,
              'settings': settings, 'observed_open_ratio': args.open_ratio,
              'input_settings_sha256': hashlib.sha256(input_settings).hexdigest() if input_settings is not None else None,
              'synthetic_pinch_drift': args.pinch_drift, 'assumed_display_pixels': SCREEN,
              'limits': 'Known synthetic gestures and supplied clocks/delay; no model, camera, physical accuracy or application double-click acceptance.',
              'summaries': summaries, 'cases': cases}
    (root / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(summaries, indent=2))


if __name__ == '__main__':
    main()
