#!/usr/bin/env python3
"""Replay known synthetic double pinches across source sampling phases.

No camera/models/devices/application. The supplied delay is not a latency result.
"""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from one_hand_baseline import observation_hand


WAVES = ((60, 80), (80, 80), (100, 100), (140, 100), (200, 100), (140, 40))


def fixture(close_ms, open_ms, phase_us, period_us, delay_us, open_ratio=0.6):
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
        commands.append((now, {'type': 'frame', 'now_us': now, 'observation': {
            'version': 1, 'source_us': source, 'ambiguous': False,
            'hands': [hand]}}))
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
    parser.add_argument('--confirm-ms', type=float, default=40)
    parser.add_argument('--release-confirm-ms', type=float)
    parser.add_argument('--release-open-factor', type=float, default=1.25)
    parser.add_argument('--open-ratio', type=float, default=0.6)
    parser.add_argument('--phases', type=int, default=17)
    args = parser.parse_args()
    period, delay, confirm = (round(t * 1000) for t in
                              (args.period_ms, args.delay_ms, args.confirm_ms))
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
    settings = {'gestures': {'min_confidence': 0.5, 'close_ratio': 0.38,
                             'open_ratio': 0.4, 'confirm_us': confirm, 'stale_us': 300_000}}
    if args.release_confirm_ms is not None:
        settings['gestures']['release_confirmation'] = {
            'confirm_us': round(args.release_confirm_ms*1000),
            'immediate_open_factor': args.release_open_factor}
    config.write_text(json.dumps(settings) + '\n')
    cases = []
    for close, opening in WAVES:
        for phase in range(args.phases):
            offset = round(phase * period / args.phases)
            commands, observed, gap_open = fixture(close, opening, offset, period, delay, args.open_ratio)
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
            downs, ups = [], []
            for (now, _), row in zip(commands, rows):
                for e in row['mouse']:
                    if e['kind'] == 1 and e['code'] == 272:
                        (downs if e['value'] else ups).append(now)
            first=[d for d in downs if 400_000<=d-delay<400_000+close*1000]
            cases.append({'close_ms': close, 'open_ms': opening, 'phase_us': offset,
                          'observed_closed_samples': observed, 'observed_open_between': gap_open,
                          'downs_us': downs, 'ups_us': ups,
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
                          'first_down_range_ms': [min(times), max(times)] if times else None})
    if hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
        raise ValueError('Binary changed during the comparison')
    report = {'schema': 'wigglecontrol.synthetic-click-timing/v1', 'binary_sha256': digest,
              'tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'hand_fixture_tool_sha256':hashlib.sha256(Path(__file__).with_name('one_hand_baseline.py').read_bytes()).hexdigest(),
              'period_us': period, 'delay_us': delay, 'confirm_us': confirm,
              'settings': settings, 'observed_open_ratio': args.open_ratio,
              'limits': 'Known synthetic gestures and supplied clocks/delay; no model, camera, physical accuracy or application double-click acceptance.',
              'summaries': summaries, 'cases': cases}
    (root / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(summaries, indent=2))


if __name__ == '__main__':
    main()
