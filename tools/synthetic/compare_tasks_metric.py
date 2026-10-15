#!/usr/bin/env python3
"""Conditional Tasks metric/onset replay; not an alternative identity pipeline.

Use matched single-hand images/geometry. Keep native ID, eligibility, confidence,
cursor and time; missing Tasks detections remove the observation. Never assign
Tasks array positions as identities. No inference, camera or system input.
"""
import argparse
import copy
import json
import math
from pathlib import Path
import statistics

from compare_pinch_interpretations import digest, matched_geometry, replay, score


def substitute(native, tasks, onset_ratio=None):
    if len(native) != len(tasks):
        raise ValueError('Expected complete matching trajectories')
    replaced = copy.deepcopy(native)
    for original, output, provider in zip(native, replaced, tasks):
        if any(original[k] != provider[k] for k in ('frame', 'source_us', 'image')):
            raise ValueError('Provider frame/time/image differs from native source')
        if len(original['hands']) > 1 or len(provider['hands']) > 1:
            raise ValueError('Conditional comparison requires at most one hand; no array identity')
        if any(not h.get('pinch_onset_allowed', True) for h in original['hands']):
            raise ValueError('Expected ungated native baseline')
        if not original['hands'] or not provider['hands']:
            output['hands'] = []
            continue
        hand = provider['hands'][0]
        world = hand['world_landmarks']
        if (len(world) != 21 or any(len(p) != 3 or any(not math.isfinite(v) for v in p)
                                    for p in world)):
            raise ValueError('Invalid provider world landmarks')
        gap = math.dist(world[4], world[8])
        if not math.isclose(gap, hand['world_pinch_distance_m'], rel_tol=1e-9, abs_tol=1e-12):
            raise ValueError('Declared gap differs from provider landmarks')
        output['hands'][0]['pinch_distance_m'] = gap
        if onset_ratio is not None:
            points = hand['landmarks']
            if (len(points) != 21 or any(len(p) != 2 or any(not math.isfinite(v) for v in p)
                                        for p in points)):
                raise ValueError('Invalid provider pixel landmarks')
            palm = math.dist(points[0], points[9])
            if palm <= 1e-6:
                raise ValueError('Degenerate projected palm')
            output['hands'][0]['pinch_onset_allowed'] = math.dist(points[4], points[8])/palm <= onset_ratio
    return replaced


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('native', type=Path)
    parser.add_argument('tasks', type=Path)
    parser.add_argument('--images', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--settings', type=Path, required=True)
    parser.add_argument('--onset-ratio', type=float)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.onset_ratio is not None and not 0 < args.onset_ratio <= 2:
        parser.error('Onset ratio requires0<ratio<=2')
    inputs = [args.native, args.tasks, args.binary, args.settings]
    hashes = {str(p): digest(p) for p in inputs}
    native, tasks = [json.loads(p.read_text()) for p in inputs[:2]]
    if any(d['camera_opened'] or d['system_input'] for d in (native,tasks)):
        raise ValueError('Expected camera/device-free evidence')
    if tasks['backend'] != 'mediapipe-1.1.1-cpu':
        raise ValueError('Expected pinned Tasks CPU comparison')
    reference, error = matched_geometry(native['reference'], tasks['reference'])
    if reference.get('reference_hand_count') != 1:
        raise ValueError('Only single-reference-hand geometry is qualified')
    rows = native['predictions']
    if len(rows) != len(reference['frames']):
        raise ValueError('Incomplete native trajectory')
    intervals = {b['source_us']-a['source_us'] for a,b in zip(rows,rows[1:])}
    interval = next(iter(intervals)) if len(intervals)==1 else None
    if any(t<=0 for t in intervals):
        raise ValueError('Expected strictly increasing source time')
    root = args.images.resolve()
    for row in rows:
        path = (root/row['image']).resolve()
        if root not in path.parents:
            raise ValueError('Image path escapes root')
        hashes[str(path)] = digest(path)
    settings = json.loads(args.settings.read_text())
    variants = [('native',rows), ('tasks_metric',substitute(rows,tasks['predictions']))]
    if args.onset_ratio is not None:
        variants.append(('tasks_metric_and_onset',substitute(rows,tasks['predictions'],args.onset_ratio)))
        variants.append(('native_onset',rows))
    cases = []
    unions = []
    for stride, offset in ((1,0),(2,0),(2,1)):
        performed = {}
        for label, predictions in variants:
            replayed = replay(args.binary.resolve(),settings,predictions,False,
                settings['gestures']['close_m'],settings['gestures']['open_m'],stride,offset,
                settings['gestures'].get('confirm_us',40000),
                args.onset_ratio if label=='native_onset' else None)
            if label == 'native' and stride == 1:
                if [o['contacts'] for _,o in replayed] != [r['contacts'] for r in rows]:
                    raise ValueError('Raw baseline does not reproduce retained contacts')
            cases.append(dict(variant=label,stride=stride,offset=offset,
                              source_interval_us=interval*stride if interval else None,**score(reference,replayed)))
            performed[label] = replayed
        if args.onset_ratio is not None:
            primary,secondary = performed['native_onset'],performed['tasks_metric_and_onset']
            combined = []
            for (row,a),(other,b) in zip(primary,secondary):
                if row['frame'] != other['frame'] or row['source_us'] != other['source_us']:
                    raise ValueError('Counterfactual source frames differ')
                # Only a Boolean for the pure scorer: never create/deliver a
                # fused contact ID or pretend this is a controller output.
                combined.append((row,dict(contacts=bool(a['contacts'] or b['contacts']))))
            counted = score(reference,combined)
            counted.pop('stop_released')
            unions.append(dict(stride=stride,offset=offset,implemented=False,
                               source_interval_us=interval*stride if interval else None,
                               component_stop_released=True,**counted))
    if hashes != {p: digest(Path(p)) for p in hashes}:
        raise ValueError('Source/binary/settings/images changed during comparison')
    report = dict(schema='wigglecontrol.tasks-metric-probe/v1',camera_opened=False,system_input=False,
        native_sha256=hashes[str(args.native)],tasks_sha256=hashes[str(args.tasks)],
        hashes=hashes,geometry_marker_max_error_m=error,onset_ratio=args.onset_ratio,
        tasks_detections=sum(bool(r['hands']) for r in tasks['predictions']),
        tasks_processing_median_ms=statistics.median(r['processing_ms'] for r in tasks['predictions']),
        tasks_model_sha256=tasks.get('model_sha256'),cases=cases,counterfactual_unions=unions,
        limits='Conditional single-hand metric/onset only; native confidence/eligibility/ID/cursor retained. '
               'Missing Tasks observations cancel. Union is an analytic OR of separately confirmed decisions, '
               'not implemented contact IDs/lifecycle/asynchronous fusion. Not a full backend, identity, '
               'fair total-cost, Pi or physical qualification.')
    with args.output.open('x') as stream:
        stream.write(json.dumps(report,indent=2,allow_nan=False)+'\n')


if __name__ == '__main__':
    main()
