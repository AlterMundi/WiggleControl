# Native controller session and independent delivery

This opt-in source path connects native perception, the exclusive controller,
manual preview controls and an independent mouse/touch writer. The existing
fixed-mode and calibrated commands remain available. Receiving deployment,
combined-device compositor behavior and real-hand comfort still need qualification.

## Prepare and start

Build with `--features native-vision`. Use the receiving session configuration and
an explicit mode-settings file (see [the example](../tools/controller-modes.example.json)).
Their gesture settings and touch workspace must agree: preflight rejects divergent
values before initializing inference or opening a camera. Session `pinch_3d`
selects the shared detection conversion; mode settings do not override it.

```sh
wigglecontrol config-check-controller SESSION.json SETTINGS.json
wigglecontrol camera-check-controller SECONDS SESSION.json SETTINGS.json PORT
wigglecontrol run-controller SECONDS SESSION.json SETTINGS.json PORT
wigglecontrol run-controller-until-stop SESSION.json SETTINGS.json PORT
```

Only preflight is camera-free. Camera-check acquires the selected camera and
provides the same semantic feedback without virtual devices. Run also creates
mouse/touch devices. Finite preview sessions use1..90 measured seconds plus the
existing bounded preparation. The measured clock starts after the first view is
painted; until-stop remains explicitly manual. Follow the existing human-ready
and recording rules before acquiring a camera. No command starts at boot.

This version uses relative mouse movement/precision and the explicit direct-touch
workspace. It does not adopt or rewrite a saved absolute pointer calibration.
Prayer/guided calibration retain their separate existing workflow. Visual macro
classification is not connected in this path (`visual_macros:false`); no gesture
can start an idle camera.

## Visible operation

The existing loopback preview shows current mode, held state and an explicit
primary-hand selector. Mode changes and hand claims release the old input and
require positive open evidence again. Two-contact mode uses both tracks and
disables the one-hand selector. Track IDs do not identify participants.

Manual commands fence observations at their monotonic control time. A queued
open frame captured before the control cannot rearm a hand that is now closed.
The first post-control fresh open observation must be new. This prevents a
buffered camera frame from undoing the mode-transition rearm rule.

Pause stops input while capture/recording remain visible; Resume requires open
rearm. A pause during preparation remains paused when the measured clock starts. Stop ends this owned session. Feedback explicitly reports recording and
input state. A delayed view does not display cached hold state as current;
check the independent local panel for observed capture state after disconnection.

Controls use the existing exact loopback Host/same-origin contract, with one
pending controller command. HTTP acceptance means queued, followed by observed
producer state; it is not an OS delivery receipt. Legacy preview sessions reject
controller controls. Selected mode settings are retained privately beside the
session configuration and raw recordings. Preview control events retain their
exact monotonic time and resulting controller state as separate event rows; they
are not gesture-intent annotations.

## Delivery boundary

The writer reuses existing evdev mouse and direct-touch factories/state encoders.
It owns both file descriptors and consumes only one mode's input per frame.
There is no competing mouse/contact stream. It does not perform perception.

`controller-stream SECONDS GAIN WATCHDOG_MS` and
`controller-stream-until-stop GAIN WATCHDOG_MS` explicitly open devices.
The producer supplies bounded JSONL packets:

- `transition`: sequence, issued monotonic time, mode and enabled state. Release
  mouse/buttons and both touch slots before arming the selected mode.
- `frame`: sequence, issued/source monotonic times, mode, mouse events and the
  complete contact set. Reject stale/future/repeated source, repeated sequence,
  unmatched mode, oversized output or incompatible mixed streams.
- `stop`: release and terminate this writer.

Producer/consumer timestamps share the same host's Linux monotonic clock. The
independent watchdog allows50..1000ms. Expiry releases both devices and latches
updates off; another explicit transition is required. Replaying held contacts
cannot reactivate input after expiry. Source watermark remains across transitions.
EOF, malformed input, error and finite expiry release before destroying devices;
closing the sole uinput file descriptors also removes them on writer death.

The controller tick and fresh-observation paths publish neutral transitions on
loss/staleness. Idle ticks cannot keep held output fresh indefinitely. Old
pre-control frames cannot supply new-mode arming evidence. Pixel/model accuracy,
application gestures and compositor device classification require separate proof.

## Camera/device-free qualification

```sh
wigglecontrol mode-replay SETTINGS.json < controller.jsonl
wigglecontrol delivery-replay GAIN WATCHDOG_MS < packets.jsonl
wigglecontrol pipeline-replay SETTINGS.json GAIN WATCHDOG_MS < controller.jsonl
```

The full pipeline serializes actual writer packets, decodes them through its
consumer contract and emits actual Linux event codes using the existing state
encoders. Its clocks are explicitly supplied by replay commands. It opens no
sensor or uinput descriptor and reports `system_input:false`.

Tests cover button/two-slot release, transition-before-repress, watchdog latching,
source/sequence/mode integrity, malformed/EOF cleanup, pending HTTP controls and
pre-acquisition config agreement. The native UI was exercised on Cuevanico at
1920x1080 with a device-free Rust pipeline and retained synthetic images, explicitly
labelled UI TEST / sensors off / system input off. All five modes, hand selection,
held-mode change and pause/resume were observed. No page overflow or console
exceptions;390px layout had no horizontal overflow. A fixture proves this UI and
semantic path, not live acquisition or compositor effects. Its temporary server
was stopped and the owned browser returned to the research workbench.

## ARM64 candidate build

The Rust workflow tests and builds this native path on an
[ARM64 Ubuntu24.04 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
with the existing Rust1.85.1 pin. This avoids compiling on the receiving Pi and
requires no new local cross toolchain. Its `wigglecontrol-arm64-candidate` artifact
contains a tar archive preserving executable mode, LICENSE/NOTICE and a manifest
with source commit, compiler, architecture, feature selection and binary SHA256.
It contains no model, inference runtime, camera recording or receiving config.
Artifacts expire after14days; the maintained workflow can build another commit.

A candidate build is not an installation or receiving qualification. Before use,
verify the run's source commit and manifest hash, compare the target's libc with
the binary's requirements, then stage at a separate user-local path and run help
and config-only preflight. Retain the selected prior binary and configurations.
Camera/input acceptance follows the existing human-ready and receiving rules.
