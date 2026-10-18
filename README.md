# WiggleControl

WiggleControl turns camera-observed hand gestures into operating-system input.
Existing applications receive mouse events and, later, direct multitouch contacts;
they do not need a WiggleControl integration.

**Status: experimental one-hand Linux mouse in human use; no packaged release yet.**
The selected beta uses headless GPU hand inference, Rust gesture interpretation
and Linux virtual input; a native CPU backend also exists. The next delivery is
the [standalone Linux screen/detail preview](docs/LINUX-PREVIEW.md).
Right-click has an [opt-in source candidate](docs/MOUSE-ACTIONS.md), not yet
physically accepted. Gesture Escape and absolute detail remain planned;
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
- [Development plan](docs/DEVELOPMENT.md): milestones, acceptance and next step.
- [End-to-end acceptance checkpoint](docs/ACCEPTANCE.md): evidence and remaining human trial.
- [Research](docs/research/state-of-the-art.md): existing systems, models, native
  Rust inference choices and Linux input semantics.
- [Implementation tracking](https://github.com/AlterMundi/WiggleControl/issues/1).
- [Project context](docs/CONTEXT.md): decisions and current limits.
- [Agent instructions](AGENTS.md): contribution and device-test workflow.

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
