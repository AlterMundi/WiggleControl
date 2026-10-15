#!/usr/bin/env python3
"""Render new yaw controls and display real render/inference progress on the TV."""
import argparse
import base64
import json
import io
import os
from pathlib import Path
import shlex
import subprocess
import uuid

from run_visible_probe import SINK


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('fbx', type=Path)
    p.add_argument('config', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--ssh-host', required=True)
    p.add_argument('--ssh-identity', type=Path, required=True)
    p.add_argument('--display-dir', required=True)
    p.add_argument('--resume', action='store_true', help='Use retained renders and completed model cases')
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=args.resume)
    if (args.output / 'results.json').exists():
        p.error('Completed results already exist; do not overwrite them')
    source = Path(__file__).resolve().parent
    render_dir = args.output / 'render'
    renderer = ['uv', 'run', '--offline', '--no-project', '--python', '3.11',
                '--with-requirements', str(source / 'requirements.lock'), 'python',
                str(source / 'render_hand_reference.py'), str(args.fbx), str(render_dir),
                '--closure', '1', '--yaw-steps', '9', '--hand-only']
    ssh = ['ssh', '-o', 'BatchMode=yes', '-o', 'IdentitiesOnly=yes', '-o', 'ConnectTimeout=8',
           '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=2', '-i', str(args.ssh_identity),
           args.ssh_host, shlex.join(['python3', '-u', '-c', SINK, args.display_dir])]
    seq = 0
    run_id = uuid.uuid4().hex
    rows = []
    with (args.output / 'transport.log').open('w') as transport_log, \
            (args.output / 'render.log').open('a' if args.resume else 'x') as render_log:
        sink = subprocess.Popen(ssh, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                stderr=transport_log, text=True)
        def publish(**state):
            nonlocal seq
            seq += 1
            state.update(seq=seq, runId=run_id, kind='yaw-sweep', total=9, repeat=10,
                         pose='near', case='yaw--1', completed=len(rows))
            state['series'] = [{'angle': r['reference']['yaw_degrees'],
                                'gapMm': r['trace'][-1]['world_pinch_distance_m'] * 1000
                                if r['trace'][-1]['detected'] else None}
                               for r in rows]
            (args.output / 'progress.json').write_text(json.dumps(
                {k: v for k, v in state.items() if k != 'imagePayload'}) + '\n')
            if sink.poll() is not None:
                raise RuntimeError('Display transport exited')
            sink.stdin.write(json.dumps(state, separators=(',', ':'), allow_nan=False) + '\n')
            sink.stdin.flush()
        env = dict(os.environ, PYTHONUNBUFFERED='1', OMP_NUM_THREADS='2')
        render = None
        if args.resume:
            labels = json.loads((render_dir / 'labels.json').read_text())
            stream = io.StringIO(''.join('WIGGLECONTROL_FRAME ' + json.dumps(f) + '\n' for f in labels['frames']))
        else:
            render = subprocess.Popen(renderer, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                      text=True, env=env)
            stream = render.stdout
        try:
            publish(status='running', phase='prepare')
            for line in stream:
                render_log.write(line)
                render_log.flush()
                if line.startswith('WIGGLECONTROL_STAGE '):
                    stage = json.loads(line.split(' ', 1)[1])
                    publish(status='running', phase='render', angle=stage['yaw_degrees'],
                            referenceMm=stage['reference_mm'], depthCm=stage['depth_cm'])
                    print('Rendering yaw', stage['yaw_degrees'], flush=True)
                elif line.startswith('WIGGLECONTROL_FRAME '):
                    frame = json.loads(line.split(' ', 1)[1])
                    image = render_dir / frame['image']
                    fields = dict(image='near/live-' + frame['image'],
                                  angle=frame['yaw_degrees'], referenceMm=frame['rig_tip_gap_m'] * 1000,
                                  referencePoints=frame['image_px'],
                                  depthCm=[min(-v[2] for v in frame['camera_m']) * 100,
                                           max(-v[2] for v in frame['camera_m']) * 100])
                    raw_path = args.output / (frame['case'] + '.jsonl')
                    retained = raw_path.exists() and len(raw_path.read_text().splitlines()) == 10
                    publish(status='running', phase='review' if retained else 'inference', iteration=0,
                            imagePayload=base64.b64encode(image.read_bytes()).decode(), **fields)
                    if not retained:
                        if raw_path.exists():
                            raise RuntimeError('Partial model output needs explicit reconciliation')
                        with raw_path.open('x') as raw:
                            subprocess.run([str(args.binary.resolve()), 'image-check', str(image),
                                            str(args.config), '--repeat', '10'],
                                           stdout=raw, stderr=subprocess.PIPE, text=True, check=True, timeout=60)
                    trace = [json.loads(s) for s in raw_path.read_text().splitlines()]
                    if len(trace) != 10:
                        raise RuntimeError('Incomplete inference')
                    projection = [{'detected': v['detected'],
                                   'gapMm': v['world_pinch_distance_m'] * 1000 if v['detected'] else None,
                                   'points': v.get('landmarks'), 'tracked': bool(v.get('tracked', False))} for v in trace]
                    rows.append({'reference': frame, 'trace': trace})
                    publish(status='running', phase='review' if retained else 'inference', iteration=10, trace=projection, **fields)
                    print('Retained yaw' if retained else 'Measured yaw', frame['yaw_degrees'], 'model mm', projection[-1]['gapMm'], flush=True)
            if (render is not None and render.wait(timeout=10)) or len(rows) != 9:
                raise RuntimeError('Incomplete rendered sweep; inspect retained log')
            (args.output / 'results.json').write_text(json.dumps({'schema': 'wigglecontrol.visible-yaw/v1',
                'camera_opened': False, 'system_input': False, 'rows': rows}, indent=2) + '\n')
            publish(status='complete', phase='inference', iteration=10, trace=projection, **fields)
        except Exception as error:
            try:
                publish(status='error', message=str(error))
            except Exception:
                pass
            raise
        finally:
            if render is not None and render.poll() is None:
                render.terminate()
                render.wait(timeout=10)
            sink.stdin.close()
            try:
                sink.wait(timeout=10)
            except subprocess.TimeoutExpired:
                sink.terminate()
                sink.wait(timeout=5)


if __name__ == '__main__':
    main()
