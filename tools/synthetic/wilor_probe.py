#!/usr/bin/env python3
"""Finite CPU WiLoR reconstruction on known hand crops; no detector/sensors/input.

Uses the pinned provider model and crop API. A known crop/hand side isolates
reconstruction; it is not end-to-end hand detection or input qualification.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import resource
import subprocess
import sys
import time

SOURCE = 'ebec42f94c389070cdd7dda6fd1bf0b4a659c960'


def write_progress(output, state):
    pending = output/'progress.json.pending'
    pending.write_text(json.dumps(state, allow_nan=False)+'\n')
    pending.replace(output/'progress.json')


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024*1024), b''):
            digest.update(block)
    return digest.hexdigest()


def load_selection(path):
    if path.stat().st_size > 1_048_576:
        raise ValueError('Selection exceeds1MiB')
    data = json.loads(path.read_text())
    if data['version'] != 1 or not 1 <= len(data['cases']) <= 512:
        raise ValueError('Expected version1 selection with1..512 cases')
    root = path.resolve().parent
    for case in data['cases']:
        image = (root/case['image']).resolve()
        if root not in image.parents or image.stat().st_size > 16_777_216:
            raise ValueError('Image escapes selection root or exceeds16MiB')
        box = case['bbox']
        if (len(box) != 4 or not all(type(v) in (float, int) and math.isfinite(v) for v in box)
                or not 0 <= box[0] < box[2] <= 4096 or not 0 <= box[1] < box[3] <= 4096
                or case['right'] is not True):
            raise ValueError('Expected finite bounded box and independently known right hand')
        if case.get('crop_source', 'independent_reference_skin_bounds') not in (
                'independent_reference_skin_bounds', 'retained_native_landmarks'):
            raise ValueError('Unsupported declared crop source')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('selection', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--focal', choices=('both', 'provider_default', 'known_renderer'), default='both')
    args = parser.parse_args()
    selection = load_selection(args.selection)
    revision = subprocess.check_output(['git', '-C', str(args.repo), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != SOURCE or subprocess.run(['git', '-C', str(args.repo), 'diff', '--quiet', 'HEAD']).returncode:
        raise ValueError('Expected unchanged pinned provider source')
    manifest = json.loads(Path(__file__).with_name('wilor-assets.json').read_text())
    for entry in manifest['files']:
        path = args.assets/Path(entry['name']).name
        if path.stat().st_size != entry['bytes'] or sha256(path) != entry['sha256']:
            raise ValueError('Asset differs from pinned manifest: '+entry['name'])
    args.output.mkdir(mode=0o700, exist_ok=False)
    rows = []

    def event(**state):
        write_progress(args.output, state)
        print(json.dumps(state, allow_nan=False), flush=True)

    event(status='running', phase='loading', camera_opened=False, system_input=False)
    try:
        sys.path.insert(0, str(args.repo.resolve()))
        import cv2
        import numpy as np
        import torch
        from skimage.filters import gaussian
        from wilor_mini.models.wilor import WiLor
        from wilor_mini.utils import utils

        torch.set_num_threads(2)
        torch.set_num_interop_threads(1)
        torch.manual_seed(0)
        cv2.setNumThreads(1)
        started = time.monotonic()
        model = WiLor(mano_model_path=str(args.assets/'MANO_RIGHT.pkl'),
                      mano_mean_path=str(args.assets/'mano_mean_params.npz'),
                      focal_length=5000, image_size=256)
        checkpoint = torch.load(args.assets/'wilor_final.ckpt', map_location='cpu',
                                weights_only=True, mmap=True)
        loaded = model.load_state_dict(checkpoint['state_dict'], strict=False)
        missing_parameters = set(loaded.missing_keys) & set(dict(model.named_parameters()))
        if missing_parameters:
            raise ValueError('Missing trained parameters: '+str(sorted(missing_parameters)))
        del checkpoint
        model.eval()
        load_s = time.monotonic()-started
        (args.output/'load.json').write_text(json.dumps(dict(source_commit=revision,
            model_manifest=manifest, selection_sha256=sha256(args.selection),
            missing_keys=loaded.missing_keys, unexpected_keys=loaded.unexpected_keys,
            load_seconds=load_s, torch=torch.__version__, python=sys.version), indent=2)+'\n')
        root = args.selection.resolve().parent
        with (args.output/'raw.jsonl').open('x') as raw, torch.inference_mode():
            for index, case in enumerate(selection['cases']):
                image_path = root/case['image']
                image = cv2.imread(str(image_path))
                if image is None or max(image.shape[:2]) > 4096:
                    raise ValueError('Failed bounded image decode')
                box = np.asarray(case['bbox'], dtype=np.float32)
                if box[2] > image.shape[1] or box[3] > image.shape[0]:
                    raise ValueError('Known crop outside actual image')
                center = (box[2:]+box[:2])/2
                size = 2.5*float(max(box[2:]-box[:2]))
                original = image
                downsample = size/256/2
                if downsample > 1.1:
                    image = gaussian(image, sigma=(downsample-1)/2,
                                     channel_axis=2, preserve_range=True)
                crop, transform = utils.generate_image_patch_cv2(image, *center, size, size,
                    256, 256, False, 1.0, 0, border_mode=cv2.BORDER_CONSTANT)
                # Provider WiLor.forward flips BGR to RGB before normalization.
                tensor = torch.from_numpy(crop[None]).to(dtype=torch.float32)
                focal_cases = [('provider_default', 5000.0),
                               ('known_renderer', case['focal_px']*256/max(original.shape[:2]))]
                for focal_label, focal in focal_cases:
                    if args.focal != 'both' and args.focal != focal_label:
                        continue
                    model.FOCAL_LENGTH = focal
                    event(status='running', phase='inference', frame=index, total=len(selection['cases']),
                          case=case['name'], focal=focal_label)
                    tick = time.monotonic()
                    output = {k:v.cpu().float().numpy() for k,v in model(tensor).items()}
                    elapsed_ms = (time.monotonic()-tick)*1000
                    world = output['pred_keypoints_3d'][0]
                    if world.shape != (21, 3) or not np.isfinite(world).all():
                        raise ValueError('Unexpected model landmark contract')
                    full_focal = focal/256*max(original.shape[:2])
                    translation = utils.cam_crop_to_full(output['pred_cam'], center[None], size,
                        np.array([[original.shape[1], original.shape[0]]]), full_focal)
                    projected = utils.perspective_projection(world[None], translation=translation,
                        focal_length=np.array([[full_focal, full_focal]]),
                        camera_center=np.array([[original.shape[1]/2, original.shape[0]/2]]))[0]
                    row = dict(schema='wigglecontrol.mesh-reference/v1', frame=index, image=case['image'],
                        case=case['name'], camera_opened=False, system_input=False,
                        crop_source=case.get('crop_source', 'independent_reference_skin_bounds'),
                        handedness_source='reference_right_hand',
                        focal=focal_label, focal_normalized=focal, processing_ms=elapsed_ms,
                        image_sha256=sha256(image_path), bbox=case['bbox'], crop_transform=transform.tolist(),
                        world_landmarks=world.tolist(), landmarks=projected.tolist(),
                        gap_m=float(np.linalg.norm(world[4]-world[8])),
                        vertices_m=output['pred_vertices'][0].tolist(),
                        reference_skin_gap_m=case['skin_gap_m'], reference_source_frame=case['source_frame'],
                        contacts=None, stable_id_available=False, presence_confidence_available=False,
                        gesture_meaning_evaluated=False)
                    rows.append(row)
                    raw.write(json.dumps(row, allow_nan=False)+'\n')
                    raw.flush()
                    event(status='running', phase='measured', frame=index, total=len(selection['cases']),
                          case=case['name'], focal=focal_label, gap_m=row['gap_m'],
                          reference_skin_gap_m=row['reference_skin_gap_m'], processing_ms=elapsed_ms)
        result = dict(schema='wigglecontrol.mesh-reference-report/v1', camera_opened=False,
                      system_input=False, source_commit=revision, load_seconds=load_s,
                      max_rss_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss*1024,
                      predictions=rows, limits='Known crops/right hands, static reconstruction only; '
                      'no detector, gesture semantics, calibrated physical scale or receiving acceptance')
        (args.output/'results.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
        event(status='complete', phase='retained', predictions=len(rows), max_rss_bytes=result['max_rss_bytes'])
    except BaseException as error:
        event(status='error', phase='failed', message=str(error), retained_predictions=len(rows))
        raise


if __name__ == '__main__':
    main()
