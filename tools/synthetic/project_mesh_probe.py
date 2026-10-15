#!/usr/bin/env python3
"""Project actual finite mesh-probe progress; never start or restart inference.

The owned workbench must already exist. Crops/right side are provided: contacts,
confidence and identity remain unknown. A timeout ends only this observer.
"""
import argparse
import base64
import json
from pathlib import Path
import shlex
import statistics
import subprocess
import time
import uuid

from run_visible_probe import SINK
from wilor_probe import load_selection


def read_complete_rows(path, offset):
    """An in-progress producer's last partial JSON line is not a snapshot."""
    rows = []
    if not path.exists():
        return offset, rows
    with path.open() as stream:
        stream.seek(offset)
        while True:
            line = stream.readline()
            if not line or not line.endswith('\n'):
                break
            rows.append(json.loads(line))
            offset = stream.tell()
    return offset, rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('selection', type=Path)
    parser.add_argument('results', type=Path)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--backend', required=True)
    parser.add_argument('--ssh-host', required=True)
    parser.add_argument('--ssh-identity', type=Path, required=True)
    parser.add_argument('--display-dir', required=True)
    parser.add_argument('--timeout', type=int, default=600)
    args = parser.parse_args()
    if not 1 <= args.timeout <= 3600:
        raise ValueError('Expected finite observer timeout1..3600s')
    selection = load_selection(args.selection)
    args.evidence.mkdir(mode=0o700, exist_ok=False)
    root = args.selection.resolve().parent
    run_id = uuid.uuid4().hex
    ssh = ['ssh','-o','BatchMode=yes','-o','IdentitiesOnly=yes','-o','ConnectTimeout=8',
           '-o','ServerAliveInterval=5','-o','ServerAliveCountMax=2',
           '-i',str(args.ssh_identity),args.ssh_host,
           shlex.join(['python3','-u','-c',SINK,args.display_dir])]
    previous = None
    offset = seq = 0
    rows = []
    by_frame = {}
    with (args.evidence/'transport.log').open('x') as errors, \
            (args.evidence/'states.jsonl').open('x') as states:
        sink = subprocess.Popen(ssh, stdin=subprocess.PIPE, text=True, stderr=errors)
        try:
            deadline = time.monotonic()+args.timeout
            while time.monotonic() < deadline:
                progress = args.results/'progress.json'
                if not progress.exists():
                    time.sleep(.2)
                    continue
                current = progress.read_text()
                if current == previous:
                    time.sleep(.2)
                    continue
                previous = current
                observed = json.loads(current)
                offset, new = read_complete_rows(args.results/'raw.jsonl', offset)
                rows.extend(new)
                for row in new:
                    by_frame[row['frame']] = row
                index = observed.get('frame', rows[-1]['frame'] if rows else 0)
                case = selection['cases'][index]
                measured = by_frame.get(index)
                if measured and observed.get('focal') not in (None, measured['focal']):
                    measured = None
                seq += 1
                state = dict(status=observed['status'],phase=observed['phase'],frame=index,
                    total=len(selection['cases']),runId=run_id,seq=seq,backend=args.backend,
                    label=args.backend+' / '+case['name'],known_crop=True,
                    skin_gap_m=case['skin_gap_m'],model_gap_m=measured['gap_m'] if measured else None,
                    contacts=None,replay_available=False,message=observed.get('message',''),
                    image='near/mesh-'+run_id[:12]+'-'+str(index)+'.png')
                if observed['status'] == 'complete':
                    if set(by_frame) != set(range(len(selection['cases']))):
                        raise ValueError('Completion does not contain every selected reconstruction')
                    milliseconds = statistics.median(r['processing_ms'] for r in rows)
                    summary = dict(backend=args.backend,reconstructions=len(rows),
                        known_crops=len(selection['cases']),processing_median_ms=milliseconds,
                        camera_opened=False,system_input=False,contacts=None)
                    state.update(result=summary,metrics_text=str(len(rows))+' reconstructions / '+
                        str(round(milliseconds,1))+' ms median CPU / supplied crops and side; '
                        'no detector, confidence, identity or gesture verdict.')
                retained = dict(state)
                state['imagePayload'] = base64.b64encode((root/case['image']).read_bytes()).decode()
                sink.stdin.write(json.dumps(state,allow_nan=False)+'\n')
                sink.stdin.flush()
                states.write(json.dumps(retained,allow_nan=False)+'\n');states.flush()
                (args.evidence/'last-state.json').write_text(json.dumps(retained,indent=2)+'\n')
                if observed['status'] in ('complete','error'):
                    break
            else:
                raise RuntimeError('Observer timeout; inspect the actual model task before any retry')
        finally:
            sink.stdin.close()
            try:
                if sink.wait(timeout=25):
                    raise RuntimeError('Display transport failed; model task is independent')
            except subprocess.TimeoutExpired:
                sink.terminate()
                sink.wait(timeout=10)
                raise


if __name__ == '__main__':
    main()
