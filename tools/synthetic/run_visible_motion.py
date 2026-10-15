#!/usr/bin/env python3
"""Observe an owned finite render and evaluate its sequence once; no sensors/input."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import shlex
import statistics
import subprocess
import time
import uuid

from run_visible_probe import SINK


def frame_projection(reference, row):
    paired = len(reference.get('reference_hands', [])) == 2
    result = dict(skin_gap_m=None if paired else reference.get('distal_skin', {}).get('surface_gap_m'),
                  closure=reference.get('closure_interpolation'),
                  model_gap_m=row['hands'][0].get('pinch_distance_m') if row['hands'] and not paired else None,
                  contacts=len(row['contacts']) if row.get('contacts') is not None else None,
                  hands=len(row['hands']), ambiguous=row['ambiguous'],
                  ambiguity_reason=row.get('ambiguity_reason'))
    if paired:
        result['references'] = [dict(id=h['id'], closure=h.get('closure_interpolation'),
                                     skin_gap_m=h.get('distal_skin', {}).get('surface_gap_m'))
                                 for h in reference['reference_hands']]
        result['model_hands'] = [dict(id=h.get('id'),
                                      gap_m=h.get('pinch_distance_m', h.get('world_pinch_distance_m')))
                                 for h in row['hands']]
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('render_dir', type=Path)
    p.add_argument('config', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--frames', type=int, required=True)
    p.add_argument('--ssh-host', required=True)
    p.add_argument('--ssh-identity', type=Path, required=True)
    p.add_argument('--display-dir', required=True)
    p.add_argument('--label', default='', help='Display label for this actual finite experiment')
    p.add_argument('--mediapipe-model', type=Path,
                   help='Optional pinned Tasks CPU comparison; does not evaluate Rust gesture meaning')
    p.add_argument('--model-sha256')
    p.add_argument('--rediscovery', type=int, default=5,
                   help='Controlled native missing-hand detector interval in frames, default5')
    p.add_argument('--pair-separation', type=float,
                   help='Override the session overlap guard for native replay only; otherwise use its config')
    args = p.parse_args()
    if not 9 <= args.frames <= 121:
        p.error('--frames must be9..121')
    if args.mediapipe_model and not args.model_sha256:
        p.error('--mediapipe-model requires --model-sha256')
    if not 1 <= args.rediscovery <= 30 or (args.mediapipe_model and args.rediscovery != 5):
        p.error('--rediscovery must be1..30 and is a native-only option')
    if args.pair_separation is not None and (not 0.8 <= args.pair_separation <= 2 or args.mediapipe_model):
        p.error('--pair-separation must be0.8..2 and is a native-only option')
    args.output.mkdir(mode=0o700, exist_ok=False)
    # Same atomic receiving writer as the retained static/yaw probes.
    command = shlex.join(['python3', '-u', '-c', SINK, args.display_dir])
    ssh = ['ssh', '-o', 'BatchMode=yes', '-o', 'IdentitiesOnly=yes',
           '-o', 'ConnectTimeout=8', '-o', 'ServerAliveInterval=5',
           '-o', 'ServerAliveCountMax=2', '-i', str(args.ssh_identity), args.ssh_host, command]
    run_id, seq, last_image = uuid.uuid4().hex, 0, None
    rows = []
    with (args.output/'transport.log').open('x') as log:
        sink = subprocess.Popen(ssh, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=log, text=True)
        def publish(**state):
            nonlocal seq
            seq += 1
            state.update(runId=run_id, seq=seq, total=args.frames, label=args.label)
            (args.output/'progress.json').write_text(json.dumps(
                {k: v for k, v in state.items() if k not in ('imagePayload', 'result')})+'\n')
            if sink.poll() is not None:
                raise RuntimeError('Display transport exited; local evidence retained')
            sink.stdin.write(json.dumps(state, separators=(',', ':'), allow_nan=False)+'\n')
            sink.stdin.flush()
        def picture(path):
            return dict(image='near/'+path.name, imagePayload=base64.b64encode(path.read_bytes()).decode())
        process = None
        try:
            deadline = time.monotonic() + 600
            publish(status='running', phase='inference' if (args.render_dir/'sequence.json').is_file()
                    else 'render', frame=0)
            while not (args.render_dir/'sequence.json').is_file():
                if time.monotonic() > deadline:
                    raise RuntimeError('Render did not complete within ten-minute observer bound')
                # Labels are published only after the entire finite render. PNG files
                # are announced only after the next file exists, avoiding partial reads.
                images = sorted(args.render_dir.glob('*-motion-*.png'))
                if len(images) > 1 and images[-2] != last_image:
                    last_image = images[-2]
                    publish(status='running', phase='render', frame=len(images)-2, **picture(last_image))
                time.sleep(0.5)
            manifest = json.loads((args.render_dir/'sequence.json').read_text())
            labels = json.loads((args.render_dir/'labels.json').read_text())
            if len(manifest['frames']) != args.frames:
                raise RuntimeError('Unexpected render frame count')
            publish(status='running', phase='inference', frame=0)
            with (args.output/'raw.jsonl').open('x') as raw, (args.output/'model.stderr').open('x') as errors:
                inference = [str(args.binary.resolve()), 'sequence-check',
                             str(args.render_dir/'sequence.json'), str(args.config)]
                if args.rediscovery != 5:
                    inference += ['--rediscovery', str(args.rediscovery)]
                if args.pair_separation is not None:
                    inference += ['--pair-separation', str(args.pair_separation)]
                if args.mediapipe_model:
                    inference = ['uv', 'run', '--offline', '--no-project', '--python', '3.11',
                        '--with-requirements', str(Path(__file__).with_name('mediapipe-requirements.lock')), 'python',
                        str(Path(__file__).with_name('mediapipe_sequence.py')),
                        str(args.render_dir/'sequence.json'), str(args.mediapipe_model),
                        '--expected-sha256', args.model_sha256]
                binary_digest = None if args.mediapipe_model else hashlib.sha256(args.binary.read_bytes()).hexdigest()
                config_digest = hashlib.sha256(args.config.read_bytes()).hexdigest()
                process = subprocess.Popen(inference,
                    stdout=subprocess.PIPE, stderr=errors, text=True)
                for line in process.stdout:
                    raw.write(line)
                    raw.flush()
                    row = json.loads(line)
                    if 'frame' not in row:
                        continue
                    rows.append(row)
                    last_image = args.render_dir/row['image']
                    reference = labels['frames'][row['frame']]
                    publish(status='running', phase='inference', frame=row['frame'],
                            **frame_projection(reference, row), **picture(last_image))
                if process.wait(timeout=10) or len(rows) != args.frames:
                    raise RuntimeError('Incomplete sequence inference; inspect model.stderr')
                if binary_digest is not None and hashlib.sha256(args.binary.read_bytes()).hexdigest() != binary_digest:
                    raise RuntimeError('Native binary changed during inference; retain a frozen artifact for comparison')
                if hashlib.sha256(args.config.read_bytes()).hexdigest() != config_digest:
                    raise RuntimeError('Configuration changed during inference; preserve a separate experiment config')
            result = dict(schema='wigglecontrol.visible-motion/v1', camera_opened=False, system_input=False,
                          experiment_label=args.label,
                          config_sha256=config_digest,
                          backend='mediapipe-1.1.1-cpu' if args.mediapipe_model else 'rust-onnx-cpu',
                          binary_sha256=binary_digest,
                          model_sha256=args.model_sha256,
                          rediscovery_interval=None if args.mediapipe_model else args.rediscovery,
                          pair_separation_widths=None if args.mediapipe_model else rows[0].get('pair_separation_widths'),
                          reference=labels, predictions=rows,
                          limits='Selected distal skin clearance is independent geometry, not qualified human intent, joint limits or whole-hand collision'
                          if labels.get('skin_clearance_measured') else
                          'Rigid open-hand movement only; no qualified contact intent or anatomical fingertips')
            (args.output/'results.json').write_text(json.dumps(result, indent=2)+'\n')
            # Publish a small replay projection; raw predictions/reference stay in the report.
            replay = dict(frames=[dict(frame=r['frame'], image='near/'+r['image'],
                                      label=args.label,
                                      **frame_projection(labels['frames'][r['frame']], r)) for r in rows])
            js = 'window.receiveSequenceResults('+json.dumps(replay, separators=(',', ':'))+');\n'
            remote = shlex.join(['python3', '-c',
                'import pathlib,sys,os;p=pathlib.Path(sys.argv[1]);t=p.with_suffix(".pending");t.write_bytes(sys.stdin.buffer.read());os.replace(t,p)',
                str(Path(args.display_dir)/'sequence-data.js')])
            subprocess.run(ssh[:-1]+[remote], input=js, text=True, check=True, timeout=20)
            ids = {h['id'] for row in rows for h in row['hands'] if 'id' in h}
            publish(status='complete', phase='retained', frame=args.frames-1,
                    detectedFrames=sum(bool(r['hands']) for r in rows),
                    referenceHands=labels.get('reference_hand_count'),
                    twoHandFrames=sum(len(r['hands']) == 2 for r in rows),
                    backend=result['backend'],
                    ambiguousFrames=None if args.mediapipe_model else sum(r['ambiguous'] for r in rows),
                    identityCount=None if args.mediapipe_model else len(ids),
                    processingMedianMs=statistics.median(r['processing_ms'] for r in rows),
                    result=result, **frame_projection(labels['frames'][-1], rows[-1]),
                    **picture(last_image))
            print('Completed sequence:', len(rows), 'frames,',
                  'stable IDs unavailable' if args.mediapipe_model else str(len(ids))+' observed IDs', flush=True)
        except Exception as error:
            try:
                publish(status='error', phase='failed', message=str(error))
            except Exception:
                pass
            raise
        finally:
            if process is not None and process.poll() is None:
                process.terminate()
                process.wait(timeout=10)
            sink.stdin.close()
            try:
                sink.wait(timeout=10)
            except subprocess.TimeoutExpired:
                sink.terminate()
                sink.wait(timeout=5)


if __name__ == '__main__':
    main()
