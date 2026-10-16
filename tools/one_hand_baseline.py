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


CASES = ('stationary', 'linear', 'reversal', 'move_stop', 'step', 'double_pinch', 'double_pinch_drift',
         'drag_gap_272ms', 'drag_gap_816ms', 'drag_gap_1496ms',
         'drag_gap_no_samples', 'drag_open_noise')
LATENCY_US = 65_000
STEP_US = 68_000
SCREEN = (1920, 1080)


def position(case, t):
    seconds = max(0, t) / 1_000_000
    if case == 'linear' or case.startswith('drag_'):
        return min(0.7, 0.3 + 0.16 * seconds)
    if case == 'reversal':
        return 0.3 + 0.25 * min(seconds, max(0, 2 - seconds))
    if case == 'move_stop':
        return 0.3 + 0.24 * min(seconds, 1)
    if case == 'step':
        return 0.3 if seconds < 1 else 0.7
    return 0.5


def closed(case, t):
    if case.startswith('drag_'):
        return 136_001 <= t < 2_176_001
    if case.startswith('double_pinch'):
        return 136_001 <= t < 340_001 or 476_001 <= t < 680_001
    return False


def fixture(case, tick_us=16_667):
    # A source clock starts before delivery. Tick cadence is independent of source.
    commands = [(0, {'type': 'start'})]
    for i in range(37):
        source = 1 + i * STEP_US
        missing = (case == 'drag_gap_272ms' and 8 <= i <= 10 or
                   case == 'drag_gap_816ms' and 8 <= i <= 18 or
                   case == 'drag_gap_1496ms' and 8 <= i <= 28)
        if case == 'drag_gap_no_samples' and 8 <= i <= 10:
            continue
        x = position(case, source)
        if case == 'stationary':
            x += 0.002 * (-1 if i % 2 else 1)
        if case == 'double_pinch_drift' and closed(case, source):
            x += 0.004 if source < 400_000 else -0.004
        observed_closed = closed(case, source) and not (case == 'drag_open_noise' and i == 10)
        hands = [] if missing else [{
            'id': 1, 'confidence': 0.99, 'engaged': True, 'pinch_eligible': True,
            'cursor': {'x': x, 'y': 0.5},
            'thumb_tip': {'x': 0.4, 'y': 0.5},
            'index_tip': {'x': 0.42 if observed_closed else 0.52, 'y': 0.5},
            'palm_width': 0.2, 'pinch_distance_ratio': 0.2 if observed_closed else 0.6,
        }]
        now = source + LATENCY_US
        commands.append((now, {'type': 'frame', 'now_us': now, 'observation': {
            'version': 1, 'source_us': source, 'ambiguous': False, 'hands': hands}}))
    end = 1 + 36 * STEP_US + LATENCY_US
    commands.extend((t, {'type': 'tick', 'now_us': t})
                    for t in range(tick_us, end + 1, tick_us))
    return sorted(commands, key=lambda entry: entry[0])


