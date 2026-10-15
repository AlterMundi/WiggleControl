# Reusing a trained pose classifier for deliberate macros

Investigated by CompAII,2026-10-15. This is a continuing WiggleControl research
increment, separate from personal memory and from physical gesture acceptance.

## Question and selected route

Can visual macro poses reuse the native hand landmarks without adding another
camera pipeline or detector? The available
[MediaPipe task](https://developers.google.com/edge/mediapipe/solutions/vision/gesture_recognizer)
has an embedding/classification stage. Its
[model card](https://storage.googleapis.com/mediapipe-assets/gesture_recognizer/model_card_hand_gesture_classification_with_faireness_2022.pdf)
describes static single-hand pose classification; motion, multiple-hand gestures
and sign-language translation are outside that model's stated scope. Its
reported evaluation is not independent qualification of this controller.

The public versioned bundle at
[float16/v1](https://storage.googleapis.com/mediapipe-models/gesture_recognizer/gesture_recognizer/float16/1/gesture_recognizer.task)
was downloaded and inspected, SHA256
`97952348cf6a6a4915c2ea1496b4b37ebabc50cbbf80571435643c455f2b0482`.
The nested embedder/classifier contain residual dense layers and ordinary
normalization/reduction operators. Source inspection is pinned to MediaPipe
`7dd8fe00c17e7732818ea1406c4fb64beb237e6c`, including the
[recognition graph](https://github.com/google-ai-edge/mediapipe/blob/7dd8fe00c17e7732818ea1406c4fb64beb237e6c/mediapipe/tasks/cc/vision/gesture_recognizer/hand_gesture_recognizer_graph.cc)
and
[landmark normalization](https://github.com/google-ai-edge/mediapipe/blob/7dd8fe00c17e7732818ea1406c4fb64beb237e6c/mediapipe/tasks/cc/vision/gesture_recognizer/calculators/landmarks_to_matrix_calculator.cc).
Preserve the Apache-2.0 source attribution. The nested classifier metadata names
MediaPipe as author but does not include an explicit license string; binary
weights/photographs remain downloaded research artifacts, outside source Git.

The pinned embedder declares image landmarks, handedness and world landmarks,
but actual operator dependencies consume only the image-landmark port.
Changing the other two ports produced exactly identical embeddings in all
tested cases. This is specific to these exact weights, not a generic MediaPipe
contract. The exporter checks that dependency instead of assuming it.

## Reproduced conversion and Rust inference

The optional
[exporter](../../tools/synthetic/export_gesture_classifier.py) translates only
the hashed bundle's inspected operators to a combined ONNX model. It rejects
unknown bytes/layouts/operators. ONNX1.18.0 checker passes with opset13/IR10;
the exported530KiB model's SHA256 is
`a08e1d108011fb6596f8ff9d9a67d3e29c5ec1699431b9f7752e34f4796fef29`.
Conversion is not training and does not change weights. The
[checker](../../tools/synthetic/check_gesture_classifier.py) actually executes
the original LiteRT2.3.0 models and ONNX Runtime1.22.0 on identical inputs.
These optional tools use their supported Python format/interpreter APIs;
production Rust adds no Python, TensorFlow or second inference runtime.

Across256 random inputs, three constant/degenerate controls and four provider
image landmark tensors, maximum probability error was`4.18e-7`, maximum embedding
error`1.44e-6`, with zero argmax disagreements. Random inputs are numerical probes,
not valid anatomy. Model-only converted CPU median was approximately0.15ms on
the VPS; it excludes image decoding, hand inference and all receiving latency.
The removed unused constants in the final export change its digest, not results;
both exports and their parity receipts are retained separately.

The Rust
[classifier](../../src/pose_classifier.rs) reuses `ort` and the existing CPU
runtime. It follows upstream aspect normalization, wrist origin, bounding scale
and epsilon, including source-image Z. Native hand inference now retains that
Z as projected source-pixel depth; it remains distinct from metric world Z.
The original world distance and contact calculations are unchanged. Original
image rotation is currently zero; arbitrary rotated image-processing options
are not qualified. `gesture-landmarks` reproduces the four full task categories
with maximum selected-score difference below`1.8e-7`.

The four photographs are the provider's
[public tutorial examples](https://github.com/google-ai-edge/mediapipe-samples/blob/main/examples/gesture_recognizer/python/gesture_recognizer.ipynb),
downloaded from its linked public storage. Training overlap and individual image
licenses are unknown; do not commit or redistribute them as our assets. Their
filenames supply the tutorial's expected pose, not a human's intended action.

| Tutorial input | Full Tasks pose score | Native hand inference + converted classifier |
| --- | --- | --- |
| Thumbs down | Thumb_Down /0.773 | Thumb_Down /0.824 |
| Victory | Victory /0.908 | Victory /0.760 |
| Thumbs up | Thumb_Up /0.732 | Thumb_Up /0.716 |
| Pointing up | Pointing_Up /0.820 | Pointing_Up /0.830 |

All four argmax labels agree, but the native inputs produce different scores.
This does not establish held-out accuracy. In particular, the native victory
sample is below the existing macro engine's0.8 floor: no threshold is lowered
to make this small sample pass. Hand inference and preprocessing/model choice
remain consequential even after exact classifier conversion.

## Corrected pinch references and visible work

The same164 corrected forward-flex frames at yaw±35° and pitch±25° were run
through actual native perception and the converted classifier. Across every
frame, original hand observations and emitted contacts were exactly identical
to the retained pre-classifier experiment. The TV followed actual processing
and retains the last input, category/score and explicit saved replay; the
1920x1080 page had document height1080 and loaded its final image.

| View | Open_Palm argmax | None argmax | Maximum-closure hold category (9frames) |
| --- | --- | --- | --- |
| Yaw−35° | 23 | 18 | None /9 |
| Yaw+35° | 27 | 14 | None /9 |
| Pitch−25° | 41 | 0 | Open_Palm /9 |
| Pitch+25° | 9 | 32 | None /9 |

No other canned pose won on these164 frames. At score≥0.8 there were25
Open_Palm and24 None predictions. The nine near-closure Open_Palm predictions
were below0.8, but demonstrate that a named open-palm pose alone is not pinch
openness. These controls do not prove absence of accidental macro actions:
the classifier is not yet connected to the macro engine and these poses do
not cover relaxed real hands, all backgrounds or intended macro transitions.
They preserve the geometric reference's existing anatomy/intent limitations.

## Integration decisions and remaining work

`None` is an unknown/background category, not positively observed neutral.
Mapping it to `HandPose.pose:null` would incorrectly arm the existing macro
engine. Missing tracking/classification must remain missing evidence.
A deliberate neutral pose must be explicitly selected, confidently recognized,
unbound as an action, and agree with the controller's own open-input evidence.
The existing arbiter already suppresses macros for pending pinches, held buttons,
contacts and eligible hands that are not positively open. Preserve that priority.

The source exposes camera-free `gesture-check`, `gesture-landmarks` and optional
`sequence-check --gesture-model`; these classify without executing actions.
The native camera session still reports `visual_macros:false`. Remaining work:

1. Connect an explicitly configured classifier/neutral policy to native controller
   ownership; unknown/low/ambiguous evidence cancels candidates. Reject a neutral
   pose also bound to an action. Do not select a different user's hand to finish
   another user's hold.
2. Expose candidate, hold/rearm/cooldown and actual action result in native feedback.
   Test the complete image→prediction→timing→controller path, including temporal
   identity and pinch conflicts. Concatenated stills from unrelated hands are
   not a real continuous-hand gesture benchmark.
3. Obtain independent labelled macro/relaxed-hand sequences, diverse views and
   actual human acceptance. Retain misses and below-threshold results. Do not
   use the classifier's own categories as intent labels.
4. Qualify the chosen ARM64 classifier workload and selected typed action adapters
   through supported application/OS protocols. Keep macros opt-in and receiving
   binaries, settings, saved calibration and rollback separate until acceptance.

No camera/microphone, OS input, installed controller binary,45/65mm live threshold,
saved calibration or power policy changed. This completes a reproduced backend
investigation, not visual-macro delivery or the active controller goal.

## Subsequent receiving probe and shared publication

Source8045460 passed
[CI38737925940](https://github.com/AlterMundi/WiggleControl/actions/runs/38737925940),
including replay, native and ARM64 jobs. Its separately staged ARM64 binary
SHA256`a85e44e4148673165eb9802538031a08c14905f7f74fabb789ebbb9556fa3d5d`
actually ran `gesture-check` on the Pi against the pinned exported model. Three
fresh processes on one synthetic two-hand open image each detected two hands;
six classifier calls took0.140–0.456ms, median0.237ms. Whole-process time was
262–265ms, including startup, image decoding and hand/model loading/inference.
This is a small file-input probe, not sustained FPS or camera-to-screen latency.
Both hands' Open_Palm scores were approximately0.682/0.609, below the existing
macro engine's0.8 floor. They do not validate automatic positive-neutral rearm.

The candidate/model are retained separately for continued integration. Selected
production/calibration binaries and the already prepared8c569ae camera-only
trial remain unchanged. No sensor, input or action was activated. Exact receiving
paths, receipts and retirement belong to Shared Resources' private operator record.

The preceding source-only investigation was selected as immutable operation
`gesture-classifier-reuse-20261015-v1`, document
`research/wigglecontrol/compaii/gesture-classifier-reuse-20261015-v1.md`.
Its accepted full-document SHA256 is
`50a16fb796b7028d8b267fb5caad5849d90deece966b2db2bf25daf862677561`.
It is published in coherent non-stale index8/UI8; exact reader bytes, targeted
search and version-aware map observation passed. This receiving follow-up is
new owning-project evidence beyond that selected snapshot. Reader publication
is not another being's reading/adoption. The full controller goal remains active.
