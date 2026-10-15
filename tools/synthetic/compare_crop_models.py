#!/usr/bin/env python3
"""Compare pinned Tasks landmark weights on exact native exported float tensors.

This bypasses both trackers. It cannot establish a complete backend, identity,
gesture intent or receiving performance. No camera or input device is opened.
"""
import argparse
import hashlib
import importlib.metadata
import json
import math
from pathlib import Path
import time

import numpy as np
from ai_edge_litert.interpreter import Interpreter


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class FixedCropComparison:
    def __init__(self, model, expected_sha256):
        if importlib.metadata.version('ai-edge-litert') != '2.3.0':
            raise ValueError('This probe requires ai-edge-litert==2.3.0')
        self.model_path = Path(model)
        self.model_sha256 = digest(self.model_path)
        if self.model_sha256 != expected_sha256:
            raise ValueError('Model SHA256 mismatch')
        self.model = Interpreter(model_path=str(self.model_path), num_threads=1)
        self.model.allocate_tensors()
        inputs = self.model.get_input_details()
        self.outputs = {x['name']: x for x in self.model.get_output_details()}
        if (len(inputs) != 1 or inputs[0]['shape'].tolist() != [1, 224, 224, 3]
                or inputs[0]['dtype'] != np.float32
                or set(self.outputs) != {'Identity', 'Identity_1', 'Identity_2', 'Identity_3'}
                or any(x['dtype'] != np.float32 for x in self.outputs.values())
                or any(self.outputs[n]['shape'].tolist() != shape for n, shape in
                       [('Identity', [1, 63]), ('Identity_1', [1, 1]),
                        ('Identity_2', [1, 1]), ('Identity_3', [1, 63])])):
            raise ValueError('Unexpected pinned landmark tensor contract')
        self.input_index = inputs[0]['index']

    def compare(self, directory):
        directory = Path(directory)
        report_path, tensor_path = directory/'report.json', directory/'input.f32'
        if report_path.stat().st_size > 65536 or tensor_path.stat().st_size != 224*224*3*4:
            raise ValueError('Unexpected crop report/tensor size')
        hashes = {str(p): digest(p) for p in [report_path, tensor_path]}
        report = json.loads(report_path.read_text())
        if (report['schema'] != 'wigglecontrol.crop-check/v1'
                or report['camera_opened'] or report['system_input']
                or report['tensor'] != dict(file='input.f32', shape=[1, 224, 224, 3],
                                           dtype='little-endian float32', rgb_range=[0, 1])):
            raise ValueError('Expected exact camera/device-free native RGB tensor')
        roi = report['roi']
        if len(roi) != 4 or not all(math.isfinite(v) for v in roi) or roi[2] <= 0:
            raise ValueError('Invalid fixed ROI')
        tensor = np.fromfile(tensor_path, dtype='<f4').reshape(1, 224, 224, 3)
        if not np.isfinite(tensor).all() or tensor.min() < 0 or tensor.max() > 1.000001:
            raise ValueError('Invalid RGB tensor values')
        # Native little-endian values are reused directly, with no PNG roundtrip,
        # new resize, normalization, detector or crop reconstruction.
        tensor = tensor.astype(np.float32, copy=False)
        self.model.set_tensor(self.input_index, tensor)
        started = time.monotonic()
        self.model.invoke()
        elapsed = (time.monotonic()-started)*1000
        output = {name: self.model.get_tensor(d['index']).copy()
                  for name, d in self.outputs.items()}
        if not all(np.isfinite(v).all() for v in output.values()):
            raise ValueError('Nonfinite landmark output')
        confidence = float(output['Identity_1'][0, 0])
        side = float(output['Identity_2'][0, 0])
        if not 0 <= confidence <= 1 or not 0 <= side <= 1:
            raise ValueError('Invalid presence/handedness output')
        world = output['Identity_3'].reshape(21, 3).astype(np.float64)
        local = output['Identity'].reshape(21, 3)[:, :2].astype(np.float64)/224-.5
        c, s = math.cos(roi[3]), math.sin(roi[3])
        pixels = local @ np.array([[c, s], [-s, c]])*roi[2]+np.array(roi[:2])
        palm = math.dist(pixels[0], pixels[9])
        if palm <= 1e-6:
            raise ValueError('Degenerate predicted palm')
        result = dict(schema='wigglecontrol.fixed-crop-model-comparison/v1',
            camera_opened=False, system_input=False, model_sha256=self.model_sha256,
            hashes=hashes, roi=roi, native=report['native'], native_confidence=report['confidence'],
            alternate=dict(gap_m=math.dist(world[4], world[8]), confidence=confidence,
                handedness=side, world_landmarks=world.tolist(), landmarks=pixels.tolist(),
                onset_ratio=math.dist(pixels[4], pixels[8])/palm),
            invoke_ms=elapsed,
            limits='Same fixed tensor, different released model/runtime. No tracker, owner, human intent or gesture output qualification.')
        if hashes != {str(p): digest(p) for p in [report_path, tensor_path]}:
            raise ValueError('Crop inputs changed during comparison')
        if digest(self.model_path) != self.model_sha256:
            raise ValueError('Model changed during comparison')
        return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('model', type=Path)
    p.add_argument('crops', type=Path, nargs='+')
    p.add_argument('--expected-sha256', required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    if not 1 <= len(args.crops) <= 1000:
        p.error('Expected1..1000 exported fixed crops')
    model = FixedCropComparison(args.model, args.expected_sha256)
    rows = [model.compare(d) for d in args.crops]
    with args.output.open('x') as file:
        file.write(json.dumps(dict(camera_opened=False, system_input=False, rows=rows),
                              indent=2, allow_nan=False)+'\n')


if __name__ == '__main__':
    main()
