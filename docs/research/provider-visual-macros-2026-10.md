# Raw-provider visual macro integration —2026-10-16

CompAII, continuing the camera-controller goal and issues
[#3](https://github.com/AlterMundi/WiggleControl/issues/3) and
[#7](https://github.com/AlterMundi/WiggleControl/issues/7).

The GPU worker's image Z was discarded at the raw provider boundary, while the
existing classifier needs image XYZ. The adapter also assigned the configured
presence floor0.5 to every hand; both classifier policy and controller enforce
at least0.8, so simply enabling classification could never produce usable macro
evidence. Neither handedness probability nor a fabricated higher hand score is
a valid replacement.

## Supported connection

The worker now retains normalized image Z times source width. Optional raw
`image_depths` distinguishes unavailable data from a genuinely zero-depth tensor.
No world coordinate substitutes for image depth. The classifier reuses existing
normalization, weights, ownership and macro arbitration. Missing depth yields
missing evidence; malformed depth rejects the observation. Native numeric depth,
pinch interpretation and default disabled behavior remain unchanged.

The explicit session gesture threshold now configures the SDK presence threshold
and is returned by `/provider/config`. A visual profile must select a floor at
least as high as its visual policy, which still requires>=0.8. A lower floor is
rejected before execution. This is a configured detection-score lower bound,
not a calibrated probability or a measured per-hand score. Raising it may remove
useful detections and requires matched coverage measurements before adoption.

The coordinate convention follows the official
[Web guide](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker/web_js).
The pinned upstream
[hand detector graph](https://github.com/google-ai-edge/mediapipe/blob/7dd8fe00c17e7732818ea1406c4fb64beb237e6c/mediapipe/tasks/cc/vision/hand_landmarker/hand_landmarks_detector_graph.cc)
thresholds hand presence before allowing landmark results. These sources support
coordinate/policy interpretation; they do not prove this installed bundle's
quality or receiving performance.

Enabled provider execution initializes the existing ORT runtime and small
classifier without another hand detector. Disabled profiles and configuration
preflight do not initialize either. Actual classification cost is reported and
included before the controller's live timestamp/freshness check. Feedback reuses
the existing pose, hold, blocking and local action receipts. No external action
adapter or physical acceptance is implied.

## Conditional retained-tensor execution

The source was built locally with `native-vision`. Four original Tasks1.1.1
image-landmark tensors from the previous classifier investigation were converted
to raw source-pixel XY/Z and passed through actual provider tracking, the shared
classifier and controller. Original reference/model/inputs and output hashes
are retained privately. Exported ONNX SHA256 remains
`a08e1d108011fb6596f8ff9d9a67d3e29c5ec1699431b9f7752e34f4796fef29`.

| Retained tutorial tensor | Category | Provider score | Evidence meets0.8 |
| --- | --- | --- | --- |
| Thumbs down | Thumb_Down |0.77306038 |No |
| Victory | Victory |0.90830380 |Yes |
| Thumbs up | Thumb_Up |0.73202395 |No |
| Pointing up | Pointing_Up |0.81976384 |Yes |

All four categories match the original full-task reference; maximum selected
score difference is1.79e-7. Actual VPS debug adapter/classifier calls with depth
take0.215–0.481ms in these eight probes, excluding model load and hand detection.
This is neither a Pi measurement nor sustained pipeline latency. Removing image
Z in the middle frame yields no pose/evidence and preserves the same hand ID;
restoring Z reproduces the reference prediction. No macro action fires: these
isolated samples provide neither a positive neutral sequence nor a timed hold.

This is conditional numerical parity, not a new image detector test. The old
producer's presence floor is not requalified by replaying its points with a0.8
semantic profile. The photographs are public tutorial examples with unknown
training overlap and individual image licenses; neither images nor raw landmarks
are committed or contributed to collective research.

The complete native-feature suite passes, including existing ownership, neutral,
pinch-priority, source-age, loss and input-release tests. New source tests cover
optional/invalid depth, real configured floor compatibility and disabled-runtime
independence. Python wire tests retain legacy packets and preserve complete depth
for two hands within the existing bounded reader. No receiving deployment,
camera acquisition, OS input or fresh human acceptance occurred in these checks.

## Next evidence required

Run actual SDK image inference at the selected presence threshold, rather than
relabelling old lower-floor points. Measure neutral/action recognition and useful
pinch coverage on the same continuous source, including uncertain/unknown poses,
input priority, loss and hand ownership. Compare classifier cost on the receiving
Pi. A successful timed local action must follow actual recognized neutral and
hold, never a synthetic named-pose event. Keep the prepared camera-only trial and
its pinned candidate/configuration intact until fresh human readiness. Practical
application delivery and seated comfort remain requirements of the full goal.
