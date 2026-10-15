#!/usr/bin/env python3
"""Conditional metric replacement in the actual offline controller.

Keep native confidence, eligibility, identity, cursor and source time. Replace
only estimated gap with a matched known-crop mesh output. This is not a detector,
real-time adapter, new identity/confidence proof or physical input acceptance.
"""
import argparse
import copy
import json
import math
from pathlib import Path

from compare_pinch_interpretations import digest, matched_geometry, replay, score


def substitute(native, selection, mesh_rows, group, images, focal):
    matched = {}
    for row in mesh_rows:
        case = selection['cases'][row['frame']]
        if case.get('reference_case') != group or row['focal'] != focal:
            continue
        frame = case['source_frame']
        if frame in matched or row['reference_source_frame'] != frame:
            raise ValueError('Duplicate/mismatched mesh frame')
        matched[frame] = (case, row)
    if set(matched) != {r['frame'] for r in native}:
        raise ValueError('Selected group must contain the entire native trajectory')
    replaced = copy.deepcopy(native)
    for original, output in zip(native, replaced):
        case, row = matched[original['frame']]
        if (case['source_us'] != original['source_us'] or len(output['hands']) != 1
                or row['image_sha256'] != digest(images/original['image'])):
            raise ValueError('Known crop/native observation source mismatch')
        if not math.isfinite(row['gap_m']) or row['gap_m'] < 0:
            raise ValueError('Mesh gap is not a finite nonnegative metric')
        # Native source confidence/identity are retained, not attributed to WiLoR.
        output['hands'][0]['pinch_distance_m'] = row['gap_m']
    return replaced


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('native_results', type=Path)
    parser.add_argument('mesh_jsonl', type=Path, help='Complete lines only; the producer may still be running')
    parser.add_argument('selection', type=Path)
    parser.add_argument('--group', required=True)
    parser.add_argument('--images', type=Path, required=True)
    parser.add_argument('--geometry', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--settings', type=Path, required=True)
    parser.add_argument('--focal', default='provider_default')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    data = json.loads(args.native_results.read_text())
    if data['camera_opened'] or data['system_input']:
        raise ValueError('Expected camera/device-free native evidence')
    selection = json.loads(args.selection.read_text())
    load = json.loads(args.mesh_jsonl.with_name('load.json').read_text())
    if load['selection_sha256'] != digest(args.selection):
        raise ValueError('Selection differs from the actual model load')
    # Preserve a pinned observation of complete rows even during producer append.
    rows = [json.loads(line) for line in args.mesh_jsonl.read_text().splitlines(keepends=True)
            if line.endswith('\n')]
    if any(r['camera_opened'] or r['system_input'] for r in rows):
        raise ValueError('Expected camera/device-free reconstruction')
    original = data['predictions']
    replacement = substitute(original, selection, rows, args.group, args.images, args.focal)
    reference = data['reference']
    marker_error = None
    if args.geometry:
        reference, marker_error = matched_geometry(reference, json.loads(args.geometry.read_text()))
    settings = json.loads(args.settings.read_text())
    comparisons = []
    for stride, offset in ((1, 0), (2, 0), (2, 1)):
        values = {}
        for name, observations in [('native', original), ('known_crop_mesh_metric', replacement)]:
            replayed = replay(args.binary.resolve(), settings, observations, False,
                settings['gestures']['close_m'], settings['gestures']['open_m'], stride, offset,
                settings['gestures'].get('confirm_us', 40000))
            values[name] = score(reference, replayed)
            if name == 'native' and stride == 1:
                if [bool(o['contacts']) for o in original] != [bool(o['contacts']) for _, o in replayed]:
                    raise ValueError('Baseline does not reproduce native contacts')
        comparisons.append(dict(stride=stride, offset=offset, **values))
    result = dict(schema='wigglecontrol.mesh-metric-comparison/v1', camera_opened=False,
        system_input=False, group=args.group, source_commit=load['source_commit'],
        selection_sha256=digest(args.selection), native_results_sha256=digest(args.native_results),
        binary_sha256=digest(args.binary), settings_sha256=digest(args.settings),
        matched_geometry_error_m=marker_error, comparisons=comparisons,
        limits='Conditional oracle-crop metric replacement; native confidence/identity retained. '
               'Offline source clock only; no detector, real-time, physical or runtime-adoption claim.')
    with args.output.open('x') as stream:
        stream.write(json.dumps(result, indent=2, allow_nan=False)+'\n')


if __name__ == '__main__':
    main()
