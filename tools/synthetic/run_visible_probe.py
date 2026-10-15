#!/usr/bin/env python3
"""Run a bounded cold-to-tracked comparison and project actual progress over SSH."""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import time
import uuid


SINK = '''import sys,json,pathlib,os,base64
root=pathlib.Path(sys.argv[1])
assert (root/'index.html').is_file()
for line in sys.stdin:
 state=json.loads(line)
 result=state.pop('result',None)
 if result is not None:
  report_temp=root/'run-results.json.pending'
  report_temp.write_text(json.dumps(result,separators=(',',':'))+'\\n')
  os.replace(report_temp,root/'run-results.json')
 image=state.pop('imagePayload',None)
 if image:
  name=state['image']
  assert pathlib.PurePosixPath(name).parts[0]=='near' and len(pathlib.PurePosixPath(name).parts)==2
  assert '..' not in pathlib.PurePosixPath(name).parts
  target=root/name
  image_temp=target.with_suffix('.pending')
  image_temp.write_bytes(base64.b64decode(image,validate=True))
  os.replace(image_temp,target)
 temp=root/'run-state.js.pending'
 temp.write_text('window.receiveRunState?.('+json.dumps(state,separators=(',',':'))+');\\n')
 os.replace(temp,root/'run-state.js')
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input_root', type=Path)
    parser.add_argument('config', type=Path)
    parser.add_argument('output', type=Path, help='New private result directory')
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--ssh-host', required=True)
    parser.add_argument('--ssh-identity', type=Path, required=True)
    parser.add_argument('--display-dir', required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    command = shlex.join(['python3', '-u', '-c', SINK, args.display_dir])
    ssh = ['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=8', '-o', 'IdentitiesOnly=yes',
           '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=2', '-i', str(args.ssh_identity),
           args.ssh_host, command]
    rows = []
    seq = 0
    completed = 0
    run_id = uuid.uuid4().hex
    with (args.output / 'display-transport.log').open('w') as transport_log:
        sink = subprocess.Popen(ssh, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                stderr=transport_log, text=True)
        def publish(**state):
            nonlocal seq
            seq += 1
            state.update(seq=seq, runId=run_id, completed=completed, total=6, repeat=60)
            payload = json.dumps(state, separators=(',', ':'), allow_nan=False)
            (args.output / 'progress.json').write_text(payload + '\n')
            if sink.poll() is not None:
                raise RuntimeError('Display transport exited; retained local progress')
            sink.stdin.write(payload + '\n')
            sink.stdin.flush()
        try:
            for case in ('reference', 'yaw--1'):
                for pose, directory in (('open', 'reference-sweep'),
                                        ('intermediate', 'middle-sweep'), ('near', 'near-sweep')):
                    source = args.input_root / directory
                    labels = json.loads((source / 'labels.json').read_text())
                    frame = next(f for f in labels['frames'] if f['case'] == case)
                    image = source / frame['image']
                    trace = []
                    simplified = []
                    publish(status='running', pose=pose, case=case, iteration=0)
                    stem = pose + '-' + case
                    with (args.output / (stem + '.jsonl')).open('x') as raw, \
                            (args.output / (stem + '.stderr')).open('x') as errors:
                        process = subprocess.Popen([str(args.binary.resolve()), 'image-check',
                            str(image), str(args.config), '--repeat', '60'], stdout=subprocess.PIPE,
                            stderr=errors, text=True)
                        started = time.monotonic()
                        try:
                            for line in process.stdout:
                                row = json.loads(line)
                                raw.write(line)
                                raw.flush()
                                trace.append(row)
                                simplified.append({'detected': row['detected'],
                                    'gapMm': row['world_pinch_distance_m'] * 1000 if row['detected'] else None,
                                    'points': row.get('landmarks') if row['detected'] else None,
                                    'tracked': bool(row.get('tracked', False))})
                                # Keep only the current skeleton in display updates; raw evidence retains all.
                                projection = [{k: v for k, v in point.items() if k != 'points'}
                                              for point in simplified[:-1]] + [simplified[-1]]
                                # Publish real inference, not an invented timer or replay.
                                publish(status='running', pose=pose, case=case,
                                        iteration=len(trace), trace=projection)
                            if process.wait(timeout=10) or len(trace) != 60:
                                raise RuntimeError('Incomplete inference; inspect retained stderr')
                        finally:
                            if process.poll() is None:
                                process.terminate()
                                process.wait(timeout=10)
                    completed += 1
                    rows.append({'pose': pose, 'case': case, 'source_image': image.name,
                                 'image_sha256': hashlib.sha256(image.read_bytes()).hexdigest(),
                                 'reference_mm': frame['rig_tip_gap_m'] * 1000,
                                 'elapsed_seconds': time.monotonic() - started, 'trace': trace})
                    print(pose, case, 'frame10', simplified[9]['gapMm'],
                          'frame60', simplified[-1]['gapMm'], flush=True)
            result = {'schema': 'wigglecontrol.visible-tracking-probe/v1', 'repeat': 60,
                      'camera_opened': False, 'system_input': False,
                      'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                      'rows': rows}
            (args.output / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
            publish(status='complete', pose='near', case='yaw--1', iteration=60,
                    trace=simplified, result=result)
        except Exception as error:
            try:
                publish(status='error', message=str(error))
            except Exception:
                pass
            raise
        finally:
            sink.stdin.close()
            try:
                sink.wait(timeout=10)
            except subprocess.TimeoutExpired:
                sink.terminate()
                sink.wait(timeout=5)
    print('Retained results:', args.output, flush=True)


if __name__ == '__main__':
    main()
