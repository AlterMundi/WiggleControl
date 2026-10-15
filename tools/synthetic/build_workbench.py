#!/usr/bin/env python3
"""Package retained synthetic measurements into an offline, sensor-free TV view."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input_root', type=Path)
    parser.add_argument('output', type=Path, help='New output directory')
    args = parser.parse_args()
    template = Path(__file__).with_name('workbench.html').read_text()
    poses = []
    sources = []
    for key, title, directory, predictions in (
        ('open', 'OPEN REFERENCE', 'reference-sweep', 'predictions-repeated.json'),
        ('intermediate', 'INTERMEDIATE CANDIDATE', 'middle-sweep', 'model-evaluation.json'),
        ('near', 'NEAR-FINGER CANDIDATE', 'near-sweep', 'predictions-repeated.json'),
    ):
        root = args.input_root / directory
        labels = json.loads((root / 'labels.json').read_text())
        measured = json.loads((root / predictions).read_text())
        rows = measured['rows'] if isinstance(measured, dict) else measured
        by_case = {row['case']: row for row in rows}
        if len(by_case) != len(rows):
            raise ValueError('Duplicate measurement case')
        frames = []
        for frame in labels['frames']:
            row = by_case.pop(frame['case'])
            if abs(row['reference_marker_gap_m'] - frame['rig_tip_gap_m']) > 1e-6:
                raise ValueError('Reference/measurement gap mismatch')
            trace = row['trace']
            if not trace or len(frame['image_px']) != 21:
                raise ValueError('Incomplete reference/trace')
            image = root / frame['image']
            if image.name != frame['image'] or not image.is_file():
                raise ValueError('Invalid image reference')
            frames.append({
                'case': frame['case'], 'image': f'{key}/{image.name}',
                'referenceMm': frame['rig_tip_gap_m'] * 1000,
                'referencePoints': frame['image_px'],
                'trace': [{'detected': bool(t['detected']),
                           'gapMm': t['world_pinch_distance_m'] * 1000 if t['detected'] else None,
                           'points': t.get('landmarks') if t['detected'] else None,
                           'tracked': bool(t.get('tracked', False))} for t in trace],
            })
        if by_case:
            raise ValueError('Unmatched model measurements')
        poses.append({'id': key, 'title': title, 'frames': frames})
        for source in (root / 'labels.json', root / predictions):
            sources.append({'file': f'{key}/{source.name}',
                            'sha256': hashlib.sha256(source.read_bytes()).hexdigest()})
    cases = [f['case'] for f in poses[0]['frames']]
    if any([f['case'] for f in pose['frames']] != cases for pose in poses):
        raise ValueError('Pose cases do not align')
    data = {'schema': 'wigglecontrol.synthetic-workbench/v1', 'observedDate': '2026-10-15',
            'imageSize': labels['image_size'], 'poses': poses, 'sources': sources,
            'cameraOpened': False, 'systemInput': False,
            'measurement': 'Retained model inference, repeated identical input; not a live run',
            'reference': labels['landmark_kind']}
    # Validate everything before creating the new package.
    args.output.mkdir(parents=True, exist_ok=False)
    for pose, directory, predictions in zip(poses,
            ('reference-sweep', 'middle-sweep', 'near-sweep'),
            ('predictions-repeated.json', 'model-evaluation.json', 'predictions-repeated.json')):
        dest = args.output / pose['id']
        dest.mkdir()
        root = args.input_root / directory
        for frame in pose['frames']:
            shutil.copyfile(root / Path(frame['image']).name, args.output / frame['image'])
        for filename in ('labels.json', predictions):
            shutil.copyfile(root / filename, dest / filename)
    shutil.copyfile(args.input_root / 'assets/LICENSE.txt', args.output / 'MHR-LICENSE.txt')
    payload = json.dumps(data, separators=(',', ':'), allow_nan=False)
    (args.output / 'data.json').write_text(payload + '\n')
    (args.output / 'run-state.js').write_text('window.receiveRunState?.({"seq":0,"status":"ready"});\n')
    # Inline the same data so file:// works offline, with no server or fetch permission.
    (args.output / 'index.html').write_text(template.replace('__WORKBENCH_DATA__', payload.replace('<', '\\u003c')))
    print(json.dumps({'output': str(args.output), 'poses': len(poses), 'cases': len(cases),
                      'detections': sum(f['trace'][-1]['detected'] for p in poses for f in p['frames']),
                      'camera_opened': False, 'server_required': False}))


if __name__ == '__main__':
    main()
