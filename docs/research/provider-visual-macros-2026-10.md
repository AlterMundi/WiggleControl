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

## Actual receiving comparison and rejected profile

Source[95633d7](https://github.com/AlterMundi/WiggleControl/commit/95633d7eafe2ad98477337464d7cdd4690c154f4)
passes[CI38842529801](https://github.com/AlterMundi/WiggleControl/actions/runs/38842529801)
for replay, native vision and ARM64. Its separately staged manual ARM64 binary
SHA256 is`cb4192c4c3782c23aaffe647fdd21bd4ffecec80a9d913b132d7a61aad86ce02`.
The existing receiving ORT1.22.0 and exact previously staged classifier are reused.
The prepared physical trial's original candidate/session/settings hashes remain
unchanged. No everyday runtime, power, calibration, camera or input is changed.

Three sequential runs process the same189 publications from a6.267s repeated
synthetic JPEG sequence on Pi5 at1.5GHz, with the requested fan-off setting.
Each uses one-hand GPU inference and the existing45/65mm contact interpretation;
only the stated presence floor and visual-classifier enablement differ. Each
source publication has a fresh host timestamp. The independent source geometry
and byte hashes are retained. Repeated images are not independent human trials.

| SDK presence floor / classifier | Accepted observations; no hand | Hz | Publication→meaning median /p95 | Matching near-hold wall time |
| --- | --- | --- | --- | --- |
|0.5 /disabled |97;0 |15.27 |70.875 /96.149ms |1035.213 of1499.029ms |
|0.8 /enabled |83;18 |13.06 |86.279 /123.626ms |0 of1501.255ms |
|0.8 /disabled control |84;18 |13.26 |84.783 /123.812ms |0 of1500.009ms |

The same high-floor failure remains when classification is disabled. This
supports rejecting the raised SDK floor as a general controller profile on this
control; classification alone is not the explanation for its lost useful hold.
The short sequential runs do not isolate a universal latency penalty or provide
a randomized sustained performance comparison. All cases retain zero open-hold
contact time. Source coalescing and actual meaning delay are included in wall-time
scoring, not only accepted-frame counts.

The enabled run actually classifies65 detected hands:52 Open_Palm and13 None.
Winning scores span0.5413–0.7333; none reaches0.8. There is no positive neutral
rearm or macro action. Existing thresholds are not lowered to make this synthetic
control pass. Classifier cost among actual predictions is0.423ms median,
0.662ms maximum, excluding hand detection/model load. Thus its measured local
cost is small, while recognition/quality evidence is still insufficient.

All three native units end successfully, neutral, with worker-close and actual
terminal-view acknowledgments. The terminal pages fit1920×1080 without whole-page
scrolling, owned workers are gone and the original workbench is restored. The
initial launcher had a Python string-escaping syntax error before remote mutation;
its output is retained. The corrected launcher validates generated Python before
execution; the first observer is reused by navigating the same owned target once
the actual server is ready. No capture/input or model restart is hidden as a retry.

The manual slot is usable for explicit offline qualification, not accepted daily
control. Keep it isolated from the pending physical-trial candidate; retire only
that experiment slot/configs after supersession, preserving receipts and the
shared classifier/runtime used elsewhere. Exact paths and commands remain in
the receiving private resource record. No startup service is installed.

## Next evidence required

The high-floor shortcut is rejected on this control. Investigate an adapter
that can retain actual per-hand presence/quality evidence without dropping useful
contact tracking, or another explicitly qualified perception route. That is a
next investigation, not an implemented capability. Do not fabricate confidence,
remove existing evidence gates or silently lower the classifier threshold.
Qualify neutral/action recognition on continuous source images, including unknown
poses, input priority, loss and ownership. A successful timed local action must
follow actual recognized neutral and hold, never a synthetic named-pose event. Keep the prepared camera-only trial and
its pinned candidate/configuration intact until fresh human readiness. Practical
application delivery and seated comfort remain requirements of the full goal.
