#!/usr/bin/env python3
"""Compare geometry-controlled reference images using the existing offline CLI."""
import argparse
import json
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('config', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/debug/wigglecontrol'))
    parser.add_argument('--repeat', type=int, default=10)
    args = parser.parse_args()
    if not 1 <= args.repeat <= 60:
        parser.error('--repeat must be1..60')
    output = args.directory/'model-evaluation.json'
    if output.exists():
        parser.error('model-evaluation.json already exists; preserve the earlier result')
    labels = json.loads((args.directory/'labels.json').read_text())
    rows = []
    for frame in labels['frames']:
        result = subprocess.run([str(args.binary.resolve()), 'image-check',
                                 str(args.directory/frame['image']), str(args.config),
                                 '--repeat', str(args.repeat)], check=True,
                                capture_output=True, text=True)
        trace = [json.loads(line) for line in result.stdout.splitlines()]
        if len(trace) != args.repeat:
            raise RuntimeError('The CLI did not return the requested number of observations')
        rows.append({'frame': frame['frame'], 'case': frame['case'],
                     'reference_marker_gap_m': frame['rig_tip_gap_m'], 'trace': trace})
        last = trace[-1]
        print(frame['case'], 'marker_mm', round(frame['rig_tip_gap_m']*1000, 3),
              'estimated_mm', round(last['world_pinch_distance_m']*1000, 3)
              if last['detected'] else 'not_detected', flush=True)
    with output.open('x') as file:
        json.dump({'schema': 'wigglecontrol.synthetic-evaluation/v1',
                   'repeat': args.repeat, 'rows': rows}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
