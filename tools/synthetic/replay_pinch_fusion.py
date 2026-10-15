#!/usr/bin/env python3
"""Execute same-frame pinch fusion through Rust, using retained conditional evidence.

Tasks array positions are never identities. This single-hand experiment inherits
native ownership/quality, not a provider confidence estimate or a live backend.
"""
import argparse
import copy
import json
from pathlib import Path

from compare_pinch_interpretations import digest, encoded_hands, matched_geometry, replay, score
from compare_tasks_metric import substitute


def attach_secondary(native, tasks, onset_ratio):
    if any(h.get('secondary_pinch') is not None for r in native for h in r['hands']):
        raise ValueError('Expected single-hypothesis native source')
    metrics = substitute(native,tasks,onset_ratio)
    output = copy.deepcopy(native)
    for original,metric,target in zip(native,metrics,output):
        target['hands'] = encoded_hands(original,False,onset_ratio)
        if not target['hands'] or not metric['hands']:
            continue
        primary,secondary = target['hands'][0],metric['hands'][0]
        primary['secondary_pinch'] = dict(source_id=2,source_us=original['source_us'],
            confidence=original['hands'][0]['confidence'],
            eligible=original['hands'][0]['engaged'] and original['hands'][0]['pinch_eligible'],
            distance_m=secondary['pinch_distance_m'],
            onset_allowed=secondary.get('pinch_onset_allowed',True))
    return output


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('native',type=Path);p.add_argument('tasks',type=Path)
    p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--settings',type=Path,required=True)
    p.add_argument('--onset-ratio',type=float,default=.38)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    if not 0<args.onset_ratio<=2:p.error('Expected0<onset ratio<=2')
    files=[args.native,args.tasks,args.binary,args.settings]
    hashes={str(f):digest(f)for f in files}
    n,t=[json.loads(f.read_text())for f in files[:2]]
    if any(d['camera_opened']or d['system_input']for d in [n,t]):
        raise ValueError('Expected camera/device-free sources')
    if t['backend']!='mediapipe-1.1.1-cpu':raise ValueError('Expected pinned Tasks source')
    reference,error=matched_geometry(n['reference'],t['reference'])
    if reference.get('reference_hand_count')!=1:raise ValueError('Only single-hand matching qualified')
    settings=json.loads(args.settings.read_text());rows=n['predictions']
    fused=attach_secondary(rows,t['predictions'],args.onset_ratio)
    metric=substitute(rows,t['predictions'],args.onset_ratio)
    cases=[];trace=[]
    for stride,phase in [(1,0),(2,0),(2,1)]:
        def run(source,gate=None):
            return replay(args.binary.resolve(),settings,source,False,settings['gestures']['close_m'],
                settings['gestures']['open_m'],stride,phase,settings['gestures'].get('confirm_us',40000),gate)
        raw=run(rows)
        if stride==1 and [o['contacts']for _,o in raw]!=[r['contacts']for r in rows]:
            raise ValueError('Legacy baseline differs from retained contacts')
        primary,secondary,actual=run(rows,args.onset_ratio),run(metric),run(fused)
        expected=[bool(a['contacts']or b['contacts'])for (_,a),(_,b)in zip(primary,secondary)]
        if expected!=[bool(o['contacts'])for _,o in actual]:
            raise ValueError('Implemented fusion differs from separately confirmed component decisions')
        previous=None
        for row,out in actual:
            if len(out['contacts'])>1:raise ValueError('A hypothesis became an extra contact')
            current=out['contacts'][0]['id']if out['contacts']else None
            if current is not None and previous is not None and current!=previous:
                raise ValueError('Contact ID changed while fused hold remained active')
            previous=current
            if stride==1:trace.append(dict(frame=row['frame'],source_us=row['source_us'],
                hands=row['hands'],controller=out,contacts=out['contacts']))
        cases.append(dict(stride=stride,phase=phase,component_union_matches=True,
            single_contact=True,continuous_hold_id=True,**score(reference,actual)))
    if hashes!={str(f):digest(f)for f in files}:raise ValueError('Inputs changed during replay')
    result=dict(schema='wigglecontrol.implemented-pinch-fusion/v1',camera_opened=False,system_input=False,
        hashes=hashes,geometry_marker_max_error_m=error,onset_ratio=args.onset_ratio,cases=cases,trace=trace,
        confidence_policy='Secondary quality inherited from native hand for conditional metric comparison; not Tasks confidence',
        limits='Actual Rust meaning/IDs/lifecycle on controlled single-hand matched frames. '
               'No complete Tasks runtime, multi-hand match, asynchronous inference, receiving or physical acceptance.')
    with args.output.open('x')as f:f.write(json.dumps(result,indent=2,allow_nan=False)+'\n')


if __name__=='__main__':main()
