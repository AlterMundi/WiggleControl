#!/usr/bin/env python3
"""Compare the exact original LiteRT and exported ONNX tensors, without sensors.

Random tensors exercise translation, scale, handedness and unused-port claims;
they do not represent valid anatomy or establish visual gesture accuracy.
Optional real task-result landmark tensors add provider-preprocessing parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import time

from ai_edge_litert.interpreter import Interpreter
import numpy as np
import onnxruntime as ort

from export_gesture_classifier import BUNDLE_SHA256, LABELS


def preprocess(points, width, height):
    """Default image rotation=0 LandmarksToMatrix, matching pinned C++ order.

    Preserve upstream positive float-min bounding initialization, including its
    behavior for coordinates entirely on one side of the wrist. Inputs are
    source-image normalized XYZ; world XYZ and canonical ROI XYZ are different.
    """
    points = np.array(points, dtype=np.float32, copy=True)
    if points.shape != (21, 3) or not np.isfinite(points).all() or min(width, height) <= 0:
        raise ValueError('Expected 21 finite normalized image XYZ points and positive image size')
    points[:, 0] = (points[:, 0]-0.5)*(width/max(width, height))+0.5
    points[:, 1] = (points[:, 1]-0.5)*(height/max(width, height))+0.5
    points -= points[0].copy()
    lo = np.minimum(np.min(points[:, :2], axis=0), np.finfo(np.float32).max)
    hi = np.maximum(np.max(points[:, :2], axis=0), np.finfo(np.float32).tiny)
    scale = np.max(hi-lo)+np.float32(1e-5)
    return (points/scale).reshape(1, 21, 3)


def check(directory, output, cases=256, fixture=None):
    if output.exists():
        raise ValueError('Parity result already exists')
    manifest = json.loads((directory/'manifest.json').read_text())
    if manifest['source_bundle_sha256'] != BUNDLE_SHA256:
        raise ValueError('Unexpected source bundle')
    for name, digest in {**manifest['tflite_sha256'],
                          'gesture_classifier.onnx': manifest['onnx_sha256']}.items():
        if hashlib.sha256((directory/name).read_bytes()).hexdigest() != digest:
            raise ValueError('Model bytes changed: ' + name)
    embedder = Interpreter(model_path=str(directory/'gesture_embedder.tflite'), num_threads=1)
    classifier = Interpreter(model_path=str(directory/'canned_gesture_classifier.tflite'), num_threads=1)
    embedder.allocate_tensors()
    classifier.allocate_tensors()
    inputs = embedder.get_input_details()
    session_options = ort.SessionOptions()
    session_options.intra_op_num_threads = 1
    session_options.inter_op_num_threads = 1
    native = ort.InferenceSession(str(directory/'gesture_classifier.onnx'), session_options,
                                 providers=['CPUExecutionProvider'])
    rng = np.random.default_rng(0x15A10)
    examples = [dict(name='random-'+str(i), tensor=rng.normal(size=(1, 21, 3)).astype(np.float32))
                for i in range(cases)]
    for name, value in [('zero', 0.0), ('one', 1.0), ('negative', -1.0)]:
        examples.append(dict(name=name, tensor=np.full((1, 21, 3), value, dtype=np.float32)))
    fixture_bytes = None
    if fixture:
        fixture_bytes = fixture.read_bytes()
        data = json.loads(fixture_bytes)
        for item in data['hands']:
            examples.append(dict(name=item['name'], tensor=preprocess(
                item['landmarks'], item['width'], item['height']), task_category=item['category']))
    max_probability_error, max_embedding_error = 0.0, 0.0
    argmax_disagreements = 0
    max_unused_port_error = 0.0
    results, timings = [], []
    for item in examples:
        value = item['tensor']
        embedder.set_tensor(inputs[0]['index'], value)
        for inp in inputs[1:]:
            embedder.set_tensor(inp['index'], np.zeros(inp['shape'], dtype=np.float32))
        embedder.invoke()
        embedding = embedder.get_tensor(embedder.get_output_details()[0]['index'])
        # Repeat with radically different declared-but-unused ports. Pair this
        # empirical check with the exporter's actual graph dependency inspection.
        for inp in inputs[1:]:
            embedder.set_tensor(inp['index'], rng.normal(size=inp['shape']).astype(np.float32)*1000)
        embedder.invoke()
        alternate = embedder.get_tensor(embedder.get_output_details()[0]['index'])
        max_unused_port_error = max(max_unused_port_error, float(np.max(np.abs(alternate-embedding))))
        classifier.set_tensor(classifier.get_input_details()[0]['index'], embedding)
        classifier.invoke()
        original = classifier.get_tensor(classifier.get_output_details()[0]['index'])
        started = time.perf_counter()
        probabilities, converted_embedding = native.run(None, {'hand': value})
        timings.append((time.perf_counter()-started)*1000)
        if not np.isfinite(probabilities).all() or not np.isfinite(converted_embedding).all():
            raise ValueError('Nonfinite converted output')
        probability_error = float(np.max(np.abs(probabilities-original)))
        embedding_error = float(np.max(np.abs(converted_embedding-embedding)))
        max_probability_error = max(max_probability_error, probability_error)
        max_embedding_error = max(max_embedding_error, embedding_error)
        argmax_disagreements += int(original.argmax() != probabilities.argmax())
        np.testing.assert_allclose(probabilities, original, rtol=2e-4, atol=2e-5)
        np.testing.assert_allclose(converted_embedding, embedding, rtol=2e-4, atol=2e-5)
        if 'task_category' in item:
            results.append(dict(name=item['name'], task_category=item['task_category'],
                original_category=LABELS[int(original.argmax())],
                converted_category=LABELS[int(probabilities.argmax())],
                probability_error=probability_error, embedding_error=embedding_error,
                converted_probabilities=probabilities.reshape(-1).tolist()))
    if max_unused_port_error != 0.0 or argmax_disagreements:
        raise ValueError('Dependency or argmax parity failed')
    result = dict(schema='wigglecontrol.gesture-parity/v1', onnx_sha256=manifest['onnx_sha256'],
        seed=0x15A10, random_cases=cases, constant_cases=3, total_cases=len(examples),
        onnxruntime_version=ort.__version__,
        fixture_sha256=hashlib.sha256(fixture_bytes).hexdigest() if fixture_bytes else None,
        maximum_probability_absolute_error=max_probability_error,
        maximum_embedding_absolute_error=max_embedding_error,
        maximum_unused_input_absolute_error=max_unused_port_error,
        argmax_disagreements=argmax_disagreements,
        converted_cpu_model_median_ms=float(np.median(timings)), provider_examples=results,
        camera_opened=False, system_input=False, parity_verified=True,
        limits='Tensor equivalence and provider examples only, not held-out recognition, native landmark preprocessing, macro intent or receiving latency')
    output.write_text(json.dumps(result, indent=2)+'\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model_directory', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--fixture', type=Path)
    parser.add_argument('--cases', type=int, default=256)
    args = parser.parse_args()
    if not 32 <= args.cases <= 4096:
        parser.error('--cases must be32..4096')
    print(json.dumps(check(args.model_directory, args.output, args.cases, args.fixture)))


if __name__ == '__main__':
    main()
