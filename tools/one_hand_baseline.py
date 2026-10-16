#!/usr/bin/env python3
"""Device-free synthetic interaction baseline through the actual absolute pipeline.

Known synthetic motion/intent is distinct from private video appearance or intent.
This measures event generation, not compositor delivery or physical usability.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess


CASES = ('stationary', 'linear', 'reversal', 'double_pinch',
         'drag_gap_272ms', 'drag_gap_816ms')
LATENCY_US = 65_000
STEP_US = 68_000
SCREEN = (1920, 1080)


def position(case, t):
    seconds = max(0, t) / 1_000_000
    if case == 'linear' or case.startswith('drag_gap'):
        return min(0.7, 0.3 + 0.16 * seconds)
    if case == 'reversal':
        return 0.3 + 0.25 * min(seconds, max(0, 2 - seconds))
    return 0.5


def closed(case, t):
    if case.startswith('drag_gap'):
        return 136_001 <= t < 2_176_001
    if case == 'double_pinch':
        return 136_001 <= t < 340_001 or 476_001 <= t < 680_001
    return False


def fixture(case):
    # A source clock starts before delivery. Tick cadence is independent of source.
    commands = [(0, {'type': 'start'})]
    for i in range(37):
        source = 1 + i * STEP_US
        missing = (case == 'drag_gap_272ms' and 8 <= i <= 10 or
                   case == 'drag_gap_816ms' and 8 <= i <= 18)
        x = position(case, source)
        if case == 'stationary':
            x += 0.002 * (-1 if i % 2 else 1)
        hands = [] if missing else [{
            'id': 1, 'confidence': 0.99, 'engaged': True, 'pinch_eligible': True,
            'cursor': {'x': x, 'y': 0.5},
            'thumb_tip': {'x': 0.4, 'y': 0.5},
            'index_tip': {'x': 0.42 if closed(case, source) else 0.52, 'y': 0.5},
            'palm_width': 0.2, 'pinch_distance_ratio': 0.2 if closed(case, source) else 0.6,
        }]
        now = source + LATENCY_US
        commands.append((now, {'type': 'frame', 'now_us': now, 'observation': {
            'version': 1, 'source_us': source, 'ambiguous': False, 'hands': hands}}))
    end = 1 + 36 * STEP_US + LATENCY_US
    commands.extend((t, {'type': 'tick', 'now_us': t})
                    for t in range(16_667, end + 1, 16_667))
    return sorted(commands, key=lambda entry: entry[0])


def score(case, commands, rows):
    if len(rows) != len(commands) + 1:
        raise ValueError('Replay did not emit exactly one receipt per command plus EOF')
    if any(r.get('camera_opened') is not False or r.get('system_input') is not False
           or r.get('absolute_pointer') is not True for r in rows):
        raise ValueError('Expected device-free absolute pipeline receipts')
    xy = [None, None]
    previous = None
    errors, jumps, move_times, buttons = [], [], [], []
    for (now, _), row in zip(commands, rows):
        for event in row['mouse']:
            if event['kind'] == 3 and event['code'] in (0, 1):
                xy[event['code']] = event['value'] / 65535
            if event['kind'] == 1 and event['code'] == 272:
                buttons.append({'at_us': now, 'down': bool(event['value']),
                                'synthetic_intent_closed': closed(case, now - LATENCY_US)})
        if None in xy:
            continue
        point = tuple(xy)
        if previous is not None and point != previous:
            jumps.append(math.hypot((point[0] - previous[0]) * SCREEN[0],
                                    (point[1] - previous[1]) * SCREEN[1]))
            move_times.append(now)
        previous = point
        if now >= 250_000:
            target = (position(case, now) - 0.2) / 0.6
            errors.append((now, (point[0] - target) * SCREEN[0]))
    duration = sum(b[0] - a[0] for a, b in zip(errors, errors[1:]))
    rms = math.sqrt(sum(a[1] ** 2 * (b[0] - a[0])
                        for a, b in zip(errors, errors[1:])) / duration)
    return {
        'case': case, 'source_interval_ms': STEP_US / 1000,
        'synthetic_delivery_delay_ms': LATENCY_US / 1000,
        'time_weighted_target_rms_px': rms,
        'max_cursor_step_px': max(jumps, default=0),
        'movement_updates': len(move_times),
        'median_movement_interval_ms': statistics.median(
            (b - a) / 1000 for a, b in zip(move_times, move_times[1:]))
        if len(move_times) > 1 else None,
        'presses': sum(b['down'] for b in buttons),
        'releases_while_synthetic_intent_closed': sum(
            not b['down'] and b['synthetic_intent_closed'] for b in buttons),
        'button_events': buttons,
        'eof_neutral': not rows[-1]['controller']['button_down'],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output-dir', required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    root = args.output_dir
    root.mkdir(parents=True, exist_ok=False)
    settings = root / 'settings.json'
    settings.write_text(json.dumps({
        'gestures': {'min_confidence': 0.5, 'close_ratio': 0.38, 'open_ratio': 0.4,
                     'stale_us': 300_000},
        'workspace': {'min': {'x': 0.2, 'y': 0.2}, 'max': {'x': 0.8, 'y': 0.8}},
    }) + '\n')
    results = []
    for case in CASES:
        commands = fixture(case)
        raw = ''.join(json.dumps(c) + '\n' for _, c in commands)
        (root / f'{case}-commands.jsonl').write_text(raw)
        run = subprocess.run([str(binary), 'pipeline-replay', str(settings.resolve()),
                              '2000', '300', '--absolute'], input=raw, text=True,
                             capture_output=True, timeout=15, check=True)
        (root / f'{case}-receipts.jsonl').write_text(run.stdout)
        rows = [json.loads(line) for line in run.stdout.splitlines()]
        results.append(score(case, commands, rows))
    report = {
        'schema': 'wigglecontrol.one-hand-baseline/v1',
        'evidence': 'synthetic_intent_and_motion_device_free_actual_rust_pipeline',
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'screen': SCREEN, 'cases': results,
        'limits': 'No camera, compositor, application or human acceptance; selected synthetic cases only.',
    }
    (root / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
