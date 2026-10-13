# WiggleControl: camera gestures as system input

Research date: 2026-10-13. Prepared initially under [Integrators #3](https://github.com/AlterMundi/daimon-integrators/issues/3); implementation now belongs to AlterMundi/WiggleControl.

Provenance: migrated from daimon-integrators commit `208a698`,
`docs/wigglecontrol-research.md`. Original remains as dated research history;
this repository owns subsequent research and development decisions.

## Decision

Build a Rust gesture engine and Linux virtual-input adapter around an existing
hand-landmark model. Preserve the proven browser tracker as a comparison source,
then qualify native inference before selecting its runtime. Do not rewrite the
whole interface or train a new hand model for the first increment.

The target is an actual system input device: existing applications consume mouse
or touch events without a WiggleControl SDK. A browser-only board is not the final
product. Rust is the preferred implementation language; a proven native inference
library behind Rust bindings is a justified option. Lower consumption remains a
measurement, not a consequence promised by the language.

This recommendation is an engineering inference from the sources below. No new
camera session, input injection, device grant or implementation was performed
during this research.

## Starting evidence

The selected host is a Pi 5 with C920e, 1080p60 HDMI, labwc/Wayland and an existing
Rust local activity panel. Live read-only inspection found kernel
6.18.50+rpt-rpi-2712 and labwc 0.20.2 / wlroots 0.20.2. The uinput device node is
present but root-only; module registration/device creation was not qualified.
Do not mistake a node's presence for a working input adapter.

Ani/Oliva's private ProyectoRPi source at `644d0ea`, documented at `699c9e7`,
uses Python for its localhost adapter, JavaScript for UI/gestures and MediaPipe
Tasks Vision 0.10.14 for browser-local inference. Its selected Pi receipt reports
one physical pinch-tap and matching styled-card feedback. Repeated accuracy,
distance, sustained FPS and false activations remain unmeasured there. The source
and selected receipts remain in their private owning repository.

## Existing implementations

| Candidate | Evidence and useful reuse | Fit for cuevanico |
| --- | --- | --- |
| [Barehands](https://github.com/jaredrhod/barehands), through ProyectoRPi | Existing hand-controlled browser interface; selected Pi passage already documented by Ani/Oliva. Reuse landmark conventions, gesture edge cases and replay examples. | Best current comparison baseline; not an OS input device yet. Preserve its AGPL provenance when deriving code. |
| [AirTouch / Neural Lab](https://neural-lab.com/gesture-control) | Vendor describes camera-based OS mouse, keyboard and touch control, including Linux. | Relevant product precedent and naming collision. Performance is vendor-reported; ARM64/Pi compatibility and suitability were not established. Not an inspected open Rust implementation. |
| [Ultraleap TouchFree](https://github.com/ultraleap/TouchFree) | Existing tracking-to-2D-input system; interaction documentation separates cursor, click, scroll and drag. | Useful interaction reference; the documented stack relies on Ultraleap tracking hardware and a Windows service, not our RGB C920/Linux deployment. |
| [Pawvis](https://github.com/alexandriax/pawvis) | Real OS cursor/buttons/scroll on macOS; explicitly distinguishes tracking from cursor engagement and handles parked input. | Useful engagement and gesture-state precedent. Uses Apple Vision, so not a deployable Pi backend. |
| [Air Paint](https://github.com/RefinedDev/air_paint) | Small Rust example combining OpenCV camera capture and ort inference for a YOLO hand-pose model. | Demonstrates native Rust plumbing; drawing inside its application is not universal OS input. Pi performance, model behavior and reusable API remain unqualified. |

No reviewed candidate establishes a ready, lightweight Rust RGB-camera mouse and
multitouch device with measured acceptance on this Pi. That is a bounded research
finding, not proof that none exists.

## Perception engines

[MediaPipe Hand Landmarker](https://developers.google.com/edge/mediapipe/solutions/vision/hand_landmarker)
uses palm detection followed by a rotated/cropped hand landmark model, returning
21 points. Video tracking avoids rerunning the palm detector while tracking holds.
Its model inputs are 192×192 and 224×224; full-resolution camera images are not
fed directly into both models. Published Pixel 6 timings are not Pi benchmarks.

| Route | Strength | Qualification needed |
| --- | --- | --- |
| Existing MediaPipe browser tracker → Rust gestures/input | Reuses the actual Pi passage and existing model pipeline. Quick way to isolate input behavior. | Measure the full browser process tree; export landmarks with timestamps and confidence. This is a transitional comparison backend, not the native final requirement. |
| [OpenCV Zoo palm + hand ONNX](https://github.com/opencv/opencv_zoo/tree/main/models/handpose_estimation_mediapipe) → Rust `ort` | Published models and inspectable preprocessing provide a concrete native path. | Port anchors, suppression, crop rotation and coordinate transforms correctly; retain temporal tracking rather than full redetection every frame. Native-library packaging and ARM64 operators need testing. |
| [tract](https://github.com/sonos/tract) | Rust engine with ARM CPU support; attractive small-runtime direction. | Load the exact selected model and compare outputs first. General ONNX support does not prove these operators/models work or meet latency targets. |
| [WasmEdge mediapipe-rs](https://github.com/WasmEdge/mediapipe-rs) | Existing Rust hand-landmark task code and model handling. | Its inspected package targets WASI-NN/WasmEdge, not a drop-in native Linux library. Inspected detection calls the palm detector; equivalent temporal tracking was not established. Extra runtime cost must justify itself. |
| [HarMoCAP-derived pipeline](https://github.com/AlterMundi/HarMoCAP/blob/main/docs/FEATURES.md) | Our existing smoothing, timestamps, state validity, identity and replay contracts are useful engineering assets. | Current body-pose pipeline is not evidence of fingertip/pinch detection. Extract selected methods; do not bring its whole body/multitude stack into a hand mouse. |

The [OpenCV benchmark](https://github.com/opencv/opencv_zoo/blob/main/benchmark/README.md)
lists Pi 4B mean inference times around 42.58 ms for hand pose and 97.05 ms for
palm detection for the cited floating models. These are separate model runs,
not capture-to-cursor latency and not Pi 5 results. They motivate temporal reuse,
not a prediction of our FPS. The hand-model README warns that int8 quantization
can degrade accuracy significantly; do not assume quantization improves the
usable interaction.

Source snapshots inspected: OpenCV Zoo `47534e27`; ort `1ee1f842`; tract
`2275e7c9`; mediapipe-rs `99512024`. Their inspected current compiler requirements
are ort 1.92, tract 1.91 and mediapipe-rs 1.89, above our component's Rust 1.85
baseline. Select compatible pinned releases or justify an isolated toolchain
change; do not upgrade production dependencies on the strength of a search result.
[ort is primarily an ONNX Runtime wrapper](https://github.com/pykeio/ort), so Rust
does not mean its inference implementation is entirely Rust.

## System input: mouse, touchpad and touchscreen differ

The [Linux uinput API](https://kernel.org/doc/html/latest/input/uinput.html)
creates virtual input devices from userspace. Prefer it for the Linux device
contract, using the existing [Rust evdev crate](https://docs.rs/evdev/latest/evdev/uinput/index.html)
rather than bespoke ioctl code. Its inspected 0.13.2 manifest declares Rust 1.64;
transitive dependency and target builds still need qualification.

- Mouse mode: relative movement, real button down/up and wheel events, ending
  each event frame with synchronization. It is the broadest first target.
- Absolute targeting: test a separately classified absolute device or compositor
  pointer route before promising pixel-exact mouse placement. A relative mouse
  does not inherently know its final screen coordinate or acceleration.
- Touchscreen mode: absolute contacts, direct-device classification and
  [type-B multitouch slots/tracking IDs](https://kernel.org/doc/html/latest/input/multi-touch-protocol.html).
  Validate device recognition, output mapping and stable contact identity.
- Touchpad mode: a different device classification and interaction contract;
  it must not be conflated with direct-screen contacts.

[libinput](https://wayland.freedesktop.org/libinput/doc/latest/gestures.html)
interprets touchpad gestures but leaves touchscreen gesture interpretation to
clients. The [labwc manual](https://labwc.github.io/labwc-config.5.html#touch)
likewise explains that touch points reach applications and offers optional mouse
emulation. Thus a virtual touchscreen can behave like real hardware, but universal
pinch-to-zoom in every application is not a valid acceptance promise. Qualify
native Wayland and XWayland clients separately.

A compositor virtual-pointer protocol is a useful alternative for pointer-only
experiments, contingent on advertised support. It does not replace the eventual
multitouch device contract. No protocol support was inferred from labwc's name.

## Interaction design to retain

Use an explicit engagement state, not every visible hand as an active cursor.
Map the comfortable hand workspace to the display and allow recentering without
clicking. Compare relative movement with absolute targeting rather than silently
choosing a tiring full-screen arm excursion.

Measure thumb/index distance normalized to palm size; use separate closing and
opening thresholds plus short confirmation windows. Preserve fist-versus-pinch,
occlusion and hand-crossing cases already addressed in Barehands. Track the same
hand across frames; a detector array index is not stable identity.

The [1€ filter](https://gery.casiez.net/1euro/) is an established speed-adaptive
noise/lag tradeoff and matches our existing smoothing experience. Apply real
timestamps, not assumed FPS. Compare palm/index/pinch-midpoint cursor anchors:
the clicking motion itself can displace the pointer. Evaluate limited cursor
stabilization at button transition, without concealing lag in measurement.

Button semantics should be down while deliberately pinched, up on release;
movement while held produces drag. Defer double-click to normal application
behavior initially. On tracking loss, stale input, stream disconnection or
stop: release held buttons/contacts and cancel the interaction. Reacquisition
must start neutral, without an invented click. A local watchdog must work
independently of a networked model. Keep the real mouse usable.

## Recommended first architecture and sequence

```text
manual session
    camera → perception backend → timestamped hand observations
                                  ↓
                         Rust gesture state machine
                                  ↓
                         Rust Linux input adapter
                                  ↓
                        libinput → labwc → application
```

The gesture engine consumes normalized points, confidence, stable hand identity,
source timestamp and explicit tracking-loss events. Native perception and replay
implement the same input contract. A minimal device writer owns buttons/contacts,
bounded input validation and release timeouts; the recognition process should
not run as an unrestricted administrator merely to emit events. Determine the
receiving operation grant during implementation; do not change uinput permissions
as part of research. Camera/session state must remain visible in the existing
panel, with manual start/stop and no startup capture or automatic inference.

1. Qualify the Rust virtual mouse with replayed points and a scratch desktop task.
2. Compare the proven tracker and native ONNX/tract candidates using identical
   inputs and transforms; choose by measured accuracy and latency.
3. Connect one real hand to pointer/click/drag/scroll and test existing applications.
4. Add direct multitouch with stable slots and application-specific acceptance.

## Acceptance experiment

These are proposed criteria, not achieved results. The future human-initiated
session should use the same C920 placement, lighting, resolution and hand sequence
for each candidate. Start with one hand and 640×480/30 input; compare 720p only
if distance/accuracy needs it. Model crop resolution and sensor resolution differ.

Record capture-to-event latency p50/p95, fresh inference FPS, dropped frames,
whole-process-tree CPU/RSS, temperature/throttling and jitter at rest. Inference
time alone is insufficient. Use a latest-frame queue so overload drops old frames
rather than accumulating input delay. Proposed usability gate: at least 18/20
intended clicks, 10 controlled drags, no unintended click in a 60-second relaxed
interval and all held inputs released on hand loss/stop/disconnect. Record misses
and false actions separately; test at approximately 1 m and the existing 2 m goal.

Try desktop selection, VLC controls and Chromium, including fullscreen and the
overlay. Repeat while video playback is active to reveal shared GPU/CPU pressure.
For multitouch, check a known touch-aware client and a mouse-only client; document
fallback rather than claiming the latter gained gesture understanding. Include a
seated five-minute comfort check before calling the device a mouse replacement.

Outstanding: exact-model ARM64 compatibility, Pi 5 native speed, uinput/libinput
classification, absolute mapping and repeated user accuracy. No camera benchmark
was run in this research. The concrete next implementation is the Rust input
adapter plus replayable gesture state, not a full application rewrite.
