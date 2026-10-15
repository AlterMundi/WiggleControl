#!/usr/bin/env python3
"""Convert the pinned MediaPipe canned classifier to the existing ONNX backend.

Offline model tooling only; no download, capture, device output or training.
This deliberately supports one hashed model bundle, not arbitrary TFLite graphs.
Python is used for the providers' FlatBuffer/ONNX model-format APIs. The receiving
controller continues to use Rust and its existing ONNX Runtime.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import zipfile

from mediapipe.tasks.metadata import schema_py_generated as schema
import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper


BUNDLE_SHA256 = '97952348cf6a6a4915c2ea1496b4b37ebabc50cbbf80571435643c455f2b0482'
BUNDLE_URL = ('https://storage.googleapis.com/mediapipe-models/gesture_recognizer/'
              'gesture_recognizer/float16/1/gesture_recognizer.task')
LABELS = ['None', 'Closed_Fist', 'Open_Palm', 'Pointing_Up',
          'Thumb_Down', 'Thumb_Up', 'Victory', 'ILoveYou']


def unpack(bundle):
    data = bundle.read_bytes()
    if hashlib.sha256(data).hexdigest() != BUNDLE_SHA256:
        raise ValueError('Expected the pinned public gesture_recognizer.task bundle')
    with zipfile.ZipFile(io.BytesIO(data)) as outer:
        with zipfile.ZipFile(io.BytesIO(outer.read('hand_gesture_recognizer.task'))) as inner:
            models = {name: inner.read(name) for name in
                      ('gesture_embedder.tflite', 'canned_gesture_classifier.tflite')}
    with zipfile.ZipFile(io.BytesIO(models['canned_gesture_classifier.tflite'])) as associated:
        if associated.read('labels.txt').decode().splitlines() != LABELS:
            raise ValueError('Unexpected classifier label order')
    return models


class Converter:
    def __init__(self):
        self.nodes = []
        self.constants = []

    def constant(self, name, value):
        self.constants.append(numpy_helper.from_array(np.asarray(value), name))
        return name

    def convert(self, data, prefix, input_aliases):
        model = schema.Model.GetRootAsModel(data, 0)
        if model.Version() != 3 or model.SubgraphsLength() != 1:
            raise ValueError('Expected a single TFLite v3 subgraph')
        graph = model.Subgraphs(0)
        names = {i: prefix + str(i) for i in range(graph.TensorsLength())}
        names.update(input_aliases)
        types = {schema.TensorType.FLOAT32: np.dtype('<f4'),
                 schema.TensorType.INT32: np.dtype('<i4')}
        values = {}
        for i in range(graph.TensorsLength()):
            tensor = graph.Tensors(i)
            buffer = model.Buffers(tensor.Buffer())
            if buffer.DataLength():
                array = np.frombuffer(buffer.DataAsNumpy().tobytes(), dtype=types[tensor.Type()])
                array = array.reshape(tuple(tensor.ShapeAsNumpy()))
                values[i] = array
                self.constant(names[i], array)
        operators = {v: k for k, v in vars(schema.BuiltinOperator).items() if isinstance(v, int)}
        used = set()
        inventory = []
        for i in range(graph.OperatorsLength()):
            op = graph.Operators(i)
            code = model.OperatorCodes(op.OpcodeIndex())
            kind = operators[code.BuiltinCode()]
            if code.Version() != 1:
                raise ValueError('Unexpected operator version')
            src_ids, dst_ids = list(op.InputsAsNumpy()), list(op.OutputsAsNumpy())
            used.update(src_ids)
            src, dst = [names[n] for n in src_ids], [names[n] for n in dst_ids]
            inventory.append(kind)
            tag = prefix + 'op' + str(i)

            def options(cls):
                result = cls()
                table = op.BuiltinOptions()
                result.Init(table.Bytes, table.Pos)
                return result

            def emit(kind, inputs, outputs, **attrs):
                self.nodes.append(helper.make_node(kind, inputs, outputs, **attrs))

            def activated(kind, inputs, opts, **attrs):
                activation = opts.FusedActivationFunction()
                if activation not in (0, 1):
                    raise ValueError('Only absent/RELU fused activations are qualified')
                intermediate = [tag + '_pre_activation'] if activation else dst
                emit(kind, inputs, intermediate, **attrs)
                if activation:
                    emit('Relu', intermediate, dst)

            if kind == 'GATHER':
                opt = options(schema.GatherOptions)
                if opt.BatchDims() != 0:
                    raise ValueError('Batched gather is unsupported')
                emit('Gather', src, dst, axis=opt.Axis())
            elif kind in ('MEAN', 'REDUCE_MAX', 'REDUCE_MIN'):
                opt = options(schema.ReducerOptions)
                emit({'MEAN': 'ReduceMean', 'REDUCE_MAX': 'ReduceMax',
                      'REDUCE_MIN': 'ReduceMin'}[kind], src[:1], dst,
                     axes=values[src_ids[1]].reshape(-1).tolist(), keepdims=int(opt.KeepDims()))
            elif kind == 'UNPACK':
                opt = options(schema.UnpackOptions)
                if opt.Num() != len(dst):
                    raise ValueError('Unexpected unpack arity')
                for index, output in enumerate(dst):
                    index_name = self.constant(tag + '_index' + str(index), np.int64(index))
                    emit('Gather', src + [index_name], [output], axis=opt.Axis())
            elif kind in ('ADD', 'MUL', 'SUB', 'DIV'):
                opt = options(getattr(schema, {'ADD': 'AddOptions', 'MUL': 'MulOptions',
                                              'SUB': 'SubOptions', 'DIV': 'DivOptions'}[kind]))
                activated({'ADD': 'Add', 'MUL': 'Mul', 'SUB': 'Sub', 'DIV': 'Div'}[kind], src, opt)
            elif kind == 'MAXIMUM':
                emit('Max', src, dst)
            elif kind == 'RESHAPE':
                shape = self.constant(tag + '_shape', values[src_ids[1]].astype(np.int64))
                emit('Reshape', [src[0], shape], dst)
            elif kind == 'STRIDED_SLICE':
                opt = options(schema.StridedSliceOptions)
                # This model has a masked full slice of a [1,1,1] tensor.
                # Reject any new slicing behavior instead of silently approximating it.
                if (opt.BeginMask(), opt.EndMask(), opt.NewAxisMask(),
                    opt.ShrinkAxisMask(), opt.EllipsisMask()) != (7, 7, 0, 0, 0):
                    raise ValueError('Unexpected slice masks')
                if (not np.array_equal(values[src_ids[1]], [0, 0, 0]) or
                        not np.array_equal(values[src_ids[3]], [1, 1, 1])):
                    raise ValueError('Unexpected full-slice bounds')
                emit('Identity', src[:1], dst)
            elif kind == 'FULLY_CONNECTED':
                opt = options(schema.FullyConnectedOptions)
                if opt.KeepNumDims() or opt.WeightsFormat() != 0 or len(src) != 3:
                    raise ValueError('Unexpected dense-layer layout')
                activated('Gemm', src, opt, transB=1)
            elif kind == 'SOFTMAX':
                if options(schema.SoftmaxOptions).Beta() != 1.0:
                    raise ValueError('Unexpected softmax beta')
                emit('Softmax', src, dst, axis=-1)
            else:
                raise ValueError('Unqualified TFLite operator: ' + kind)
        dependencies = [int(i) for i in graph.InputsAsNumpy() if i in used]
        return names[int(graph.Outputs(0))], dependencies, inventory


def export(bundle, output):
    if output.exists():
        raise ValueError('Output directory already exists; preserve previous evidence')
    models = unpack(bundle)
    converter = Converter()
    embedding, dependencies, embed_ops = converter.convert(models['gesture_embedder.tflite'], 'e', {0: 'hand'})
    # These two declared TFLite ports have no graph consumers in this exact model.
    # This is a pinned graph finding, not a general promise about MediaPipe models.
    if dependencies != [0]:
        raise ValueError('Pinned embedder unexpectedly consumes handedness/world inputs')
    scores, class_dependencies, class_ops = converter.convert(
        models['canned_gesture_classifier.tflite'], 'c', {0: embedding})
    if class_dependencies != [0]:
        raise ValueError('Unexpected classifier input dependencies')
    converter.nodes.append(helper.make_node('Identity', [scores], ['probabilities']))
    converter.nodes.append(helper.make_node('Identity', [embedding], ['embedding']))
    referenced = {name for node in converter.nodes for name in node.input}
    graph = helper.make_graph(converter.nodes, 'mediapipe-canned-hand-gesture-v1',
        [helper.make_tensor_value_info('hand', TensorProto.FLOAT, [1, 21, 3])],
        [helper.make_tensor_value_info('probabilities', TensorProto.FLOAT, [1, 8]),
         helper.make_tensor_value_info('embedding', TensorProto.FLOAT, [1, 128])],
        [tensor for tensor in converter.constants if tensor.name in referenced])
    model = helper.make_model(graph, producer_name='wigglecontrol-pinned-model-export',
                              opset_imports=[helper.make_opsetid('', 13)])
    model.ir_version = 10  # Supported by the selected ONNX Runtime 1.22.0.
    helper.set_model_props(model, {'source_bundle_sha256': BUNDLE_SHA256,
                                  'labels': json.dumps(LABELS),
                                  'input': 'MediaPipe LandmarksToMatrix preprocessed image XYZ, not raw ROI/world XY'})
    onnx.checker.check_model(model, full_check=True)
    output.mkdir(mode=0o700)
    onnx.save(model, output/'gesture_classifier.onnx')
    for name, data in models.items():
        (output/name).write_bytes(data)
    manifest = dict(schema='wigglecontrol.gesture-model-export/v1', source_url=BUNDLE_URL,
        source_bundle_sha256=BUNDLE_SHA256, onnx_version=onnx.__version__,
        onnx_sha256=hashlib.sha256((output/'gesture_classifier.onnx').read_bytes()).hexdigest(),
        inputs=[dict(name='hand', shape=[1, 21, 3], dtype='float32')], labels=LABELS,
        consumed_embedder_ports=dependencies, unused_embedder_ports=[1, 2],
        operators=dict(embedder=embed_ops, classifier=class_ops),
        tflite_sha256={name: hashlib.sha256(data).hexdigest() for name, data in models.items()},
        camera_opened=False, system_input=False, parity_verified=False,
        limits='Conversion/checker success alone proves neither numeric parity nor native preprocessing or gesture accuracy')
    (output/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    manifest = export(args.bundle, args.output)
    print(json.dumps({k: manifest[k] for k in ('onnx_sha256', 'consumed_embedder_ports', 'parity_verified')}))


if __name__ == '__main__':
    main()
