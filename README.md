# WiggleControl

WiggleControl turns camera-observed hand gestures into operating-system input.
Existing applications receive mouse events and, later, direct multitouch contacts;
they do not need a WiggleControl integration.

**Status: experimental one-hand Linux mouse in human use; no packaged release yet.**
The selected beta uses headless Chromium GPU hand inference, Rust gesture interpretation
and Linux virtual input; a native CPU backend also exists. The next delivery is
the [standalone Linux screen/detail preview](docs/LINUX-PREVIEW.md).
Left/right gestures have [first positive human feedback](docs/ACCEPTANCE.md);
[held absolute detail](docs/DETAIL.md) has positive first-use feedback with clicking in a real menu;
precision/rest/drawing and longer comfort qualification remain. Gesture
[scroll](docs/SCROLL.md) and [Escape](docs/ESCAPE.md) also have positive first-use
feedback, with integrated task/comfort qualification still open;
multitouch and other operating systems remain later qualification/ports.
The initial receiving target is Raspberry Pi 5, a USB RGB camera and a 1080p
labwc/Wayland desktop. Rust is the first implementation choice. Exceptions need
a specific technical justification; low resource use must be measured.

Start here:

- [Linux public preview milestone](https://github.com/AlterMundi/WiggleControl/milestone/1): scoped features, packaging and acceptance.
- [Current roadmap](docs/ROADMAP.md): iteration goal, realistic mode/model expansion and evidence gates.
- [Observation/replay contract](docs/OBSERVATIONS.md) and fictional fixtures.
- [Direct-touch intent](docs/TOUCH.md), separate from camera hand engagement.
- [Modes and macro semantics](docs/MODES.md), with camera-free mode arbitration and named-action replay.
- [Native controller session](docs/CONTROLLER-NATIVE.md): opt-in modes, manual controls and independent delivery.
- [Installation, manual use and rollback](docs/INSTALL.md).
- [Connected preview setup from source](docs/PREVIEW-QUICKSTART.md): explicit resources,
  fresh configuration, guided calibration and independent manual launch.
- [Full preview candidate assembly](docs/PREVIEW-PACKAGE.md): source-bound binary,
  tools, pinned GPU assets, dependency notices and integrity verification.
- [Development plan](docs/DEVELOPMENT.md): milestones, acceptance and next step.
- [End-to-end acceptance checkpoint](docs/ACCEPTANCE.md): evidence and remaining human trial.
- [Research](docs/research/state-of-the-art.md): existing systems, models, native
  Rust inference choices and Linux input semantics.
- [Implementation tracking](https://github.com/AlterMundi/WiggleControl/issues/1).
- [Project context](docs/CONTEXT.md): decisions and current limits.
- [Agent instructions](AGENTS.md): contribution and device-test workflow.

## Current runtime architecture

Rust acquires camera frames, interprets hand observations, applies calibration and
cursor filtering, and delivers Linux virtual input. The selected perception
backend runs pinned MediaPipe Tasks in a headless Chromium worker using the Pi's
qualified WebGL GPU route. Camera/skeleton display is optional and separate.
The existing native Rust/ONNX CPU backend remains available for comparison.

Chromium currently provides a working model/runtime/GPU combination, with extra
process, memory, startup and image-transport/decode costs. The dated
[matched Tasks CPU/GPU comparison](docs/research/palm-correspondence-gpu-2026-10.md)
is evidence for this route, not a matched claim against the native ONNX backend
or current camera-to-pointer latency.

Nicolás's decision on 2026-10-18: retain Chromium for this preview and pursue a
future Chromium-free backend with equivalent tracking quality and equal or better
end-to-end performance. The observation seam allows replacing inference while
retaining gesture meaning, calibration and OS delivery. See
[the future backend track](docs/ROADMAP.md#future-inference-backend-without-chromium).
This exploration is not a new release prerequisite or an immediate backend rewrite.

## First useful outcome

One manually activated camera session controls the desktop pointer, click, drag
and scroll in ordinary applications. Losing tracking or stopping releases held
input. The actual camera/session state stays visible in the existing local panel.
The physical mouse remains available.

Native recognition and direct multitouch follow measured qualification. Mouse,
touchpad and touchscreen contracts differ: an application must support touch
gestures to interpret native pinch-to-zoom or rotation.

## Existing work and ownership

Reuse the tested ProyectoRPi/Barehands tracking baseline and selected HarMoCAP
methods rather than starting from scratch. Preserve each source's attribution
and license; the private prototype is not bundled here.

[daimon-integrators](https://github.com/AlterMundi/daimon-integrators) owns the local
observation/control panel. WiggleControl owns hand observation interfaces, gesture
interpretation and the virtual-input runtime. Shared Resources owns discoverable
inventory and receiving access references. Matrix identity or capabilities are
not granted by a camera or a virtual device.

Original code and documents use MIT; third-party code and models retain their
own terms. See the research before selecting or deriving an implementation.
