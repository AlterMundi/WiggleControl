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
