# Fixed native tensors and mixed-model tracking

CompAII,2026-10-15. Continues [implemented fusion](native-pinch-fusion-2026-10.md),
[#7](https://github.com/AlterMundi/WiggleControl/issues/7) and ownership
[#6](https://github.com/AlterMundi/WiggleControl/issues/6).
These are synthetic, camera/device-free experiments. The optional new producer
is implemented in Rust; selected receiving runtime/calibration remain unchanged.

## Findings

Changing the model and its crop together obscures their separate effects. The
new `crop-check` command exports the actual native224×224 RGB float tensor and
fixed-crop native result.286 retained primary/proximal crops across six actual10Hz
sequences reproduce their original native world gaps **exactly**. Pinned Tasks
landmark weights then run on those same float values, without a new detector,
resizer, normalization, PNG roundtrip or tracker.

A conditional native-primary/Tasks-proximal combination covers17/18 near-hold
frames, versus12/18 for shared-model native dual crop. A sole Tasks metric on
native primary crops covers only4/18. Keeping the controller's default0.7 gate
with the alternate model's actual presence scores loses the apparent gain;
explicit0.5 admits it. Alternate presence ranges down to0.5763 on this set;
confidence scores are not calibrated across models.0.5 is the already selected
provider threshold in the earlier pinned Tasks comparison, not a fitted distance
threshold. The original native image producer still rejects presence below0.8.
Conditional metric-only replay, own-score replay and a full producer remain
separate evidence.

## One inference runtime

The Tasks bundle SHA256 is
`fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1`.
Its `hand_landmarks_detector.tflite` member has SHA256
`6acda74af3fbf40e68265c20c7394b2bad81a16a481dcd79ad7a081887c3d6b9`.
The observed conversion uses TensorFlow CPU2.15.1,tf2onnx1.16.1,ONNX1.17.0,
NumPy1.26.4,protobuf3.20.3 and opset17. It produces an approximately11MB model,
SHA256`2672e7204afb4b965c391a9f77f910d392d5c0757c1af1ac7607b7a103a9ef5c`.
Conversion runs once on the VPS. No Python/TensorFlow/LiteRT runtime is added to
the controller or installed on the Pi.

All286 exact tensors were subsequently run through both LiteRT2.3.0 CPU/XNNPACK
and the actual Rust ONNX Runtime1.22.0 probe, checking all four complete outputs.
Declared tolerances precede this validation. Observed maximum absolute differences:

| Output | Maximum difference |
| --- | --- |
| Image landmark components, in crop pixels |0.000373841 |
| Presence score |0.000002862 |
| Handedness score |0.000003756 |
| World landmark components, meters |0.000000394881 |

These are cross-runtime numerical differences, **not physical hand accuracy**.
The [model manifest](../../tools/synthetic/tasks-onnx-model.json) preserves the
asset/conversion chain. The tensor/output convention follows
[the pinned upstream graph](https://github.com/google-ai-edge/mediapipe/blob/998a5d77bb3d0e06211b6e9cc1fd6094e6051bfb/mediapipe/tasks/cc/vision/hand_landmarker/hand_landmarks_detector_graph.cc).
The existing [tf2onnx converter](https://github.com/onnx/tensorflow-onnx/tree/v1.16.1)
provides TFLite conversion; no custom converter or new Rust dependency was built.

## Actual native producer

An explicit experimental `secondary_model` loads one additional ONNX Session
through the existing runtime. Primary tracking/cursor/quality stay authoritative.
The alternate model runs its own initial inference and subsequent proximal-crop
feedback. It contributes its own presence, eligibility, world gap, pixel onset
cue and distinct estimator ID. It never inherits the primary model's initial
metric as if it had executed. Primary session/diagnostics are restored even if
secondary inference fails. Existing same-hand/frame association, independent
open/confirmation, ambiguity, loss, Stop and stale handling remain in force.

The factory is shared by pointer/touch/controller and image-sequence source paths.
Default remains the existing model/policy; explicit model selection requires
`dual_crop:true`,3D pinch, an absolute model path, distinct source ID and compatible
explicit gesture confidence. Recording/feedback/replay metadata retain policy.
A new preflight regression verifies old defaults and rejects unusable combinations
without opening resources. Full native-feature tests and clippy pass locally
with Rust1.95.0; maintained CI targets Rust1.85.1 separately.

All1042 image inputs were processed by the actual new native controller:
655 broad20Hz inputs,143 fresh10Hz inputs and244 paired inputs. Primary hand
observations/ambiguity remain exact against retained native baselines.
Independent skin labels are matched against the complete retained marker/camera
trajectory with zero marker error; one original render manifest predates its
skin metadata, so its already measured matching baseline labels are reused.
No prediction supplies its own geometry truth.

| Source policy |20Hz near contact frames | Actual10Hz hard-scene near frames |
| --- | --- | --- |
| Retained primary with0.38 cue |107/117 |9/18 |
| Shared-model native dual crop |115/117 |12/18 |
| New native mixed models, explicit0.5 |116/117 |17/18 |

Every positive20Hz trajectory has one contiguous contact interval. The adverse
new10Hz view improves0/5→4/5 and3/4→4/4; negative pitch retains5/5 and4/4. Both V
negative trajectories, their10Hz phases and all fully open holds have zero
contacts. There are no macro action frames in these1042 inputs; this does not
establish useful macro recognition. These repeated synthetic frames are not
independent trials, diverse anatomy, real intent or a real-world accuracy rate.
Opening transitions still contain held contacts above50mm selected skin separation.

Opposite/same-side separated pairs each retain61/61 dual detections, two stable
tracks and41/41 expected stable-hold contact counts. Both crossings still have
0/13 dual near holds. Same-side crossing additionally produces five contact
frames without qualified spatial association, versus none for the prior shared-
model run. They are **unscored ownership**, not proved correct or wrong owners;
this is an adoption blocker. An empty observed owner-transfer list cannot hide
those unknown intervals. At most two contacts are emitted.

On the VPS, the broad source processing median is51.687ms,p95103.369ms;
paired medians range82.3–91.8ms. These include decode/classifier work under actual
shared-host conditions, not matched model-only rankings or Pi performance.
The TV shows actual stages, both gap estimates and explicitly retained replay;
its owned1920×1080 page was checked with loaded imagery and no whole-page scroll.
No camera, microphone, system input or external action was activated.

## Reproduce and next qualification

Use an experimental copy of the existing session, retaining the calibration:

```json
{
  "pinch_3d": true,
  "dual_crop": true,
  "gesture_min_confidence": 0.5,
  "pinch_onset_max_screen_ratio": 0.38,
  "secondary_model": {
    "path": "/absolute/experimental/converted.onnx",
    "min_presence": 0.5,
    "source_id": 3
  }
}
```

The separate controller settings must also explicitly select
`gestures.min_confidence:0.5`; existing `config-check-controller` checks agreement.
This validates configuration, not model availability or physical acceptance.
Use camera-free `controller-sequence SEQUENCE SESSION SETTINGS` for actual source
qualification. Do not replace selected receiving binaries/config/profile from
this table.

`crop-check IMAGE SESSION NEW_DIRECTORY [ROI.json]` writes the exact little-endian
float32 tensor and native fixed-ROI result. The optional ROI is
`[center_x_px,center_y_px,side_px,angle_rad]`. `compare_crop_models.py MODEL.tflite
CROP_DIRECTORY... --expected-sha256 SHA --output NEW_REPORT` uses the pinned
`litert-requirements.lock` environment. The extended Rust
`wigglecontrol-ort-probe MODEL.onnx 224 RUNS INPUT.f32 NEW_REPORT.json` consumes the
same tensor; set the existing pinned `ORT_DYLIB_PATH`. 
Extract the manifest-qualified TFLite member from the pinned Tasks bundle, then:

```sh
TF_NUM_INTEROP_THREADS=1 TF_NUM_INTRAOP_THREADS=1 OMP_NUM_THREADS=1 uv run --no-project --python 3.11   --with-requirements tools/synthetic/tflite-conversion-requirements.lock   python -m tf2onnx.convert --tflite HAND_LANDMARKS.tflite   --output NEW_MODEL.onnx --opset 17
```

Model binaries and raw experiment evidence remain outside Git. Preserve their
source/model terms; the independently maintained Rust runtime is unchanged.
Receiving ARM64 image-only latency/thermal qualification is next. Broader
conflicting poses/backgrounds, release behavior, occlusion ownership, macro
recognition and fresh human/application acceptance remain open. The original
pending physical trial and the full modes/macros/participant roadmap remain intact.
