#!/usr/bin/env python3
"""Finite MobRecon SpiralConv comparison on declared right-hand crops.

No detector, confidence, identity, camera, gesture prediction or system input.
Only Euclidean joint gaps are compared; rigid camera registration is unnecessary
for that norm and is explicitly excluded from the measured inference time.
"""
import argparse
import json
from pathlib import Path
import resource
import sys
import time
from types import SimpleNamespace

from mobrecon_prepare import verify_source
from wilor_probe import load_selection, sha256, write_progress

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('selection', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--checkpoint', type=Path, required=True)
    parser.add_argument('--topology', type=Path, required=True)
    args = parser.parse_args()
    selection = load_selection(args.selection)
    revision = verify_source(args.repo)
    manifest = json.loads(Path(__file__).with_name('mobrecon-assets.json').read_text())
    checkpoint_sha = manifest['checkpoint']['sha256']
    if (args.checkpoint.stat().st_size != manifest['checkpoint']['bytes']
            or sha256(args.checkpoint) != checkpoint_sha
            or sha256(args.topology) != manifest['prepared_topology']['sha256']):
        raise ValueError('Checkpoint or prepared topology differs from selected evidence')
    args.output.mkdir(mode=0o700, exist_ok=False)

    def event(**state):
        write_progress(args.output, state)
        print(json.dumps(state, allow_nan=False), flush=True)

    event(status='running', phase='loading', camera_opened=False, system_input=False)
    try:
        import cv2
        import numpy as np
        import torch
        sys.path.insert(0, str(args.repo.resolve()))
        from cmr.models.mobrecon_densestack import MobRecon
        from cmr.datasets.FreiHAND.kinematics import mano_to_mpii
        from utils.vis import base_transform
        torch.set_num_threads(2)
        torch.set_num_interop_threads(1)
        torch.manual_seed(0)
        cv2.setNumThreads(1)
        tick = time.monotonic()
        with np.load(args.topology, allow_pickle=False) as topology:
            spirals = [torch.from_numpy(topology['spiral_'+str(i)]) for i in range(4)]
            up = [tuple(torch.from_numpy(topology[k+'_'+str(i)]) for k in ('row','col','value'))
                  for i in range(4)]
            regressor = topology['j_regressor'].copy()
        if regressor.shape != (21,778):
            raise ValueError('Unexpected MANO joint regression contract')
        model = MobRecon(SimpleNamespace(out_channels=[32,64,128,256], dsconv=False), spirals, up)
        checkpoint = torch.load(args.checkpoint, map_location='cpu', weights_only=True)
        model.load_state_dict(checkpoint, strict=True)
        del checkpoint
        model.eval()
        load = dict(source_commit=revision, checkpoint_sha256=checkpoint_sha,
            topology_sha256=sha256(args.topology), selection_sha256=sha256(args.selection),
            load_seconds=time.monotonic()-tick, torch=torch.__version__, python=sys.version,
            input_size=128, crop_scale=1.3, output_scale_m=0.20,
            registration=False, camera_opened=False, system_input=False)
        (args.output/'load.json').write_text(json.dumps(load,indent=2)+'\n')
        rows=[]
        root=args.selection.resolve().parent
        with (args.output/'raw.jsonl').open('x') as raw, torch.inference_mode():
            for index, case in enumerate(selection['cases']):
                image_path=root/case['image']
                image=cv2.imread(str(image_path))
                if image is None or max(image.shape[:2]) > 4096:
                    raise ValueError('Failed bounded image decode')
                box=np.asarray(case['bbox'],dtype=np.float64)
                if box[2] > image.shape[1] or box[3] > image.shape[0]:
                    raise ValueError('Provided box outside actual image')
                center=(box[2:]+box[:2])/2
                half=max(box[2:]-box[:2])*1.3/2
                # Match the provider's non-augmented training crop geometry.
                bounds=[int(center[0]-half),int(center[1]-half),
                        int(center[0]+half),int(center[1]+half)]
                x1,y1,x2,y2=bounds
                transform=np.array([[127/(x2-x1),0,-127*x1/(x2-x1)],
                                    [0,127/(y2-y1),-127*y1/(y2-y1)]],dtype=np.float32)
                crop=cv2.warpAffine(image[...,::-1],transform,(128,128),borderMode=cv2.BORDER_CONSTANT)
                tensor=torch.from_numpy(base_transform(crop,size=128)).unsqueeze(0)
                event(status='running',phase='inference',frame=index,total=len(selection['cases']),case=case['name'])
                tick=time.monotonic()
                output=model(tensor)
                vertices=output['mesh_pred'][0].cpu().numpy()*0.20
                points=mano_to_mpii(regressor@vertices)
                uv=output['uv_pred'][0].cpu().numpy()
                elapsed_ms=(time.monotonic()-tick)*1000
                if (vertices.shape != (778,3) or points.shape != (21,3) or uv.shape != (21,2)
                        or not all(np.isfinite(v).all() for v in (vertices,points,uv))):
                    raise ValueError('Unexpected finite mesh/landmark contract')
                row=dict(schema='wigglecontrol.mesh-reference/v1',frame=index,image=case['image'],
                    case=case['name'],image_sha256=sha256(image_path),camera_opened=False,system_input=False,
                    crop_source=case.get('crop_source','independent_reference_skin_bounds'),
                    crop_bounds=bounds,crop_transform=transform.tolist(),focal='not_required_for_gap',
                    reference_source_frame=case['source_frame'],source_us=case.get('source_us'),
                    reference_case=case.get('reference_case'),skin_gap_m=case['skin_gap_m'],
                    gap_m=float(np.linalg.norm(points[4]-points[8])),processing_ms=elapsed_ms,
                    world_landmarks=points.tolist(),vertices_m=vertices.tolist(),uv=uv.tolist(),
                    contacts=None,stable_id_available=False,presence_confidence_available=False,
                    gesture_meaning_evaluated=False)
                raw.write(json.dumps(row,allow_nan=False)+'\n');raw.flush();rows.append(row)
                event(status='running',phase='measured',frame=index,total=len(selection['cases']),
                      case=case['name'],gap_m=row['gap_m'],processing_ms=elapsed_ms)
        result=dict(schema='wigglecontrol.mesh-probe/v1',backend='MobRecon SpiralConv CPU',
            camera_opened=False,system_input=False,known_crop=True,rows=len(rows),
            processing_ms=[r['processing_ms']for r in rows],load=load,
            max_rss_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss*1024)
        (args.output/'results.json').write_text(json.dumps(result,indent=2)+'\n')
        event(status='complete',phase='retained',frame=len(rows)-1,total=len(rows))
    except Exception as error:
        event(status='error',phase='failed',message=str(error))
        raise


if __name__ == '__main__':
    main()
