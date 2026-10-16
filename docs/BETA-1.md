# Beta 1 — one-hand desktop control

Nicolás selected a usable one-hand beta before resuming open multipoint research
on2026-10-16. The wider controller goal retains its full scope and is paused
while this release is prepared. This beta is an explicit manual desktop tool.

## Selected behavior

One fixed GPU hand graph feeds the existing Rust controller and independent Linux
writer. An accepted comfortable workspace maps hand position to the absolute
screen pointer. Open thumb/index before engagement; pinch to click, hold and move
to drag, separate to release. Pointer and manual scroll are the selected modes.
Changing modes or resuming releases input and requires fresh open evidence.

The beta selects the measured projected wrist→middle-MCP pinch ratio0.38/0.40,
input/presence floor0.5 and freshness300ms. Existing smoothing remains unchanged.
Visual macros, measured-presence internals, precision and multi-contact modes are
not selected. These are release choices, not evidence that every hand/view works.
The saved calibration and prior runtime remain available for rollback.

## Manual startup and use

The receiving dashboard entry opens the beta's feedback/control view. The manual
server can wait idle; it opens no model, camera, recorder or virtual device until
the human presses **Start Beta1**. Startup warms the graph on a separately labelled
public synthetic image. Private recording and actual camera/input begin afterward.
The preparation deadline starts on Start, not when the idle page was opened.

Use the desktop with the feedback window retained/minimized. Pause disables input
while capture continues; Resume requires open evidence. Stop closes this session,
its camera, recorder, worker and mouse. Restart is another explicit manual action.
Physical mouse/keyboard remain usable. No camera/model service starts at boot.
The dashboard's independent device observations remain distinct from managed
session and HTTP feedback. A requested Stop is not itself proof of sensor closure.

The beta uses `provider_absolute_pointer:true`, maxHands1 and matching session/
controller workspace and gestures. The explicit command is:

```
wigglecontrol provider-camera-until-stop[-preflight] SESSION SETTINGS ASSETS SYNTHETIC_WARMUP PORT 1 [--inject]
```

Preflight opens no listener/model/camera/device. The live command initially waits
for manual Start. Without `--inject` it supplies feedback only. Existing finite
camera commands stay finite; zero does not select continuous mode in those CLIs.
The source-bound absolute writer reuses transition, source-age, sequence, mode and
watchdog fences. It creates only the absolute mouse. Position is emitted before
a same-frame press. A missing/stale hand cannot invent a new press.

## Qualification and limits

Native-feature source tests and Clippy pass. Tests cover calibrated absolute
coordinates, position-before-press, watchdog release/stale rejection and the
manual Start boundary. Browser startup testing confirms no model preparation
before that click. CI, receiving transport/device delivery and human use must be
recorded separately below before calling the complete beta accepted.

Known limits: projected pinch can fail at adverse angles; tracking loss cuts a
drag and requires reopening. This beta does not solve occlusion, participant
ownership, macros or touchscreen equivalence. The next useful evidence is seated
pointer alignment, deliberate clicks/drags, loss/Stop and actual application use.

## Receiving delivery —2026-10-16

Manual Beta1 source `1cf0aed6b52028b510f14dc098b4cf342febd400` passes CI38849576988
(replay, native and ARM64). Receiving binary SHA256 is
`f4ad91eaf5f8b1e5f44c0e87d68e57ec6ea71666d23094c4025e068234a40eb9`.
The separate explicit launcher is sourceb855c48, CI38849745647 allgreen.
The dashboard buttons are daimon-integrators6b36436, CI38849765459 allgreen.
No build runs on the Pi. Existing runtime/calibration/system panel binary remain
retained; the new panel uses a reversible user-service override.

On the actual desktop, **WG1** opens the beta and **■ WG1** stops its two named
user units. Open the feedback window, press **Start Beta1** when ready, then
minimize that window to use ordinary applications; retain it for the GPU worker.
Return through WG1 for mode/pause/feedback. Use the dedicated Stop WG1 control,
which also closes this beta browser. The older panel capture Stop remains a
separate control. Neither beta unit is enabled at boot and Restart is disabled.
The earlier experiment workbench was closed at Nicolás's request and is not
restored automatically.

The receiving app now starts maximized, with a1920×1018 client inside the
1920×1080 desktop. The1440×990 fallback fit its DOM but retained a window position
that extended below the display; DOM bounds alone did not prove desktop fit.
An earlier1080×820 view also had footer/control overlap. The server remained
idle beyond the old30s preparation bound: zero GPU workers,
camera/input/recording false and independent observer camera/mic false. Existing
calibration digest is unchanged. These are startup/layout/source-delivery proofs.
Nicolás subsequently pressed Start and used the camera/input for approximately
119seconds. A desktop folder drag remained attached after semantic button-up,
source/browser shutdown and virtual-device removal. The source journal contains
release events; no contemporaneous kernel/client trace exists, so the point of
failure is unknown. Escape and a balanced virtual-pointer click did not cancel
the drag. Restarting only PCManFM's desktop process removed the floating icon;
subsequent pointer movement and a screenshot verified recovery. The original
folder remained on the desktop. No compositor/session restart was needed.

Both beta units are stopped pending delivery diagnosis. The beta is not accepted
for everyday use. Next: reproduce with empty owned test folders and trace this
writer's kernel button events alongside actual application drop results, without
camera acquisition. Stop must be qualified at the application level as well as
the controller/device levels. The terminal journal's camera_opened/system_input
fields currently retain historical acquisition flags; use actual unit/device
observations for teardown, and correct the receipt fields before acceptance.

Follow-up source corrects these terminal fields and adds a device-free absolute
pipeline replay; see the [interaction baseline](research/one-hand-interaction-2026-10.md).
This correction is not yet deployed and does not diagnose the sticky drag.
The replacement one-hand goal is active. The human has reserved the receiving
screen for Farscape; receiving injection and heavy experiments wait until it is free.

The next source-only candidate adds optional bounded drag continuity, with
independent writer liveness and geometric same-owner return. It is disabled by
default and is not installed. Synthetic dropouts improve, but older retained
predictions show no held-time change, and catch-up motion/noisy opening remain
unfinished. See the same comparison report; do not treat this as Beta acceptance.
