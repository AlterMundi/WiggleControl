#!/usr/bin/env python3
"""Package measured controller results and a camera-free TV experiment view."""
import argparse
import json
import hashlib
from pathlib import Path
import shutil


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('report', type=Path)
    p.add_argument('output', type=Path, help='New private directory')
    p.add_argument('--asset-license', type=Path, required=True)
    p.add_argument('--vision-report', nargs=2, action='append', default=[],
                   metavar=('LABEL','REPORT'), help='Retained comparison label and result JSON')
    args = p.parse_args()
    report = json.loads(args.report.read_text())
    if report.get('schema') != 'wigglecontrol.controller-simulation/v1' or not report.get('rows'):
        p.error('Expected completed controller-simulation/v1 report')
    report['filter_source_sha256'] = hashlib.sha256(args.report.read_bytes()).hexdigest()
    report['comparisons'] = []
    for label, file in args.vision_report:
        result = json.loads(Path(file).read_text())
        rows = result['predictions']
        times = sorted(r['processing_ms'] for r in rows)
        report['comparisons'].append(dict(label=label,
            two_hand_frames=sum(len(r['hands']) == 2 for r in rows), frames=len(rows),
            median_ms=(times[(len(times)-1)//2]+times[len(times)//2])/2,
            source_sha256=hashlib.sha256(Path(file).read_bytes()).hexdigest()))
    args.output.mkdir(mode=0o700, exist_ok=False)
    (args.output/'near').mkdir()
    shutil.copy(Path(__file__).with_name('controller.html'), args.output/'index.html')
    shutil.copy(args.asset_license, args.output/'MHR-LICENSE')
    text = json.dumps(report, separators=(',', ':'), allow_nan=False)
    (args.output/'controller-results.json').write_text(text+'\n')
    (args.output/'controller-data.js').write_text('window.receiveControllerResults('+text+');\n')
    state = dict(status='idle', phase='ready', runId='prepared', seq=0)
    (args.output/'run-state.js').write_text('window.receiveRunState('+json.dumps(state)+');\n')
    print('Packaged measured cases:', len(report['rows']))


if __name__ == '__main__':
    main()
