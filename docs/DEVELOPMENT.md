# WiggleControl development plan

## Intended result

A manually enabled Rust input device translates camera-observed hands into
system pointer/button/scroll events, followed by direct multitouch contacts.
The runtime is local, lightweight and usable without a daimon/model connection.
Existing applications do not integrate a new SDK. The initial target is Linux
ARM64 on Pi 5 with labwc/Wayland; other platforms require their own adapters.

[Research and evidence](research/state-of-the-art.md) explain these decisions.
This is the selected development direction; all milestone acceptance below is
pending. No implementation or device benchmark is claimed by publishing the plan.

## Proposed component boundaries

| Component | Owns | Does not assume |
| --- | --- | --- |
| Perception backend | Camera lifecycle, model preprocessing/inference/tracking and timestamped points/confidence | That detector array order is stable hand identity |
| Rust gesture engine | Engagement, smoothing, calibration, pinch transitions, drag, scroll and tracking loss | Screen/application semantics or an inference-runtime dependency |
| Linux input adapter | Device creation/classification, event frames, held input, timeout/release and destruction | That a mouse is an absolute touchscreen or that every app supports gestures |
| Local controls/status | Manual session start/stop, actual camera/device/gesture state | Automatic model activation or duplication of the integrators panel |
| Qualification/replay | Deterministic fixtures, measured runtime and human acceptance | That a unit test proves camera accuracy |

Proposed flow: camera → hand observations → Rust gesture state → Linux input
adapter → libinput/labwc → ordinary application. A typed local observation seam
lets replay and alternative native inference consume the same gesture engine.
Finalize and version its contract in milestone 1; it is not frozen by this prose.

## Milestone 1 — Reproducible observations and gesture behavior

Deliver the Rust project, explicit versioned observation/event contract and
fictional replay fixtures. Include monotonic source time, normalized coordinates,
confidence, stable hand association and explicit loss/staleness. Specify the
coordinate space, mirrored display convention and calibration assumptions.

Implement engagement separately from detection; smooth pointer input with a
measured noise/lag tradeoff. Normalize pinch distance by palm scale, use distinct
close/open thresholds and prevent fists/occlusion from becoming clicks. Start
with one selected hand. Emit pointer intent, button down/up, drag and loss;
choose scroll mapping explicitly before adding it.

Acceptance: replay tests cover normal pinch, drag, jitter, malformed/out-of-order
input, hand replacement, dropout and reacquisition. Held state always returns
neutral on stop/loss/stale input; reacquisition never invents a click. Queues and
message sizes are bounded. Replay requires neither camera nor root.

## Milestone 2 — Linux virtual mouse on the real desktop

Use Rust evdev/uinput rather than implementing a parallel ioctl library. Inspect
actual kernel/module and libinput readiness; a uinput node alone is insufficient.
Select a narrowly scoped device-writer grant or helper using existing receiving
administration, preserving access and avoiding unrestricted root inference.

Qualify relative pointer, button and wheel event framing. Compare absolute-target
mapping separately; acceleration and current cursor location must not be guessed.
Choose an explicit comfortable workspace/recentering behavior. Keep physical
mouse and emergency/manual stop functional.

Acceptance: libinput recognizes the virtual mouse; deterministic movement and
click/drag/scroll reach a scratch desktop task, VLC and Chromium. Verify Wayland
and XWayland paths separately, fullscreen focus, button release on disconnect,
writer/process death and removal of only this virtual device. No live camera yet.
Record reversible device permissions/setup and actual receiving evidence.

## Milestone 3 — Native perception selection and comparative measurements

Reuse existing trained palm/hand models, not a new training program. Compare the
proven MediaPipe browser baseline with exact ONNX models through ort/ONNX Runtime
and a compatible tract candidate. OpenCV Zoo preprocessing is inspectable;
palm anchors/suppression, rotated crops and output transforms must match the
chosen models. Reuse temporal hand tracking so palm detection is not repeated
unnecessarily. Native FFI is acceptable if its advantage is measured.

Inspect model/operator availability, compiler requirements and packaging before
selecting releases. Pin hashes/licenses. Do not transplant a GPU dependency
from Legion or assume CUDA on the Pi. Evaluate one hand on CPU first; use an
available accelerator only with verified target support and measured benefit.

Acceptance: compare output correctness on the same permitted source inputs;
record detection versus tracked-frame cost and complete pipeline resources.
Choose the actual native backend with evidence or retain an explicit compatible
fallback. Browser inference may be a comparison/transition backend, not the
final architecture implied by a Rust binary. Camera session remains finite and
explicitly activated; no recording or upload is required for normal operation.

## Milestone 4 — Camera-driven mouse acceptance

Join perception, gestures and device output on cuevanico. Integrate manual status
with the existing panel through its supported owning interface; avoid a second
sensor observer or opaque capture. Local device operation must survive VPS loss.

Use the [research acceptance experiment](research/state-of-the-art.md#acceptance-experiment):
latency p50/p95, whole-pipeline CPU/RSS, thermal state, fresh FPS, dropped frames,
jitter, repeated clicks/drags, relaxed-hand false actions, seated comfort and
hand-loss/stop release. Compare identical camera settings and scenes against the
existing tracker. Targets are proposed criteria, not guarantees.

Acceptance: the human can select desktop items and operate VLC/Chromium without
application changes; indicators agree with observed use. Failure/reconnection
returns neutral, no permanently pressed button. Reversible install/start/status/
stop/rollback instructions and runtime versions are published. No boot capture.

## Milestone 5 — Direct multitouch

Create a separately classified absolute/direct-touch device with type-B slots,
stable tracking IDs and explicit display mapping. Define how hand gestures create,
move and end contacts; two visible fingertips do not imply active touches.
Separate touchscreen mode from pointer mode and avoid duplicate click streams.

Acceptance: one and two contacts reach a known touch-aware application, with
correct ownership after hand crossing, occlusion and loss. Demonstrate supported
zoom/rotation only where the client interprets touch. Record mouse-only fallback
and labwc mouse-emulation choices, without claiming universal gestures.

## Milestone 6 — Delivery and shared maintenance

Package the qualified binaries/models with explicit receiving configuration,
versioned contracts, bounded retention/resource use, backup/rollback and device
permission removal. No startup camera or model attention by default. Document
coexistence with another camera application and coordinated shared-device use.

Update Shared Resources with actual capability, operator/grant, source version,
verification date and limits. Portable guidance follows the maintained Skills
workflow once qualified; a repository publication is not receiving adoption.

## Gates and open decisions

- Choose cursor anchor, relative versus absolute targeting and scroll gestures
  from real-user evidence; do not hide hand fatigue in a sensitivity default.
- Select compatible model/runtime/toolchain pins after correctness and performance
  qualification; preserve independent existing deployments.
- Reuse selected HarMoCAP timing/smoothing/validity/replay ideas, not its full-body
  pipeline. Preserve third-party licenses when deriving actual source.
- Camera ownership and desktop tests must be coordinated with Ani/Oliva and
  Nicolás; the topic's planning request does not authorize unrelated capture.
- Each milestone closes with code/test evidence plus its distinct real-device
  and human acceptance. No fleet rollout, new being or Matrix scope is implied.