def score(case, commands, rows):
    if len(rows) != len(commands) + 1:
        raise ValueError('Replay did not emit exactly one receipt per command plus EOF')
    if any(r.get('camera_opened') is not False or r.get('system_input') is not False
           or r.get('absolute_pointer') is not True for r in rows):
        raise ValueError('Expected device-free absolute pipeline receipts')
    xy = [None, None]
    previous = None
    errors, jumps, move_times, buttons, output_positions = [], [], [], [], []
    for (now, _), row in zip(commands, rows):
        for event in row['mouse']:
            if event['kind'] == 3 and event['code'] in (0, 1):
                xy[event['code']] = event['value'] / 65535
            if event['kind'] == 1 and event['code'] == 272:
                buttons.append({'at_us': now, 'down': bool(event['value']),
                                'cursor_px': [v * size for v, size in zip(xy, SCREEN)]
                                if None not in xy else None,
                                'synthetic_intent_closed': closed(case, now - LATENCY_US)})
        if None in xy:
            continue
        point = tuple(xy)
        if previous is not None and point != previous:
            jumps.append(math.hypot((point[0] - previous[0]) * SCREEN[0],
                                    (point[1] - previous[1]) * SCREEN[1]))
            move_times.append(now)
        previous = point
        output_positions.append((now, point[0]))
        if now >= 250_000:
            target = (position(case, now) - 0.2) / 0.6
            errors.append((now, (point[0] - target) * SCREEN[0]))
    duration = sum(b[0] - a[0] for a, b in zip(errors, errors[1:]))
    rms = math.sqrt(sum(a[1] ** 2 * (b[0] - a[0])
                        for a, b in zip(errors, errors[1:])) / duration)
    presses = [b for b in buttons if b['down']]
    intervals = sorted((b-a)/1000 for a,b in zip(move_times,move_times[1:]))
    truth = [(position(case, t) - 0.2) / 0.6 for t, _ in commands]
    low, high = min(truth), max(truth)
    return {
        'case': case, 'source_interval_ms': STEP_US / 1000,
        'synthetic_delivery_delay_ms': LATENCY_US / 1000,
        'time_weighted_target_rms_px': rms,
        'max_cursor_step_px': max(jumps, default=0),
        'path_envelope_excursion_px': max(
            (max(low - p, p - high, 0) * SCREEN[0] for _, p in output_positions), default=0),
        'stop_overshoot_px': max((max(p - (position(case, 1_000_000)-0.2)/0.6, 0)*SCREEN[0]
                                 for t, p in output_positions if t >= 1_000_000), default=0)
        if case == 'move_stop' else None,
        'movement_updates': len(move_times),
        'median_movement_interval_ms': statistics.median(
            (b - a) / 1000 for a, b in zip(move_times, move_times[1:]))
        if len(move_times) > 1 else None,
        'p95_movement_interval_ms': intervals[min(len(intervals)-1,round(len(intervals)*0.95))]
        if intervals else None,
        'presses': sum(b['down'] for b in buttons),
        'press_intervals_ms': [(b['at_us'] - a['at_us']) / 1000
                               for a, b in zip(presses, presses[1:])],
        'successive_press_distances_px': [math.dist(a['cursor_px'], b['cursor_px'])
                                          for a, b in zip(presses, presses[1:])
                                          if a['cursor_px'] is not None and b['cursor_px'] is not None],
        'releases_while_synthetic_intent_closed': sum(
            not b['down'] and b['synthetic_intent_closed'] for b in buttons),
        'button_events': buttons,
        'eof_neutral': not rows[-1]['controller']['button_down'],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output-dir', required=True, type=Path)
    parser.add_argument('--drag-grace-ms', type=int, default=0)
    parser.add_argument('--filter', choices=('exponential', 'one-euro'), default='exponential')
    parser.add_argument('--tick-ms',type=float,default=16.667)
    parser.add_argument('--cursor-output', action='store_true')
    parser.add_argument('--cursor-prediction-ms', type=float, default=80)
    parser.add_argument('--cursor-smoothing-ms', type=float, default=30)
    parser.add_argument('--cursor-velocity-smoothing-ms', type=float, default=80)
    parser.add_argument('--cursor-max-speed', type=float, default=2)
    parser.add_argument('--cursor-max-lead', type=float, default=0.01)
    parser.add_argument('--cursor-no-direction-check', action='store_true')
    args = parser.parse_args()
    tick_us=round(args.tick_ms*1000)
    if not 1_000 <= tick_us <= 100_000:
        parser.error('--tick-ms must be1..100')
    binary = args.binary.resolve(strict=True)
    binary_digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    root = args.output_dir
    root.mkdir(parents=True, exist_ok=False)
    settings = root / 'settings.json'
    config = {
        'gestures': {'min_confidence': 0.5, 'close_ratio': 0.38, 'open_ratio': 0.4,
                     'stale_us': 300_000, 'drag_grace_us': args.drag_grace_ms * 1000},
        'workspace': {'min': {'x': 0.2, 'y': 0.2}, 'max': {'x': 0.8, 'y': 0.8}},
    }
    if args.filter == 'one-euro':
        config['gestures']['adaptive_smoothing'] = {
            'min_cutoff_hz': 1.0, 'beta': 10.0, 'derivative_cutoff_hz': 1.0, 'speed_floor': 0.0}
    if args.cursor_output:
        config['cursor_output'] = {
            'interval_us': 16_667, 'smoothing_us': args.cursor_smoothing_ms*1000,
            'prediction_us': round(args.cursor_prediction_ms*1000),
            'velocity_smoothing_us': args.cursor_velocity_smoothing_ms*1000,
            'max_speed_per_s': args.cursor_max_speed,
            'max_prediction_distance': args.cursor_max_lead,
            'consistent_prediction': not args.cursor_no_direction_check,
        }
    settings.write_text(json.dumps(config) + '\n')
    results = []
    for case in CASES:
        commands = fixture(case,tick_us)
        raw = ''.join(json.dumps(c) + '\n' for _, c in commands)
        (root / f'{case}-commands.jsonl').write_text(raw)
        run = subprocess.run([str(binary), 'pipeline-replay', str(settings.resolve()),
                              '2000', '300', '--absolute'], input=raw, text=True,
                             capture_output=True, timeout=15, check=True)
        (root / f'{case}-receipts.jsonl').write_text(run.stdout)
        rows = [json.loads(line) for line in run.stdout.splitlines()]
        results.append(score(case, commands, rows))
    if hashlib.sha256(binary.read_bytes()).hexdigest() != binary_digest:
        raise ValueError('Binary changed during the comparison; retain partial evidence and retry a new case')
    report = {
        'schema': 'wigglecontrol.one-hand-baseline/v1',
        'evidence': 'synthetic_intent_and_motion_device_free_actual_rust_pipeline',
        'binary_sha256': binary_digest,
        'tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'screen': SCREEN, 'cases': results,
        'settings': config,
        'tick_us': tick_us,
        'limits': 'No camera, compositor, application or human acceptance; selected synthetic cases only.',
    }
    (root / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
