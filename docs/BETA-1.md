# Beta 1 — one-hand desktop control

Next delivery scope — 2026-10-18: [Linux public preview](LINUX-PREVIEW.md),
[milestone 1](https://github.com/AlterMundi/WiggleControl/milestone/1).
This is planned screen/detail and basic-action work, not receiving adoption.
Detail may change during a held action if naturally independent of button state;
defer only separate added work. Existing beta evidence follows.

Update2026-10-18: Nicolás confirms that the stuck-folder incident has not
recurred and asks to consider[#8](https://github.com/AlterMundi/WiggleControl/issues/8)
resolved. It no longer blocks the beta; the original cause remains unknown.
This supersedes pending-reproduction statements below. See
[the goal update](ONE-HAND-GOAL.md#sticky-desktop-drag-resolved-by-human-acceptance-2026-10-18).

Nicolás selected a usable one-hand beta before resuming open multipoint research
on2026-10-16. The active priority is now to improve this one-hand controller;
the wider touchscreen roadmap remains preserved for later. This beta is an
explicit manual desktop tool.

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

Local source experiments now include bounded drag continuity, independent cursor
reconstruction and brief click stabilization. Their
[measured tradeoffs](research/one-hand-interaction-2026-10.md) are not receiving
adoption: the installed beta and calibration remain unchanged. Single-observation
press confirmation is available only by explicit matching configuration; its false
closure counterexample prevents selecting it as the default. The Pi is reserved
for Farscape, so issue#8 and integrated application/human acceptance remain pending.
An optional weak-opening confirmation experiment also stays disabled: it filters
one noisy opening but merges an indistinguishable deliberate brief release and
reduces weak-open double-pinch coverage. More replay held time is not acceptance.

## Manual startup and use

### Optional feedback with a separate detector

The mouse-only route is `/provider/control`. Its Start explicitly requests
preparation; opening or closing it does not acquire the camera or stop the mouse.
A separately managed browser loads `/provider/worker`, which waits for that
request before model warmup/capture. That page loads no preview script, camera
view, skeleton drawing or feedback-image poll. The selected browser must retain
hardware GPU inference; a headless flag alone is not receiving qualification.

Optional `/provider/feedback` contains only the existing visual feedback, without
a detector client. Closing it ends its polling/rendering while the separate
detector continues. The older `/provider/view` retains the combined comparison
path. Do not use that combined route as the mouse-only detector.

Losing the detector's frame requests/results for five seconds now ends capture
and input through the existing cleanup. This is separate from hand loss and its
shorter cursor/held-input rules. An absent hand with a connected detector keeps
the session usable. Manual standby stays indefinite; no automatic restart or
startup capture is added. Private recording remains enabled for human trials.

This increment follows human feedback that the latest calibration was precise
and comfortable but felt slow. Closing the old combined view killed its detector
and left capture active; that receiving session was stopped.

Native source d092c51, artifact SHA256
`e5acf7812e4a3b4862e8a4c8dc108c856c280dfb1623bad098bf9905a74dc1c3`,
passes all jobs of CI39023513160. Receiving Chromium154 headless mode retains
Broadcom V3D hardware inference. In an actual synthetic-only run, accepted results
increased from28 to49 after both optional pages closed; the detector loaded zero
feedback-image requests and had no camera canvas. Closing its target exercised
the five-second worker-loss error and neutral cleanup; camera/input stayed off.
The actual desktop Stop also reaped source and detector in manual standby, and
the launcher reopened controls bound to the fresh run. Source/worker/control are
manual user units, without restart or boot enablement. Prior runtime/configuration
and WG1 remain rollback; the verified workspace and gesture settings are unchanged.

The independent controls are ready for fresh human Start. The control window can
be closed after starting; reopening it through `tools/mouse_launcher.py open`
does not restart the detector or held input. Private recording remains enabled.
Physical comfort and comparative performance in this mode remain untested.
Removing preview work is not evidence that inference or end-to-end latency improved.

### Existing combined route

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

Both beta units were stopped at the last receiving inspection, pending delivery
diagnosis; no live inspection occurred during Farscape. The beta is not accepted
for everyday use. Next: reproduce with empty owned test folders and trace this
writer's kernel button events alongside actual application drop results, without
camera acquisition. Stop must be qualified at the application level as well as
the controller/device levels. The installed release's camera_opened/system_input
terminal fields retain historical acquisition flags; use actual unit/device
observations for teardown, and correct the receipt fields before acceptance.

Follow-up source corrects these terminal fields and adds a device-free absolute
pipeline replay; see the [interaction baseline](research/one-hand-interaction-2026-10.md).
This correction is not yet deployed and does not diagnose the sticky drag.
The replacement one-hand goal is active. The human has reserved the receiving
screen for Farscape; receiving injection and heavy experiments wait until it is free.

The [camera-free drag trace](ABSOLUTE-DRAG-TRACE.md) is now prepared locally. It
reuses the installed finite writer and compares ordinary release, Stop while
held, producer silence/EOF and writer death using owned empty folders and only
the new virtual mouse's kernel stream. Fixture identity and event framing/protocol
tests pass without devices. This is a prepared diagnostic, not an executed Pi
test or issue#8 diagnosis; actual filesystem drop and remaining desktop grab
must be inspected separately.
Trace-sourcefc7bb9d CI38865365610 passes all jobs including ARM64; this verifies
the prepared source checks, not a real kernel trace or application recovery.

The next source-only candidate adds optional bounded drag continuity, with
independent writer liveness and geometric same-owner return. It is disabled by
default and is not installed. Synthetic dropouts improve, but older retained
predictions show no held-time change, and catch-up motion/noisy opening remain
unfinished. See the same comparison report; do not treat this as Beta acceptance.

A further source-only option reconstructs cursor motion at independent ticks and
bounds prediction by time, distance and velocity consistency. Replay measures
smoother output cadence with remaining jitter/lag/overshoot tradeoffs. It is also
disabled by omission and not installed; pinch stability, issue#8 and human/application
acceptance remain open. No receiving screen or heavy work is used during Farscape.

## Camera-free receiving return —2026-10-16

After Nicolás freed the Pi and resumed the one-hand goal, the unchanged installed
1cf0aed writer passed one owned desktop fixture for each of normal release,
held Stop, watchdog, producer EOF and writer SIGKILL: synchronized kernel down/up,
original folder moved, child reaped and device removed. A separate cursor-alive
no-click probe showed no floating folder in the inspected image after each case.
[ABSOLUTE-DRAG-TRACE.md](ABSOLUTE-DRAG-TRACE.md#first-receiving-desktop-comparison-2026-10-16)
owns exact scope and limits. Issue#8 is still unreproduced, not fixed or closed;
The later window comparison below extends this checkpoint; physical one-hand
acceptance remains pending. No camera,
application restart or candidate/runtime/settings switch occurred. Only the owned
empty fixture folders were removed after verification; original evidence remains.

## Window/click comparison and staged candidate —2026-10-16

The same five controlled terminations subsequently passed once each on a PCManFM
window, with original empty-folder moves, kernel down/up and no floating folder in
inspected post-run hover images. Issue#8 remains unreproduced, not diagnosed or
fixed. Two traced exact-position mouse pairs confirmed application behavior:
300ms opened the owned folder,500ms only selected it. GTK3 reports400ms/25px for
double clicks and8px drag threshold; spatial tolerance was not functionally tested.
[The trace report](ABSOLUTE-DRAG-TRACE.md) retains exact scope and limits. The two
owned views and remaining empty fixture folders were closed/retired afterwards.

A1cbb99's CI38950807712 passes all jobs. Its ARM64 candidate manifest and executable
agree on SHA256 `1787298ce84fc2237f62fd074f90189d7e893bc57a8f059422d3f9db15da9a7c`.
The candidate is staged separately, without selecting it for WG1 or starting a
service. Both receiving `provider-camera-until-stop-preflight` runs pass with
camera/input/acquisition false, one hand, original0.5 presence floor and recording
configured. The reference session/mode bytes match installed Beta1; the smooth
profile changes only the matched optional one-second drag grace and cursor output:
16.667ms interval,30ms smoothing,80ms bounded prediction,0.01 maximum normalized
lead, and200ms closing/150ms pressed stabilization within0.008 radius. Forty-ms
press confirmation and immediate measured opening remain the defaults; the
rejected weak-opening filter is absent. Calibration, models, camera request,
recording root and disabled macros are preserved.

This is staged/preflight-tested, **not a deployed or accepted camera controller**.
The manifest's receiving-qualified flag remains false. Actual SDK/provider timing,
input observation, reversible A/B launcher/standby preparation and fresh-ready
physical trials are next. Installed1cf0aed remains the selected beta. There is no
new background model, camera, input writer or discarded experiment service.

## Manual finite comparison preparation —2026-10-16

Source `d8f9bb920776caaba8d5ab386230d757b286c8ce`, CI38956972191, passes
replay/native/ARM64 jobs. Receiving executable SHA256 is
`a9af09be68df91dd12dc534296c7f2e7865e2a3457d1ac14cf681f3d6532e34f`.
It adds the [manual finite trial/observer tools](ONE-HAND-TRIAL.md), using the
same reference/smooth configuration bytes described above. The separate trial
units are manually started, not enabled, with no restart. WG1 still selects1cf0aed;
the calibration, models, recording root, physical input and power profile remain.

Both manual45s preflights pass with recording configured and acquisition off.
An actual source/browser remained idle for at least89.7s with no virtual mouse,
camera captured0 and no input started. Rendered standby is1920x1018, without
page overflow; Start/Stop fit and the right panel margin is preserved. The initial
1440x990 restored browser window required explicit compositor maximization;
the startup flag alone did not establish desktop fit. Clicking the actual
frontend Stop ended the provider and observer with exit0, reaped both and left
the unit inactive/success, no virtual node and empty stderr. No camera/model
trial ran. Observer startup coverage correctly remains unknown before attachment.

A separate camera-free owned-folder move qualified the new independent observer
against the existing fixture observer: each retained47 records, one down/up,
no loss/partial records and device removal. Original source identity moved and
the inspected cursor-alive image showed no floating folder; the empty fixture
was then retired. Attachment preceded all scripted packets in this particular
case. This is event/lifecycle evidence, not a signed ownership claim, issue#8
reproduction, natural pinch accuracy or end-to-end latency measurement.

The receiving runbook owns exact trial selection/Stop/rollback and private logs.
Physical45s comparison remains pending fresh human readiness and manual Start.
Keep camera video, semantic journal, raw kernel edges, application result and
human observations separate; do not promote this candidate to the everyday beta
or mark its manifest receiving-qualified from standby/source checks alone.

## First finite smooth trial and receiver diagnostics —2026-10-17

The human manually started the prepared45s smooth trial on2026-10-16 and reported
that the portion that ran worked very well. This is positive overall feedback,
not separate acceptance of every gesture or completion of the full goal. The
trial ended early with `delivery requires fresh source and an explicit armed mode`
from the Frame validator, followed by provider cancellation. Issue
[#9](https://github.com/AlterMundi/WiggleControl/issues/9) owns the interruption;
the precise failed condition and original wire timing were not recorded.

The independent observer retained3257 kernel records and four down/up pairs,
no SYN loss/partial/unsynchronized records and device removal. Its last movement
preceded the last release by about303.5ms. Initial state/startup before attachment
remain unknown; no compositor-grab claim follows. Terminal source receipts confirm
camera/input off, worker closed and writer reaped; provider exit1/observer exit0.
The source unit was failed with an empty cgroup and the own browser was stopped.
The retained recording has305 frames over20.064818s,32016800 original bytes,
continuous in-range index entries and full640x480 JPEG decode.298 payloads retain
up to31 bytes after JPEG EOI from their raw capture buffers. Do not trim those
bytes or call that padding corruption. Decoding does not establish intent labels.

A device-free full pipeline replay uses all248 interpreted observations with
their original source/evaluation intervals, shifted by one epoch, supplied5ms
ticks and a shared producer/receiver clock. It passes. A separate delivery replay
adds a selected delay only to the final generated Frame: its136.245ms source age
at publication becomes306.245ms with170ms transit, yielding source expiry.
299ms transit also yields unarmed receiver state.0/50/150/160ms transit cases
pass; all seven cases release every generated button pair. These are artificial
counterexamples, **not measured original transport delay or its demonstrated cause**.

Current source reports `wigglecontrol.delivery-rejection/v1` context on this exact
Frame rejection path: failed predicates, source/packet ages, previous source,
receive gap, mode and arm state before/after timing checks. It records no pointer
coordinates. The freshness/arm/order/count checks and terminal release behavior
are unchanged. A regression distinguishes source expiry during transit from an
independent writer-watchdog lapse. This diagnostic improvement is not a runtime
fix or a newly accepted beta; WG1 still selects1cf0aed. Capture the actual failing
predicate on a receiving repetition before recording a causal finding, then fix
that cause and qualify complete finite/application use. New camera trials retain
fresh readiness/manual Start and private recording.

Diagnostic source `228cc30d96d4426f564a1107816b342cb98cc48d` passes native tests,
Clippy and CI38989727471 (replay/native/ARM64). The receiving ARM64 executable
SHA256 is `bced308a98698f4fd2bb0ad42c32f07d4c3ac248436415b1ebb1765bd08819a9`.
Its device-free replay passes4410 retained-input commands/4411 receipts and four
simulated down/up pairs. The delayed-source example reports the intended precise
diagnostic and releases its simulated pair. Both finite trial specifications now
select that exact executable; session/settings bytes, private recording and
previous executable/specifications are preserved. Both manual preflights pass.
Actual standby confirms camera/input off and an armed independent observer;
the interface's same-origin `POST /stop` ends provider/observer with exit0,
both reaped, unit inactive/success and empty cgroup. No camera, virtual mouse or
browser was started for these checks. Prior rendered-control evidence is reused
because the frontend is unchanged. This qualifies preparation and Stop, not a
complete camera trial. The diagnostic candidate is ready for a fresh manual45s
repetition; WG1 remains the separate baseline.

Operational distinction: `/stop` is the human Stop route. `/provider/stop` reports
worker cancellation and intentionally yields an error. An initial receiving probe
omitted Origin (403), then used that worker route (cancellation/exit1); both probes
were reaped without acquisition. Neither is evidence of a broken human Stop.

The retained failed trial also supplies actual pointer cadence:1235 motion
reports at15.488ms median/20.731ms p95 spacing, while interpreted hand evaluations
are78.757ms apart at median. This is device-output evidence, not screen refresh,
reduced perception delay or complete-trial acceptance. The
[cadence analysis](research/one-hand-interaction-2026-10.md#actual-finite-trial-pointer-cadence-2026-10-17)
records clock, eligibility-gap, unlabelled-hold and unmatched-baseline limits.

## Subsequent human trial and recalibration —2026-10-17

A second manual45s smooth run retained651 captures,394 issued results and four
observed kernel down/up pairs; provider and observer both exited0 and were reaped,
with the virtual device removed. The browser reported a503 delayed meaning receipt
at the end. A finite-deadline/in-flight-result race is a hypothesis to verify;
the original Frame-guard rejection did not recur. This is not whole-goal acceptance.

The human then explicitly requested use until Stop and recalibration first.
The existing manual `provider-camera-until-stop` command is prepared with the
same228cc30 smooth profile and private recording; camera-free preflight and Stop
pass. It remains stopped while recalibration is pending. The first guided attempt
failed, with out-of-frame hand reported by the human. Its source ended while the
native overlay stayed running. The receiving launcher now restarts the overlay
per attempt and stops it on source exit; two camera-free desktop-launch cycles
pass. See [the display lifecycle](ONE-HAND-TRIAL.md#guided-calibration-display-lifecycle).
The previous accepted calibration remains selected until a new verified result
is adopted and matching candidate session/settings are updated. Do not restart
a finite controller in place of the human's explicit until-Stop request.

A further guided retry accepted both measurement poses and entered verification,
then timed out at the first upper-left target. Its mapped position remained near
lower-right throughout. Source guidance incorrectly said `hold` immediately on
every verification corner, despite requiring movement. It now reports `move`
until actual fresh in-target dwell begins, then `hold`; loss/movement resets that
cue. This changes guidance, not sampling, verification or adoption thresholds.
Receiving adoption of this source correction must be recorded separately.

The corrected calibration source4581f4a passes all three jobs of CI39020555617,
including ARM64 tests. Its receiving executable SHA256 is
`cca591e8a17a62c905fad8eda2f045720917625957b4d5801c496e7dd767c758`.
The calibration unit selects that artifact; config-check passes with recording
enabled and the unit remains inactive for human manual Start. The native overlay
binary, sampling configuration, prior accepted profile and until-Stop controller
remain unchanged. Previous failed draft/evidence and executable are preserved.
No camera/input was opened for delivery. A new human run must still demonstrate
all four verified corners before adopting its result.

The next human retry failed at the lower-right measurement again, with no usable
samples and an out-of-frame hand, before verification. The source now puts that
tracking rejection directly into the guided instruction and suppresses HOLD for
invalid tracking, rather than relying only on the smaller status text. This
extends the earlier verification-cue correction; it does not bypass either pose
or any of the four corner checks. Receiving delivery remains a separate record.

Tracking-cue source79f01f6 passes native tests/Clippy and all jobs of CI39021786482.
Receiving calibration now selects its ARM64 artifact, SHA256
`eefbd8baf7373824ada73d03412bc2ab1f96e5bb64f6d351335325e555e63735`;
recording-enabled config-check passes and the source remains inactive for manual
Start. A camera-free HTTP1.1 metadata fixture on the actual1920x1080 display
visibly confirms both the inward-movement warning and the first verification
corner's MOVE instruction. The fixture/overlay were reaped; no camera or input
was opened. This qualifies the displayed cues, not a new successful calibration.


The subsequent human run completed both poses and all four corners. The existing
`accept-calibration` command verified the saved candidate exactly and adopted it
without acquisition. The matching reference/smooth workspaces and specification
hashes were updated; both controller configuration checks and the continuous
manual preflight pass. The prior profile and settings remain available privately.

At the human's request the smooth228cc30 controller now runs until explicit Stop
with its inference browser minimized: the desktop shows the regular pointer,
without camera video or skeleton. Camera capture and private recording remain
active, as required for hand control. This hides the feedback window; it does not
remove the browser inference dependency or prove a performance improvement.
The task-local STOP desktop entry remains available. WG1 rollback is unchanged.


## Software-only hot-path work —2026-10-18

The human deferred camera/lighting changes and authorized software optimization.
The first increment encodes each journal event once, reuses its exact newline-
terminated bytes for both destinations, coalesces frame-index writes, and sends
each native-writer packet with one `write_all`. No batching timer or deferred
flush is introduced. Regression checks compare previous bytes, including floats,
Unicode/escaping and recording-index rows; existing release/freshness tests remain.

The no-preview worker now also avoids server-side snapshot preparation unless
`/frame` or `/state` was requested within500ms. Empty first requests register
interest; views can reopen and resume without restarting inference. Calibration
publication is unchanged, and terminal feedback is always published. The terminal
`feedback_snapshots` counter counts live feedback publications, excluding terminal.

Recent browser timing is bounded to1024 entries with batched eviction and an
explicit discarded count. Private video/observation journals retain full evidence.
This removes unbounded diagnostic growth, not input samples. Source/native tests,
receiving replay and human comfort must be reported separately; write-call or CPU
savings alone do not demonstrate lower camera-to-screen latency.


Receiving adoption: native2c533e5 passes full CI39122960408, receiving GPU synthetic
replays, feedback-demand recovery and camera-free manual until-Stop preflight.
It is selected for the next manual Start, with the old binary/units retained.
A separate browser quota comparison selects200% aggregate CPU allowance, removing
frequent scheduler stalls without changing host frequency/fan policy. Repeat
source→meaning p95 improves~101→79ms in the camera-free fixture; the model and
physical gesture quality are unchanged. A60s15Hz source processes901/901 frames.
No production camera/model/input/display was started for adoption. Human
smoothness acceptance and camera/lighting work remain pending; see the
[measurement scope](research/one-hand-interaction-2026-10.md#receiving-software-only-comparison-and-scheduling-quota-2026-10-18).
