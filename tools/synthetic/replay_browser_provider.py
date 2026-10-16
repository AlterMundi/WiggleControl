#!/usr/bin/env python3
"""Feed saved browser landmarks to actual Rust tracking/meaning, without native IDs.

This is offline source-time replay, not capture/worker transport or input delivery.
The provider's configured presence threshold is a floor, not its handedness score.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess

MODEL_SHA = 'fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def right_probability(categories):
    if len(categories) != 1:
        raise ValueError('Expected the retained binary handedness category')
    category = categories[0]
    score = category['score']
    name = category['categoryName']
    if name not in ('Left', 'Right') or not math.isfinite(score) or not 0 <= score <= 1:
        raise ValueError('Invalid handedness category')
    return score if name == 'Right' else 1-score


def commands(case, images, delay_us):
    width, height = case['reference']['image_size']
    rows = case['predictions']
    if case['camera_opened'] or case['system_input'] or not 1 <= len(rows) <= 10000:
        raise ValueError('Expected finite camera/device-free predictions')
    if len(rows) != len(case['reference']['frames']):
        raise ValueError('Incomplete source reference')
    result = [dict(type='start')]
    previous = -1
    for index, (row, reference) in enumerate(zip(rows, case['reference']['frames'])):
        if (row['frame'] != index or reference['frame'] != index
                or row['source_us'] != reference['source_us'] or row['source_us'] <= previous
                or row['camera_opened'] or row['system_input']):
            raise ValueError('Source frame/time or acquisition differs')
        previous = row['source_us']
        image = (images/row['image']).resolve()
        if images.resolve() not in image.parents or digest(image) != row['image_sha256']:
            raise ValueError('Image escapes scene root or digest differs')
        hands = [dict(landmarks=h['landmarks'], world_landmarks=h['world_landmarks'],
                      right_probability=right_probability(h['handedness'])) for h in row['hands']]
        for hand, raw in zip(hands, row['hands']):
            if raw.get('presence_confidence') is not None:
                hand['presence_confidence'] = raw['presence_confidence']
            if raw.get('image_depths') is not None:
                hand['image_depths'] = raw['image_depths']
        result.append(dict(type='frame', observation=dict(sequence=index+1,
            source_us=row['source_us'], now_us=row['source_us']+delay_us,
            width=width, height=height, hands=hands)))
    result.append(dict(type='stop'))
    if any(len(json.dumps(row).encode())+1 > 16384 for row in result):
        raise ValueError('Provider command exceeds the bounded Rust reader')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    parser.add_argument('--images', type=Path, required=True, help='Parent of SCENE/image files')
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--session', type=Path, required=True)
    parser.add_argument('--settings', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True, help='New evidence directory')
    parser.add_argument('--delay-us', type=int, default=0,
                        help='Artificial constant age, not measured IPC latency')
    args = parser.parse_args()
    if not 0 <= args.delay_us <= 10000000:
        parser.error('Delay must be0..10000000 microseconds')
    if args.report.stat().st_size > 64*1024*1024:
        raise ValueError('Browser report exceeds64MiB')
    data = json.loads(args.report.read_text())
    if 'state' in data:
        if data['state'] != 'complete':
            raise ValueError('Expected completed browser producer')
        data = data['result']
    if (data['schema'] != 'wigglecontrol.browser-provider-comparison/v1'
            or data['package_version'] != '1.1.1' or data['model_sha256'] != MODEL_SHA
            or data['camera_opened'] or data['system_input']):
        raise ValueError('Expected pinned Tasks Vision1.1.1 offline report')
    files = [args.report, args.binary, args.session, args.settings]
    hashes = {str(p): digest(p) for p in files}
    args.output.mkdir(mode=0o700)
    names = set()
    for case in data['results']:
        scene, delegate = case['scene'], case['delegate']
        name = delegate+'-'+scene
        if not re.fullmatch(r'[A-Za-z0-9_-]{1,100}', name) or name in names or delegate not in ('CPU','GPU'):
            raise ValueError('Invalid or duplicate scene/delegate')
        names.add(name)
        sequence = commands(case, args.images/scene, args.delay_us)
        wire = ''.join(json.dumps(c, allow_nan=False)+'\n' for c in sequence)
        (args.output/(name+'-commands.jsonl')).write_text(wire)
        result = subprocess.run([str(args.binary.resolve()), 'provider-replay',
            str(args.session), str(args.settings), '0.5'], input=wire,
            text=True, capture_output=True, timeout=30)
        (args.output/(name+'.jsonl')).write_text(result.stdout)
        (args.output/(name+'.stderr')).write_text(result.stderr)
        result.check_returncode()
        receipts = [json.loads(s) for s in result.stdout.splitlines()]
        if len(receipts) != len(sequence)+1 or any(
                r['camera_opened'] or r['system_input'] for r in receipts):
            raise ValueError('Rust provider output contract differs')
        if any(r['controller']['contacts'] or r['controller']['button_down'] for r in receipts[-2:]):
            raise ValueError('Stop/EOF failed to release')
        predictions = []
        for source, receipt in zip(case['predictions'], receipts[1:-2]):
            if receipt['sequence'] != source['frame']+1 or receipt['source_us'] != source['source_us']:
                raise ValueError('Rust source association differs')
            predictions.append(dict(receipt, frame=source['frame'], image=source['image'],
                                    contacts=receipt['controller']['contacts']))
        output = dict(schema='wigglecontrol.browser-provider-controller/v1',
            camera_opened=False, system_input=False, offline_now_equals_source=args.delay_us==0,
            artificial_age_us=args.delay_us, delegate=delegate, reference=case['reference'],
            config_sha256=hashes[str(args.session)], hashes=hashes, predictions=predictions,
            limits='Actual Rust spatial track/meaning from saved provider points. No inherited native IDs, '
                   'quality, eligibility or cursor. Presence0.5 is configured SDK threshold floor. '
                   'No capture, worker/IPC, physical acceptance or system input.')
        (args.output/(name+'.json')).write_text(json.dumps(output, indent=2, allow_nan=False)+'\n')
        print(name, len(predictions), 'frames;', sum(r['ambiguous'] for r in predictions), 'ambiguous')
    if hashes != {str(p): digest(p) for p in files}:
        raise ValueError('Source, binary or configuration changed during replay')


if __name__ == '__main__':
    main()
