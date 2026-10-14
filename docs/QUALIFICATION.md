# Development and receiving evidence

## 2026-10-13 — Gesture replay and Linux registration

Published gesture engine at b55d645, Linux adapter at f35eaa9.
VPS Rust 1.95.0 and receiving Pi 5 ARM64 Rust 1.85.1 each passed 14 tests:
pinch/drag, jitter/confirmation, identity replacement, neutral reacquisition,
stale/stop/loss/ineligible/malformed/out-of-order input release, CLI size and EOF
handling, Linux event codes and fractional movement. Local clippy passed;
GitHub Actions run 38387111298 passed fmt, tests and clippy on Rust 1.85.1.

The earlier run 38386242490 failed only an older Clippy `format_collect` lint in
a fixture-building test; its actual job output was inspected and that expression
was corrected. Do not attribute this to camera or ARM incompatibility.

Receiving temporary ACL allowed only the existing operator to create uinput
devices; original retained locally for reversal. A five-second no-event check
observed udev ID_INPUT_MOUSE=1 and automatic removal after expiry. A separate
finite check observed libinput's `Capabilities: pointer` and adaptive/flat/custom
acceleration availability. No camera or microphone was used. No application
movement/click was emitted. The existing panel remained running.

Remaining for milestone 2: actual native Wayland/XWayland application delivery,
finite watchdog/button-release/SIGKILL device removal tests, calibrated workspace
and scroll mapping. Device registration is not user acceptance.

## Native perception candidates

The separately enabled model-probe feature evaluates tract 0.21.15 with pinned
OpenCV Zoo palm/hand ONNX artifacts listed in models/manifest.json. These are
Apache-2.0 candidates, not MIT assets or a selected production backend. Licenses
are retained alongside the manifest; downloaded model binaries are ignored.
The probe uses a synthetic zero crop, opens no sensor, and separates model-only
load/timing from the eventual full pipeline. Compatibility/performance results
will be recorded after execution, rather than inferred from successful download.

Default camera-free runtime does not link the optional inference engine.
The dependency resolver honors the declared Rust 1.85 floor; an initial native
probe resolve selected kstring 2.0.5 (Rust 1.96) and was corrected to a compatible
locked dependency set without changing either host's compiler.


Offline candidate probe on VPS Rust 1.95.0: hand landmark model produced finite
outputs with shapes [1,63], [1,1], [1,1], [1,63] from the synthetic zero crop.
Debug-profile model-only timing was about 2591 ms for one run and is not a Pi
performance estimate. The palm model failed at Resize__235: expected output
[1,256,12,12], actual [1,256,6,6], both with and without graph optimization. Thus
this pinned tract candidate does not yet qualify the full two-model pipeline.
Preserve this concrete incompatibility and evaluate a different supported
backend/version before choosing production inference. No camera used.


ONNX Runtime 1.22.0 with ort 2.0.0-rc.10 runs both pinned models and produces
finite expected output shapes on VPS and receiving ARM64 Rust 1.85.1. On the
Pi's explicit silent profile (1.5 GHz cap, fan off, ongoing one-core encoder,
SoC about 83–85 C and firmware thermal limiting), 20 synthetic model-only runs
measured palm p50/p95 80.5/87.3 ms and hand 49.8/54.6 ms, using one inference
thread. This is not an accuracy, full-pipeline or thermally neutral benchmark.
Public runtime artifacts and their observed hashes are in runtime-manifest.json;
upstream assets did not supply a digest. Rust wrapper plus native CPU library is
selected for the next perception experiment after the exact tract palm failure.

Native perception geometry/anchor/fist-eligibility tests pass with replay and
Linux mapping tests (19 total). Generated 2016-anchor ordering was compared to
the pinned upstream literal table (maximum coordinate error 3.34e-8). The
current rotated ROI sampling uses bilinear interpolation and direct square crops,
not a claim of byte-identical OpenCV INTER_AREA preprocessing; source-linked
real-input comparison and hand accuracy remain pending. The producer uses
mirrored palm-center cursor and square fingertip metric; these are initial
calibration choices, not final human acceptance.


The first real 20-second camera-only session captured 293 frames, processed 184
(latest-frame drops 108), detected no accepted hands and injected nothing. Driver
timestamps were monotonic start-of-exposure; timestamp-to-intent p50/p95 about
119/150 ms while no hand was tracked. Camera closed afterward; microphone stayed
inactive. Absence of accepted hands does not establish that no hand was visible.

A cropped gesture-grid reference was rejected by both this pipeline and original
OpenCV, so it was not a positive test. The first frame of the pinned upstream
palm demonstration animation gave OpenCV confidence 0.9904 and Rust/ORT 0.9878.
Corresponding 21-point deviation: median 0.45 pixels, maximum 2.50 pixels on a
600x450 image. This is one source-backed positive preprocessing comparison, not
physical accuracy acceptance. Palm threshold now matches upstream's 0.5; hand
acceptance stays 0.8. Native vision plus replay/input tests now total 20.


The relative device delivered trusted pointer, click, drag and wheel events to
Chromium native Wayland at 1080p. A producer stall released a held button in
the application. Forced writer death was separately tested with a keepalive
preventing watchdog expiry: the application observed release and the virtual
device was absent afterward. These are real uinput/libinput/application effects,
not JavaScript/CDP-dispatched input. CDP was used only for state/geometry reads
and preparing this task-owned qualification window.

Installed manually enabled receiving artifacts with model/native-library hash
verification and rollback snapshot; a named-operator uinput rule persists the
required access without root perception. No startup camera/model service exists.
Integrators `input 20` entered visible preparation, then observed input activity
and camera-open with microphone inactive; its existing Stop ended the owned
process group and actual camera usage. Application/gesture human accuracy, VLC
/XWayland acceptance and direct multitouch remain pending.

## Direct-touch writer checkpoint — 2026-10-13

Twenty-two native-vision/replay/Linux tests pass on the development host. New
tests verify stable slots despite producer-array reordering, individual contact
loss, neutral release and rejection of duplicate/nonfinite contacts. The shared
bounded intent reader is used by mouse and touch writers; their watchdog clocks
remain independent of a blocked reader.

Receiving ARM64 Rust 1.85.1 built the camera-free touch writer. Real type-B
contacts reached the task-owned Chromium native Wayland page through uinput,
libinput and labwc. The page received trusted touchstart/touchmove events with
two simultaneous stable contacts despite alternating producer order. Its explicit
touch interpretation measured scale 1.709 and rotation 5.88 degrees. These values
describe the scratch touch-aware client, not universal system gesture support.

A producer stall then delivered touchend and left zero active contacts in the
application. Explicit Stop exited successfully and removed only the virtual
touch device. No camera/microphone was opened. Coordinates matched the single
1080p output; multi-output mapping remains unqualified. Two-hand recognition,
crossing/occlusion identity and camera-to-touch engagement are still pending.

## Two-hand gesture semantics checkpoint — 2026-10-13

Twenty-nine native-feature tests now pass. Five additional state tests qualify
independent confirmed pinches, stable contact ownership after producer reordering,
loss of one hand preserving the other, ambiguity release/open-before-repress with
new contact IDs, malformed/future/duplicate/regressed input, stale/Stop neutrality
and explicit absolute workspace mapping. Two CLI tests verify the fictional
two-hand replay, final EOF release and malformed-input failure with neutral output.
This is camera-free semantic evidence. It does not qualify native recognition of
two hands or whether the producer correctly notices a real crossing/occlusion.

## Native two-hand pipeline checkpoint — 2026-10-13

Native perception now uses score/NMS palm selection and two independent temporal
ROIs. Thirty source tests pass, including suppression and conservative ambiguous
identity geometry; those thresholds still require physical qualification.
A deterministic paired/mirrored version of the same permitted upstream positive
frame produced two accepted hands on both VPS and Pi 5. IDs 1/2 remained stable
through three repeated model frames; confidence ranged 0.9866–0.9916. This tests
native model/ROI plumbing on synthetic source-backed input, not real crossing,
occlusion, seated comfort or gesture accuracy.

The first finite supervised touch attempt captured 123 frames, processed 85,
accepted no hands and ended with BrokenPipe. Its actual log showed a final
inference/cleanup extending past the independent writer's finite deadline. The
session now avoids sending final inferred intent after expiry and accepts neutral
successful writer teardown at that deadline; early failure remains an error.
Receiving retest, rather than the code change alone, qualifies this correction.

Actual receiving five-second retests passed for both touch and mouse: panel phase
completed without error, camera opened then closed, matching virtual device was
created then removed, and microphone stayed inactive. Touch captured 70 frames,
processed 40, dropped 28, accepted zero hands; timestamp-to-intent p50/p95
106.95/138.17 ms. Mouse captured 69, processed 40, dropped 27, accepted zero hands;
p50/p95 111.86/143.16 ms. Neither run emitted a press. These no-accepted-hand
measurements do not estimate two-hand tracked latency or physical accuracy.
The receiving finite expiry correction is verified; camera-driven selection,
two-hand touch manipulation and real crossing/occlusion acceptance remain pending.

## Installation and original-permission reversal — 2026-10-13

The installation-time uinput snapshot included a temporary operator test grant.
An explicit original-ACL input now selects the retained pre-test baseline, while
preserving the installation-time ACL separately. It validates the single-device
baseline instead of guessing which earlier grants to remove.

The disposable network/mount namespace fixture passed injected snapshot failure,
retry, original content/mode restoration and selected ACL restoration on a fake
regular-file device. It loaded no model, started no service and touched no host
input device. Fictional artifacts exercise hash/package validation, not real ONNX.

Actual Pi receiving installation selected the retained original baseline. Rollback
removed installed WiggleControl artifacts and its udev rule; independent SSH
readback matched the original uinput ACL exactly. The independent panel remained
active. Fresh administrator access worked and SSH/sudo/access-file digests were
unchanged. Reinstallation restored the manually enabled package/rule, without
opening camera or microphone. Baseline custody and original snapshots remain
private. This qualifies real reversal/reinstallation, not human gesture acceptance.

## Whole-session resource measurements — 2026-10-13

Session summaries now include CPU accounting over model load, camera/gesture work,
the reaped device writer and cleanup, plus separate process/child lifetime RSS
peaks. CPU uses one-core normalization; peaks are neither summed nor described
as simultaneous process-tree memory. Thirty-one source tests pass, including
counter regression/unknown values and multicore CPU normalization.

Actual eight-second supervised trials on Pi 5, silent profile (1.5 GHz, 0 RPM):

| Mode | Fresh processed FPS | CPU, one-core % | Runtime peak KiB | Child peak KiB | Timestamp-to-intent p50/p95 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mouse | 8.30 | 63.91 | 45552 | 36848 | 107.06 / 141.22 |
| Touch | 8.15 | 64.29 | 46032 | 37328 | 110.87 / 142.03 |

Each captured 114 frames, processed 69/68, dropped 44 and counted zero stale
processed frames. Both accepted zero hands, so these measure repeated palm search,
not tracked-hand latency, jitter, accuracy or two-hand manipulation. Neither
emitted a press. Runtime peak is about 44.5/45.0 MiB; child peaks may include the
fork/exec transient, not steady-state writer usage. CPU accounting wall intervals
were 8.38/8.41 seconds; model load is included there, distinct from camera duration.

Temperatures progressed 71.6 → 73.8 → 75.45 C after compilation; firmware throttling
was not separately sampled in this trial. Panel status confirmed silent/0 RPM and
camera/microphone inactive afterward; the virtual devices were absent. These
dated no-accepted-hand trials do not qualify sustained thermal equilibrium or
the proposed browser-baseline comparison.

## VLC / XWayland native mouse delivery — 2026-10-13

A task-owned VLC Qt instance selected `QT_QPA_PLATFORM=xcb`; its PID/window and
fullscreen property were read from the real XWayland server. A synthetic 20-second
640x360 color clip contained no audio, camera or private media. The desktop and
physical inputs were preserved. Only WiggleControl's finite relative device writer
sent movement/button/wheel events; no XTest, UI-script click or application control
command was used to produce the effects.

Native click changed explicit VLC state from playing to paused. Dragging its time
bar moved the paused position from 3 to 18 seconds; wheel over its volume control
changed 0 to 20. Double-click entered fullscreen and, after a separate interval,
returned to windowed state, observed independently through the WM property.
VLC's local control interface was used only to read status/time/volume. An earlier
observer used `is_playing`, which remained 1 despite the paused UI; the corrected
qualification reads explicit `status` and confirms the paused time. X11 pointer
readback must not be assumed fresh while outside an XWayland client; the final
trial uses the task-owned fullscreen client before targeting its controls.

The writer exited successfully; its virtual device and the owned VLC process were
absent afterward, with camera/microphone inactive. Screenshots and receiving
bindings remain private. This qualifies VLC/XWayland pointer/button/drag/wheel and
fullscreen delivery, complementing Chromium native Wayland's trusted-event proof.
It does not establish camera-driven VLC accuracy or native VLC touch gestures.

## Matched static-reference comparison preparation — 2026-10-13

A camera-free browser tool independently selects the public MediaPipe Tasks
Vision 0.10.14 / float16 hand-landmarker task version 1, GPU delegate, two hands
and VIDEO mode used by the inspected existing prototype. It imports runtime/model
only on manual start and processes a bounded 1–60 frames, with no camera,
microphone, board integration or system input. No private prototype source is
copied or relicensed. The API is grounded in the official Hand Landmarker web
guide; this is SDK comparison, not full prototype/rendering performance.

The native paired reference now supports a bounded repeated diagnostic with
per-frame inference timing. Thirty frames on Pi accepted/tracked two hands:
initial call 173.38 ms, subsequent p50/p95 96.56/96.96 ms. The input is the same
paired/mirrored public reference used by the earlier positive comparison, not
a camera scene. This includes model preprocessing/inference/tracking but excludes
model load and application delivery. No physical gesture accuracy follows.

The first corrected browser invocation timed out after 90 seconds without a
result; its task-owned page closed and the finite local reference server expired.
The timeout does not diagnose model, GPU, network or delegate failure. Phase
markers are now available for the next controlled attempt. Browser/native
comparison and browser asset-byte provenance remain unfinished; no relative
speed advantage or fallback choice is established by this attempt.

## Browser baseline execution and coverage discrepancy — 2026-10-13

Phase probes showed hidden-tab attempts stayed before runtime import. The visible
task-owned tab completed thirty frames; the comparison tool now requires visibility.
The first mirrored fixture contains four hands, since the original already has
two. Native and browser selected different pairs, so those paired results cannot
establish coordinate equivalence or equal-work performance.

On the original 600x450 reference, Tasks Vision 0.10.14 requested GPU and reported
ANGLE/Broadcom V3D 7.1.7.0 OpenGL ES 3.1. The task was loaded from a byte buffer
with observed SHA-256 fbc2a30080c3c557093b5ddfc334698132eb341044ccee322ccf8bcf3607cde1.
It accepted two hands initially and finally: model initialization about 499 ms,
first inference 537.1 ms, subsequent p50/p95 69.7/74.0 ms in this warm-browser run.

The native original-reference run accepted one hand throughout: first call
124.12 ms, subsequent p50/p95 48.17/122.62 ms. These perform different accepted-hand
work and do not establish a relative speed advantage. Coverage discrepancy is
unfinished correctness work; its cause is not established. A trial increasing
palm proposals from two to eight did not change accepted-hand count and was
discarded, without lowering confidence thresholds or altering production code.
The diagnostic now includes last hand score/ROI to support the next investigation.

All inputs were permitted static images; no camera, microphone or device opened.
Owned browser targets closed and bounded local servers expired. Native timing
remains a receiving development diagnostic, not an installed production upgrade.

## Palm-stage coverage localization — 2026-10-13

Native traces now expose the last detector's proposed ROIs and current skipped
ROIs in the camera-free image diagnostic only. On the original reference there
is one proposed right-hand ROI, zero skips and accepted confidence 0.9878; the
missing hand never reaches the hand regressor. The trace vectors are bounded by
the two-proposal detector and remain in memory, not camera-session summaries.

The pinned original OpenCV implementation, with the same ONNX artifacts and
permitted image, also returns one palm and one hand (confidence 0.9904). Its raw
maximum palm score in the left image region is 0.1628, below its 0.5 threshold;
the right region reaches 0.8763. Thus the single-hand result is reproduced upstream
and is not evidence of an extra native ROI/hand rejection. It does not prove every
preprocessing detail identical or identify why the browser's different pipeline
accepts both. Model/input coverage comparison remains explicit; thresholds and
production behavior were not changed to force a pass. Thirty-one source tests
and Clippy passed after adding the diagnostic seam.

## Local offline runtime and camera-open failure — 2026-10-13

The installed runtime ran a finite four-second direct-touch session inside a new
network namespace, with only loopback present and down. Privileges were dropped
to the existing operator with its normal supplementary groups before model/camera
work. The host network, SSH and compositor namespace were unchanged. It completed
successfully: 55 captured frames, 31 processed, zero accepted hands/press intents,
and timestamp-to-intent p50/p95 113.01/139.63 ms. This proves local session operation
without an external network path, not hand accuracy or application gesture delivery.
The test used direct finite CLI ownership, not a panel-managed Stop scenario.

A separate temporary configuration pointed the camera at a nonexistent test node;
models/runtime/grants and the real maintained configuration were preserved. The
finite mouse session failed with ENOENT, processed zero frames, closed its writer
and left no virtual device. Independent panel status afterward showed camera and
microphone inactive and silent mode preserved. Fresh SSH remained functional.
Temporary test config was removed. Missing-path startup is not a physical USB
disconnect/reconnect test; that acceptance remains separately pending.

Historical summary field `injected` identifies requested writer mode; it does not
prove an emitted press, device delivery or successful session. Actual exit status,
event counters, independent device state and application observations remain the
effect evidence, especially in a failing startup.

## Controlled receiving camera loss and return — 2026-10-13

A selected camera UVC video interface was detached from its driver during a
panel-managed thirty-second mouse session. Baseline confirmed complete/free
camera and inactive mic; before detach a privileged scan of both interface video
endpoints found no other process using them. Only the owned managed runtime held
the camera. The change addressed one interface, not the USB device/hub, other
peripherals, storage or the host network. Its private recovery binding was retained
before modification and the root test restored the interface in finally cleanup.

The actual failed session log reported ENODEV after 6 captured / 3 processed
frames, zero accepted hands/press intents and about 1.18 seconds. Panel reported
failed with its child reaped; the virtual mouse was absent. Rebinding the same UVC
interface and waiting for udev restored camera presence. A new three-second
panel-managed session completed; camera/microphone were inactive afterward,
virtual device absent and silent mode preserved. Fresh SSH and panel control
worked independently after recovery.

This qualifies actual driver-visible camera loss/return and session/device cleanup.
It does not claim a physical cable/electrical hotplug test, nor held-hand accuracy:
this trial had no accepted hands or held click. Held-button release already has
distinct replay/application writer-stall/death evidence. No continuous recording
or automatic capture restart was installed.

## Visible touch-aware manipulation — 2026-10-13

The scratch receiving client now visibly scales/rotates a target rather than
displaying only numeric interpretation. Actual trusted two-contact uinput events
produced scale 1.709 and rotation 5.88 degrees; read-only computed style returned
matrix(1.70003,0.175009,-0.175009,1.70003,0,0), and a receiving screenshot confirmed
the enlarged/rotated target in the LCARS test page. Producer stall ended contacts
and Stop removed the device. No camera opened. This is application/native-touch
interpretation evidence; human camera-to-touch manipulation remains pending.

ACCEPTANCE.md now maps every DEVELOPMENT.md milestone to current evidence and
remaining physical acceptance. No full goal achievement follows from this demo.

## First human-present mouse trial — 2026-10-13

The human reported approximate hand following, movement less fluent than the
previous browser experiment, unclear workspace boundaries/calibration and an
unsuccessful pinch click. Do not mark camera-to-mouse acceptance passed. The
trusted scratch client displayed two button-target clicks and 171 pointer moves,
but these do not establish successful intended gestures or override the report.

The finite installed-runtime mouse session finished after 60.29 seconds:
855 captured, 535 processed, 232 detected, 202 tracked, three button-down intents,
318 mailbox drops and 139 stale processed observations. Fresh processing was
6.57 FPS; timestamp-to-intent p50/p95 127.52/191.71 ms; one-core-normalized
process-tree CPU 65.57%, runtime peak RSS 53744 KiB, reaped-child peak 36848 KiB.
The selected silent profile remained 1.5 GHz/fan-off; observed temperature during
the trial rose from 71.05 to 75.45 C, without a throttling sample. The microphone
remained inactive and no frames were recorded. Stale observations are a concrete
fluency concern, not proof of the entire cause or a reason to weaken release rules.

The panel process and status endpoint were running but its surface was absent
from the compositor screenshot, even after requesting expanded view. Once the
camera session ended, restarting only the panel restored its expanded surface;
returning to compact view retained visible top-right status/Stop above Chromium
fullscreen. Cause and recurrence remain unverified. Camera and microphone were
inactive afterward. Current mouse motion is relative, with no camera rectangle
corresponding to screen edges; direct-touch workspace calibration does not change
that behavior. Next qualification must make cursor/workspace behavior clear and
resolve tracked-hand timing and intended pinch reliability.

### Human-present camera-only baseline

A subsequent explicitly requested 15-second camera-only trial asked the human
to hold one open palm still. No input writer was created, microphone remained
inactive and no frames were saved. Summary: 203 captured, 132 processed, 41
detected, 37 tracked, zero button-down intents, 70 dropped, 36 stale; fresh FPS
6.28 and timestamp-to-intent p50/p95 128.81/183.72 ms. One-core-normalized CPU
63.73%, runtime peak RSS 52672 KiB, no child RSS. Silent profile remained selected;
observed temperature 69.95 C before, 72.15 C immediately after. The human then reported not attending this trial. It is invalid as a
human-controlled hand baseline; retain only its runtime evidence, not a hand
recognition acceptance claim.

The explicitly repeated 15-second camera-only trial returned 208 captured,
135 processed, 130 detected, 129 tracked, zero button-down intents, 71 dropped
and one stale observation. Fresh FPS 8.72; timestamp-to-intent p50/p95
108.28/140.15 ms. One-core-normalized CPU 48.48%, runtime peak RSS 51200 KiB,
no child RSS. This supports substantially more continuous detection in this
requested open-palm scene; it is not yet intended pinch/click or movement
acceptance. No input writer, recording or microphone activation occurred.

The human reports approximately 1 m from the TV with the camera mounted above
the display. A subsequent 20-second camera-only trial requested five deliberate
thumb/index pinches (one second closed, one second open, other fingers extended).
Actual completion of the five gestures still requires human confirmation.
The engine emitted two button-down intents without any input writer/injection.
Summary: 281 captured, 179 processed, 138 detected, 134 tracked, 100 dropped,
10 stale, fresh FPS 8.30; timestamp-to-intent p50/p95 116.25/152.97 ms;
one-core-normalized CPU 53.98%, runtime peak RSS 52176 KiB, reported reaped-child
peak 12512 KiB. The latter counter is process lifetime accounting, not proof that
an input writer existed (injected=false). Hand absence/rejection and gesture
failures cannot be localized from aggregate counters alone. No recording or
microphone activation. Do not claim a measured five-attempt success rate until
the human confirms participation.

## First live aligned-preview trial

After an explicit human ready acknowledgment, the receiving development binary
ran a 30-second camera-preview. Its local Chromium view rendered detected-hand
state and countdown at 1920x1080 with no page overflow. No input writer or frame
recording was created. Summary: 413 captured, 271 processed, 256 detected,
249 tracked, three press intents, 141 dropped and 16 stale observations; fresh
FPS 8.41, timestamp-to-intent p50/p95 111.71/152.17 ms. One-core-normalized
CPU 52.34%, runtime peak RSS 55216 KiB; child-lifetime counter 12688 KiB is not
proof of an input writer. Instructions requested slow open-hand movement;
whether pinches were deliberate requires human feedback before interpreting
those intents. Spatial alignment and perceived visual fluency remain pending
human confirmation. The installed production binary remains unchanged.

Human feedback on the first aligned preview: the camera initially activated
while the preview showed no image; the image then appeared and the skeleton
aligned perfectly with the hand. This confirms perceived spatial alignment
for this scene, not startup presentation quality, pinch reliability or general
accuracy. Initial blank duration and its cause were not measured. The three
press intents remain unclassified pending whether the human deliberately
pinched. Improve startup presentation/coordination before treating capture
activation as a visible, ready preview.

The human subsequently confirmed deliberate pinching during the first aligned
preview. Its three press intents therefore occurred in a trial containing
intentional pinches; this is not an all-open-hand false-positive test. The number
and timing of attempted gestures were not counted, so do not claim three-of-three
recognition or a measured success rate. Next: five counted attempts with visible
feedback, following a fresh explicit readiness acknowledgment.

### Counted-pinch preview trial

After a fresh explicit ready reply, a 30-second aligned preview requested five
counted thumb/index pinches, opening between attempts and waiting for the image.
The native engine reported four press intents; human completion and displayed
state perception are not yet confirmed. Summary: 410 captured, 273 processed,
270 detected, 268 tracked, 135 dropped, five stale; fresh FPS 8.83, timestamp-
to-intent p50/p95 110.78/143.15 ms, one-core CPU 49.98%, runtime RSS 52640 KiB,
child RSS zero. The live page was observed showing Pinch closing while model
processing continued. No input writer or recording was created. Do not classify
the four intents as four-of-five success until human attempt count is confirmed.

Human correction: the last requested pinch may have reached the trial deadline,
and the pinch-state labels were not noticed. Attempt timing is therefore
uncertain; do not label the four-intent result a lost fifth gesture. The preview
now adds a prominent red TESTING lamp/countdown tied to producer remaining
milliseconds, an explicit end/disconnect state, a green upper-right pressed
state and a co-located PINCH PRESSED label on the recognized thumb/index span.
No new trial was activated for this correction.

### Counted-pinch trial with conspicuous timer

After a fresh ready acknowledgment, development source b8017a9 ran a 30-second
preview with the prominent TESTING lamp/countdown and green/spatial pressed
feedback. Read-only Chromium inspection observed TESTING and 00:18 during the
run. Summary: 401 captured, 256 processed, 164 detected, 163 tracked, four press
intents, 144 dropped and 88 stale; fresh FPS 5.54, timestamp-to-intent p50/p95
132.70/207.09 ms, one-core CPU 67.49%, runtime RSS 51632 KiB, child RSS zero.
This trial had substantially more stale observations than the preceding run;
its cause is unverified. No input writer or recording. Human attempt count and
visibility of the new state feedback remain pending, so no success rate claimed.

The human reports that the pinch seems to need more positional tolerance.
Configurable close/open ratios and a receiving diagnostic 0.35/0.5 profile are
prepared for a subsequent acknowledged camera-only trial; they are not accepted
calibration yet. Default 0.25/0.4 and all release/eligibility rules remain.

The timer-visible trial's normal expiry was observed as PREVIEW DISCONNECTED
in Chromium rather than TEST ENDED. Cached metadata carried publication-time
remaining milliseconds. The server now ages this value at response time, and
the browser anchors the countdown at request start rather than header receipt;
a camera-free regression proves cached frames cannot extend the deadline. This
addresses a concrete timing discrepancy without claiming the sole observed
cause; receiving terminal presentation still needs its next live check.

### Wider-tolerance preview trial

With a fresh ready acknowledgment, the receiving development binary ran the
30-second diagnostic close/open 0.35/0.5 profile, requesting five counted pinches.
The engine reported ten press intents; exact human gesture count is pending.
Do not equate more intents with improved accuracy: duplicate/extra activations
remain a possibility until the actual attempts are confirmed. Read-only live
inspection observed TESTING, 00:20 and Open fingers first. Summary: 425 captured,
270 processed, 245 detected, 244 tracked, 153 dropped, 28 stale; fresh FPS 7.98,
timestamp-to-intent p50/p95 119.00/163.94 ms, one-core CPU 56.96%, runtime RSS
52624 KiB, child RSS zero. No input writer or recording; production config
retained its original profile. Comfort, counted recognition and relaxed-hand
false activation remain separate human acceptance questions.

Human feedback on the wider-tolerance trial: more than five pinches were
deliberate, including fast attempts, so the ten intents are not evidence of
duplicates. Perceived skeleton/movement fluency improved. Some estimated closed
poses appeared without PINCH PRESSED; their rejection cause remains unverified.
The human observed that the image became visible with about twenty of thirty
seconds remaining, making the prior trial window unsuitable as thirty seconds
of visible interaction.

Preview timing now distinguishes preparation from testing. After the page draws
and paints the first frame it acknowledges readiness; only then does the finite
test countdown begin. Preparation is limited to thirty seconds; preview test
duration is 1–90 seconds, keeping total acquisition at or below 120 seconds.
Preparation press intents are excluded from the displayed/test counter, and
aggregate summaries separately report test start, press count and elapsed time.
No page navigation starts camera acquisition. The owning CLI still requires
human readiness before starting any finite preparation/capture.

Pinch diagnostics now report concrete blocked conditions and elapsed confirmation
time without weakening gesture rules. The human requests less separation to
release/rearm; a receiving-only 0.35/0.45 diagnostic candidate is prepared for
a later acknowledged trial, preserving production defaults. Actual comfort,
intended count, relaxed-hand behavior and new timing adoption need live evidence.

### Visible-window and relaxed-release trial

After a fresh ready acknowledgment, receiving development source dc8f531 ran
the 0.35/0.45 candidate for thirty measured seconds after first-frame readiness.
The camera session lasted 37.39 seconds in total, including about seven seconds
preparation; preview_test_elapsed_seconds was 30.37 and preview_test_started
true. The test-specific counter reported seven press intents (aggregate seven);
human attempt count/comfort/block explanations remain pending. Read-only live
inspection observed TESTING at 00:28 with Open / ready and the selected 0.35/0.45
thresholds. No input writer, microphone stream or recording.

Aggregate preparation-plus-test summary: 506 captured, 334 processed, 265
detected, 264 tracked, 170 dropped and 33 stale; fresh FPS 8.05, timestamp-to-
intent p50/p95 120.54/159.78 ms, one-core CPU 57.52%, runtime RSS 55200 KiB,
child RSS zero. These whole-session counters must not be relabeled as
test-only recognition accuracy.

Human feedback on the 0.35/0.45 visible-window trial: movement/skeleton remained
fluid but pinch felt less responsive than the initial settings. The right-side
block explanations were not noticed. The human requests restoration of the
original parameters while retaining the improved test view, one further
diagnostic trial, and deferral of extensive pinch tuning to later iterations
while advancing overall development. Original source defaults are 0.25 close
and 0.4 release/open; 0.5 was a later candidate, not the original default.
The receiving diagnostic profile was restored to 0.25/0.4 and validated without
acquisition; production config was already unchanged. No new trial is started
until a fresh human ready acknowledgment.

### Invalid baseline attempt: preview navigation unavailable

The next explicitly acknowledged original-profile attempt failed to show the
image. Chromium Page.navigate returned net::ERR_CONNECTION_REFUSED; the native
summary confirms preview_test_started=false, null test counter/elapsed and
30.33 seconds preparation expiry. The human reported camera activation while
seeing the previous ended state, then a blank connection-error page. This is
invalid as a gesture trial, not a negative recognition result. Camera/microphone
closure and listener removal were verified afterward. The owned browser was
restored to the local qualification page without acquisition.

The operator route had opened the live URL without receiving HTTP readiness.
A receiving-local probe now requires HTTP 200 from the existing preview root
within twelve seconds before live navigation. A failed probe must abort that
owned attempt and restore the local page, not retry acquisition without another
human readiness acknowledgment. No raw frames are retained.

## Feedback moved to native desktop layer — 2026-10-13

Nicolás requested using the real overlay instead of maintaining a temporary
web-only hand cursor. daimon-integrators now renders the existing metadata-only
projection using GTK3/layer-shell/Cairo. The qualification page's JavaScript hand
renderer was removed; native uinput remains the application input path. The
receiving overlay passed camera-free fullscreen/transparent input delivery
checks, including real trusted click/drag/wheel and watchdog release. See
[the owning panel qualification](https://github.com/AlterMundi/daimon-integrators/blob/main/docs/qualification.md).

Visual palm position maps the mirrored camera workspace to the output, preserving
camera-pixel bone lengths. It is not the global position of the relative mouse.
Guided calibration and a qualified cursor-coincident mapping remain pending.
No new physical gesture trial or camera acquisition occurred in this increment.

## First human native-overlay trial — 2026-10-13

The human explicitly acknowledged readiness. The measured window began after
native metadata paint and ran 30.35 seconds; total capture was 31.20 seconds.
Native summary: 415 captured, 278 processed, 124 detected, 113 tracked, 135
dropped, 75 stale, 203 fresh processed (6.51 FPS); one accepted press intent.
Source timestamp-to-intent latency p50 131.10 / p95 197.59 ms, not photon-to-screen
latency. Process-tree CPU averaged 67.07% of one core; runtime peak RSS 53.4 MiB
and reaped-child peak 36.4 MiB are separate measurements, not additive concurrent RSS.

The browser observation tunnel initially required reopening, so no exact
application baseline was collected before acquisition. The first available
sample had 51 pointer moves, five downs/ups and two target clicks; the final
sample had 103 moves, six downs/ups and the same two target clicks. That establishes
52 moves during the observed interval, one additional trusted down/up pair and
no new target click. The retained event pair occurred near screen (808,106),
outside the large click target. No accuracy fraction is claimed without a
human intended-gesture count. Drag/wheel acceptance was not tested in this trial.

The terminal compositor image showed TEST ENDED / 00:00 and press intents 1.
The native process exited successfully, feedback listener and virtual mouse
were absent afterward, and the independent observer reported camera/microphone
inactive. Silent power mode and compact status panel persisted. Stop-button
termination was not exercised; this qualifies finite expiry cleanup.

Human feedback: hand flickered and appeared unrelated to the system pointer/click
representation. The separate mirrored-palm versus relative-pointer mapping is
known; the strict visual freshness cutoff is a hypothesis for flicker alongside
75 stale frames, not a proved exclusive cause. The human requested calibration
first. Issue #2 owns the shared mapping/manual calibration and later prayer entry.
The earlier goal was cleared; no additional acquisition or replacement goal
followed this request.

## Absolute mouse output precondition — 2026-10-13

A separate finite `absolute-stream` adapter uses Linux evdev/uinput ABS_X/Y,
normal mouse buttons and wheel axes. Relative mode stays separate; writers
reject position intents from the other mode. Both reuse bounded JSONL input,
finite expiry, held-button teardown and the 150 ms producer watchdog.

On the receiving Pi/Wayland fullscreen 1920x1080 Chromium, read-only trusted
application events observed six requested normalized positions: (0.25,0.35),
(0.5,0.5), (0.75,0.75), (0.01,0.01), (0.99,0.99), (0.25,0.4). All were within
0.03 pixels of normalized-coordinate times output dimensions. A real target
click arrived; a held button released after a producer stall; focus remained
in Chromium. The writer exited successfully and its virtual device was absent
afterward. Camera/microphone stayed inactive; no inference ran. This qualifies
an absolute mouse route on this single-output compositor, not a human hand
profile, multi-output routing or completed calibration/overlay integration.

The source calibration core now reuses the validated normalized workspace map.
Manual stages sample left/right/top/bottom bounds with up to 24 fresh points,
at least ten points across 750 ms, central stability checks and median selection.
Tests reject stale/duplicate/future data, narrow/reversed bounds, cancelled/short
calibration and invalid profile version/output; one extreme outlier is tolerated.
Review/candidate generation alone does not adopt a profile. Native controls,
corner verification, profile persistence/rollback, perception/output/overlay
wiring, visual continuity and prayer activation remain unfinished.

## Source corner verification, profile rollback and command seam

Camera-free Rust tests now verify four ordered corners with 500 ms continuous
fresh dwell, and reject stale/lost/invalid/out-of-order observations as completion
evidence. No button gesture is required to verify a corner. Profile storage tests
exercise first adoption at 0600, previous-profile preservation, rollback/swap,
invalid profile refusal, symlink refusal and preservation of corrupt current data.

Loopback HTTP tests demonstrate calibration controls disabled by default,
same-origin requirement, single-pending-command backpressure, and consumption
of each named control. The response explicitly acknowledges queueing, not
producer execution/profile adoption. Twenty native-feature library tests and
all-target Clippy passed. These components have not been wired to the receiving
UI/session or qualified with physical hands; no acquisition took place.

## Camera-free prayer-like candidate preparation

Fictional pixel-space hand skeletons verify one activation after a one-second
continuous fresh hold, stable identity under array reordering, no repeated
activation while held or after occlusion, and rearming only after 400 ms of
observed separation. Disabled, stale/future, low-confidence, ambiguous, incomplete,
folded-finger and uncertain/same-handed observations do not activate. Tests and
native-feature Clippy passed. These are algorithm/state tests, not model accuracy.

The 2-D heuristic cannot establish physical palm contact. Native multi-hand
ambiguity and nearby-ROI suppression can reject a real joined-palm pose; no real
pose was acquired. Prototype thresholds, native integration, performance and
human gesture acceptance remain unqualified. The detector is neither wired nor
enabled in the receiving manual calibration release, and activates no sensors.

## Optional native prayer-entry wiring

The source session can opt into the existing multi-hand perception path and
convert native pixel landmarks, confidence, handedness and tracker IDs into the
prayer candidate. Requests are gated by an active accepted profile, visible
measured-window start and finite deadline, and release/rearm ordinary input
before neutral calibration. Model detection, gesture candidate and adoption
remain distinct. A gesture cannot accept a profile or replace active review.

The existing camera-free lifecycle test now proves no-profile/modal rejection
and accepted-profile prayer entry; CLI tests prove legacy opt-in defaults false
and unsupported session mode rejection before fictional unavailable inference
paths are opened. Full native-feature tests and Clippy passed. No live pose,
model coverage, paired latency or human acceptance is established by these tests.
The prepared receiving manual configuration has not enabled this option.

## Distracted manual trial and finite step preparation

The acknowledged native trial expired after 90.35 measured seconds (91.28 total),
with 1185 captured / 825 processed / 212 tracked frames, 225 stale observations,
6.57 fresh processed FPS and p50/p95 source-to-intent age 130.5/166.5 ms. It produced
zero press intents during neutral calibration, saved no profile and closed its
camera/device/listener. The human subsequently reported distraction; this is
lifecycle evidence, not physical calibration or pinch acceptance. The human also
reported less fluid native skeleton presentation than the earlier web preview.
Those trials were not a controlled comparison, so the cause remains unproved.

Confirmed bounds now persist separately in an owner-only atomic `.draft` beside
the receiving profile. Finite sessions resume the next unconfirmed bound with
empty samples; a completed draft restarts all four corner checks. Expiry never
adopts it. Start/Cancel discard the draft; Accept requires fresh corner review.
No images, pending samples, dwell timers or verified corners persist. A changed
output, corrupt/invalid/noncontiguous bounds or symlink is refused. Tests cover
restart without adoption, empty resumed samples, incompatible output and Cancel.

Delayed detected frames no longer erase a stable sampling window by themselves;
only fresh observations enter it. Actual tracking loss or a 500 ms fresh-sample
gap clears pending samples. Input freshness, pinch thresholds and corner checks
remain unchanged. This fixes source-level sample starvation, not measured gesture
accuracy. Native UI polling/drawing scheduling was reduced independently; physical
fluency and idle/resource impact still require receiving qualification.

Next trial: one comfortable bound at a time, explicit human verbal confirmation
before operator Next, then Stop. Resume in a new finite ready-acknowledged session;
never automatically restart acquisition to complete the draft. The human must
remain in the same seated position until all bounds/corners are checked; if they
move, explicitly restart the draft. No physical mouse is required.

## Diagonal calibration source qualification

Camera-free tests cover two-axis stability, reversed/narrow diagonal rejection,
median positions, 5% interior-margin mapping, partial and full draft resumption
without observations or automatic adoption, and unchanged fresh corner checks.
The loopback control regression covers the new same-origin diagonal route with
existing single-command queueing. Twenty-eight native library tests and all-target
Clippy passed; the rest of the native suite also passed before adding these two
new library regressions. Real diagonal comfort, precision and all-corner reach
remain unqualified until the next acknowledged physical trial.

Sampling/review now also require the native detector's existing all-landmarks-in-view
geometry check, independently of pinch eligibility. Previously calibration checked
confidence and age only and could collect a clamped palm position while some
landmarks were outside the image. This is a source finding, not proof of the cause
of the reported corner barrier. In-view/out-of-view reference geometry is covered
by the existing native vision test; no new camera trial was opened.

## Saved review acceptance independent of camera lifetime

The camera-free runtime regression now verifies that completing all four fresh
corner dwells persists its candidate without adopting it, survives Runtime drop,
refuses an altered confirmed draft, and accepts explicitly through the saved
verification path with previous-profile rollback preserved. Missing completion
proof is rejected. Saved candidate equality includes output and full validated
workspace. Adoption is an operator action, not a sensor event. This prevents the
operator chat round-trip from requiring a running camera after the corners finish.
The completed human-reported receiving trial predates this persistence; it does
not establish a saved producer result or an accepted physical profile.

## First receiving two-diagonal profile accepted — 2026-10-14

After the human moved the camera, old positions were preserved as local evidence
and explicitly restarted. Upper-left and lower-right were confirmed in separate
human-ready finite sessions. One failed lower-right advance had too few fresh
samples; it adopted nothing and was repeated with operator sampling wait.

On the new corner-review trial the native producer reported all four corners
verified and saved, with 53.17 seconds remaining. The human independently said
"Terminé". Operator Stop was acknowledged; the verified candidate existed on disk.
The explicit camera-free accept-calibration command then accepted the unchanged
draft/candidate, and receiving observation showed camera/microphone inactive.
Output is 1920x1080; candidate uses the implemented 5% interior margin. Exact
receiving coordinates/access remain private. This establishes one physically
completed calibration/adoption, not click/drag accuracy or broad seated comfort.
Pinch, drag, hand loss, Stop during held input, fluidity and prayer remain pending.
The old camera-position draft is evidence only and is not the new active profile.

## Guided preparation and native display check

Guided camera-only source ef9af5b / native UI 8722e69 built on receiving ARM64 Rust
1.85.1 and were installed as reversible user-local releases, preserving original
production executables/services and the accepted profile. A ten-second fictional
metadata fixture on actual Wayland TV produced separate native PAINT_ACK 0 and
PAINT_ACK 1 receipts. Compositor capture showed HOLD STILL, its step countdown,
position guide, insensitive manual controls and the independent compact panel
within 1920x1080. Fixture time was shortened solely for this camera-free UI check;
real producer timing remains 15 seconds move + 5 hold per pose, up to 20 per corner.
Camera/microphone observed inactive afterward. These receipts qualify UI routing
and layout only, not actual camera startup, guided comfort or physical accuracy.
Thirty-two native library tests, full native suite, and all-target Clippy passed.
Guided physical acceptance awaits Nicolás's fresh ready acknowledgment.

## Exact JSON coordinate round-trip correction

The first one-pass receiving trial displayed completion and retained a verified
candidate, but camera-free adoption refused because reconstructed draft and saved
candidate differed under exact JSON value comparison. Receiving Python decimal
reconstruction reported zero differences; no accepted-profile replacement occurred.
A camera-free regression over fictional diagonal coordinates reproduced the Rust
failure at sample 3: 0.20031500000000002 versus 0.200315 after JSON persistence.

The existing pinned serde_json 1.0.145 float_roundtrip feature corrects float
parsing without weakening exact candidate matching or changing dependency version.
That same 999-case persistence regression passes with the feature. Camera remains
off; retained real draft/verification are preserved for corrected explicit adoption.
This is a serialization correction, not a new hand measurement or relaxed gate.

Corrected receiving Rust 1.85.1 build at 9dce3da accepted the original retained
verified candidate through the explicit camera-free command. Previous accepted
profile exists in rollback storage. New/old workspace ratios are approximately
1.80 horizontal and 2.96 vertical; no camera repeat occurred. Receiving observer
reported camera/microphone inactive afterward. Human completion plus retained
fresh all-corner proof and successful exact adoption establish one physical guided
calibration. User-perceived timing, pinch/drag precision, held-input Stop, tracking
loss, visual continuity and prayer remain pending; no broad goal completion claim.

## Calibrated pinch/drag trial: application delivery observed, drag pending

Nicolás acknowledged a finite 90-second calibrated native-input trial with the
accepted larger workspace, requesting three orange-button clicks and one pink
block drag. Task-owned qualification page was reloaded and observed at zero
moves/down/up/click/drag/wheel counters before acquisition, 1920x1080. Producer
reported active profile, testing, fresh age118.2ms at startup. Human said completed;
by final control read the listener was closed, and observer showed camera/mic off.

Application readback: 758 pointer moves, seven trusted target clicks, seven downs
and seven ups, zero completed drags and wheels. All retained button pairs occurred
inside the orange target; no down was observed over the pink block. Final pointer
was inside the block, which remained at its initial rectangle. No held-button
state remained in retained events. These are application observations, not seven
confirmed intentional pinches: requested count was three, human extra attempts vs
unintended repetitions still needs clarification. No producer terminal metrics
were recovered in this turn. Drag is not qualified by this test; no additional
capture started. Keep pinch stability/drag/loss/Stop/visual/prayer gates open.

Human acceptance correction for the preceding click/drag trial: Nicolás explicitly
reported pinch/drag unusable, only a few visible pinches after switching to the
left hand, and much better previous web behavior (informal estimate ~90%). The
seven application clicks are delivery evidence only, not intentional accuracy or
functional acceptance. Keep the human feedback and zero completed drags together.
The native skeleton repeatedly switched colors; source UI ties whole-hand color
to the consumer's 150 ms input-freshness threshold, independently of actual
recognition. This establishes its rendering rule, not the cause of gesture failure.

Next diagnostics add bounded aggregate processed-frame counts by gesture state
and block reason, plus existing hand ID/handedness estimate/in-view metadata. No
raw recording, threshold change, model change or camera activation. Next human
trial should isolate perception/gesture state camera-only before another OS drag
qualification, retain terminal summary on the receiving host, and wait fresh ready.

## Camera-only recognition diagnostic and persistent pinch feedback

A fresh human-ready, finite 40-second right-hand trial requested three slow
pinches, holding the last for approximately three seconds. No OS input was
injected. The retained summary reports 381 detections out of 390 processed
frames, two press intents and six total pressed-state frames. Block counts:
72 invalid hand, 26 stale frame, six tracking lost; 104 total not-eligible frames.
Neither three intentional pinches nor a sustained three-second hold is qualified.
An earlier unattended trial is runtime evidence only, not human acceptance.

The specific invalid field was not retained by that version. New diagnostics
count the first failing hand field and publish it in metadata, reusing the same
validation predicates; confidence, geometry, thresholds and freshness behavior
are unchanged. This prepares diagnosis, not a fix for gesture reliability.
The native overlay now retains a recognized-onset expanding/fading pulse for
700 ms, independently of brief PINCH state text. Camera remains off during
source/replay preparation; a new physical trial still requires fresh human ready.

## Held-color human trial — 2026-10-14

Nicolás acknowledged a 40-second camera-only trial of onset pulse and held-green
skeleton, requesting three right-hand pinches and a three-second final hold.
He reported never seeing the hand remain green: recognition appeared absent or
very brief. Summary: 390 processed, 348 detected, one press intent, ten pressed
frames; blocks stale_frame43, tracking_lost33, invalid_hand2 (palm_width),
open_before_press39. No OS input injected; camera/mic inactive after completion.
Color implementation was separately qualified with synthetic held/released
Wayland frames; sustained physical pinch remains failed/unqualified.

Source inspection confirms stale observations cancel held input and clear arming;
fresh closed fingers cannot rearm until an open-ratio observation. Timing p50
123.828 ms / p95 154.038 ms crosses the configured 150 ms stale cutoff. This is
a concrete cancellation/rearming mechanism and measured timing conflict, not
proof that every held-input release in this trial had that cause. Aggregate
state/block counts do not retain release chronology. Retain per-release reasons
and timing before choosing a gesture or freshness change; do not hide actual
release by keeping visual green. The previous trial's 72 invalid-hand frames
cannot be retroactively assigned to palm_width from this later trial's two.

Baseline comparison correction: the earlier WiggleControl web camera preview
(tools/preview.html, introduced at 5cdf9e6) consumes Rust-produced pinch_state;
JavaScript draws state and never recognizes gestures. Comparing that revision
with bb3026b shows unchanged pinch entry/hold/release predicates and perception
eligibility, with diagnostics/accessors added. Thus replacing native gestures
with this page's logic would not recover another recognizer. Investigate runtime
and configuration regression against that baseline. ProyectoRPi/Barehands is a
separate browser recognizer; do not conflate its implementation or model pipeline
with the user-qualified WiggleControl preview. No new capture or parameter change
occurred during this comparison.

## Selected gesture freshness window — 2026-10-14

Nicolás requested a less brittle late-frame window. Session config now accepts
freshness_ms (50..1000, legacy default150); the selected receiving diagnostic
configs use300. Gesture source-age rejection, receive-gap expiry, summary freshness
and published metadata use that setting. Native and web consumers read the same
budget; pointer/contact writer CLI accepts an optional bounded WATCHDOG_MS and
sessions pass the setting, preventing an independent writer cutoff at150.
Standalone writer defaults and replay defaults remain unchanged. Calibration
sampling/corner/prayer validity retain their separate150 ms rules; accepted
profile and pinch close/open .25/.4 are unchanged.

A replay regression qualifies a200 ms-old continuous held observation at the
selected300 ms window, release at the exact300 ms receive gap and neutral closed
reacquisition. Consumer regression qualifies matching300 ms display expiry and
legacy150 ms default, rejecting unbounded budgets. Config-check accepts300 and
rejects1001 without opening devices. Loss/ineligibility/Stop still release; this
change tolerates processing delays and does not infer a hand from missing data.
Physical sustained-pinch acceptance still requires a new acknowledged finite trial.

## First human-ready 300 ms camera-only trial

Nicolás acknowledged forty measured seconds, requesting three right-hand pinches
and a three-second final hold. Live metadata confirmed freshness_ms300, testing
and input_enabledfalse. Summary:608 captured,387 processed,370 detected,363
tracked, seven press intents and73 pressed-state frames; stale0. Blocks:
hand_not_engaged36, invalid_hand9 (palm_width), tracking_lost17, open_before_press3.
Timing p50/p95 120.189/154.254 ms, elapsed41.181 s, measured test40.335 s.

Previous150 ms trial had one intent, ten pressed frames and43 stale. This
comparison establishes reduced stale rejection and more held-state frames, not
seven intentional pinches or a continuous three-second hold. Human extra attempts
versus split/repeated activations and perceived sustained green remain pending.
No OS input or recording; microphone stayed inactive and a later fresh observer
confirmed camera inactive (the immediate post-exit observer was still cached open).
Do not infer that all remaining posture/tracking failures were solved by timing.

Human feedback for the first300 ms trial: sustained green remained fixed while
the pinch was recognized as held; held-state color is accepted for that observed
case. Nicolás made multiple deliberate attempts, initially unsuccessful, then
pinches began working after rotating his hand sideways. Front-facing recognition
was worse despite visually correct skeleton tracking. Seven intents therefore
are not evidence of seven unintended activations, and no intended success rate
is established. Recognition across orientation remains unresolved.

Source currently computes2D thumb/index distance divided by projected MCP5–17
palm width and separately gates engagement on finger extension/full in-view.
Thus failed front-facing pinches may involve geometry or posture eligibility;
the aggregate36 hand_not_engaged frames do not identify specific attempts.
The pinned OpenCV hand model's reference adapter consumes screenXYZ plus a
worldXYZ output; current Rust consumes only screenXY and discards depth/world
coordinates. Primary reference:
https://github.com/opencv/opencv_zoo/blob/47534e27c9851bb1128ccc0102f1145e27f23f98/models/handpose_estimation_mediapipe/mp_handpose.py

Next corpus should include frontal/sideways poses and preserve estimated3D tip
distance, stable anatomical scale and posture gates separately. Compare a
3D proximity/contact estimator with current2D ratio on labeled held/released
intervals, including unintended overlaps. Model-derived3D is an estimate, not
a measured physical contact; qualify output order/units before adoption. No
new model inference is intrinsically needed to retain existing outputs. Keep
hysteresis and loss/Stop release; do not change multiple gates without replay
evidence. No geometry change or recording was started in this turn.

## Experimental metric3D pinch and recorded trials

Human selected direct estimated3D thumb/index tip distance. The exact pinned
model already emits worldXYZ as its fourth output; Rust now retains/validates
that63-float output and computes Euclidean distance without projected palm width.
Opt-in session pinch_3d attaches that measurement to the shared gesture engine.
Selected receiving close/open thresholds are10/30 mm (user correction);300 ms
freshness and40 ms confirmation remain. The legacy2D mode remains available.
Confidence, full-hand visibility and existing posture eligibility still apply;
legacy2D scroll interpretation is disabled during isolated3D trials. Estimated
world units follow the pinned upstream adapter, not measured physical distance.

Replay tests cover metric entry/hold/release independent of tiny projected palm
width, invalid metric rejection and hysteresis. Perception test covers depth
separation and rotated-axis invariance. Indexed capture and label-validation
regressions are camera-free. Physical orientation/metric accuracy, continuous
recording under load and human annotation usability still need acceptance.

Receiving user-local producer9995a8d / native UI1bbcb22 installed reversibly.
Config-check confirmed3D enabled, close10 mm, open30 mm, freshness300 ms and
recording enabled. VPS/Pi camera-free inference on the same permitted reference
returned21 world points, confidence0.987821 and distance72.8093 mm (difference
approximately4.1e-8 m); this qualifies output extraction compatibility, not pinch
accuracy or physical metric calibration. Tests/Clippy and CI38527771488 /
38527782291 passed.

A fictional90-frame receiving take qualified camera-free review playback
(frame22 at733.326 ms after approximately750 ms), stepping, press/hold interval
creation and saved labels333.33..1666.65 ms with revision retained. Actual TV
viewport1920x1080, body height999 px; all controls fit and preserve the compact
panel. Synthetic marks are fixture evidence only, never human truth. Camera/mic
observed inactive after preparation. Future acknowledged human trials will retain
video and inference metadata; no real take was acquired during this increment.

## First recorded3D trial; next selected35/60 mm window

Fresh human ready activated forty measured seconds with10/30 mm thresholds,
freshness300 ms, recordingtrue and input_enabledfalse, confirmed live. Summary:
604 captured,385 processed,368 detected, one press intent,48 pressed frames,
zero stale/invalid; blocks hand_not_engaged7 /tracking_lost17. Timing p50/p95
122.204/154.820 ms. These are recognition observations, not labeled success rate.

Private take contains604 indexed raw frames /385 observations /75,954,432 video
bytes covering40.230469 s. Final indexed offset matches video size; source times
strictly increase. Saved config retains original10/30 mm. Nicolás selected35 mm
close /60 mm release for the next trial; config-check accepted,300 ms freshness
and recording retained. Both earlier configs are preserved locally for rollback.

Human had no keyboard/mouse and explicitly deferred playback/annotation. The
task-owned review server was terminated without navigating to the take; raw video
remains retained and unlabeled. Do not infer annotation acceptance or convert
predictions into ground truth. Fresh observer confirmed camera/mic inactive.
No second capture starts until a new explicit ready reply.

Fresh human-ready35/60 mm camera-only trial: forty measured seconds, recording
and thresholds confirmed live.608 captured /389 processed /365 detected, nine
press intents /91 pressed frames, zero stale/invalid frames and24 tracking_lost
frames. No posture-block frames in this take. p50/p95 age123.024/153.955 ms.
Private take verified608 indexed frames /389 observations /77,271,232 bytes /
40.230257 s, with35/60 config preserved. Human proposes45/65 mm to catch remaining
pinches; intentional count/hold continuity and false activations remain unlabeled.
Next selected config45/65 passes config-check; old35/60 configs retained. Camera
and microphone observed inactive. No next capture without fresh human ready.

Fresh human-ready45/65 mm recorded camera-only trial:40 measured seconds,
thresholds/recording/no input confirmed live.607 captured /389 processed /373
detected, seven press intents /53 pressed-state frames, zero stale/invalid.
Blocks hand_not_engaged133 /tracking_lost16 /open_before_press64. Source age
p50/p95 123.043/153.484 ms. No human event labels or acceptance yet.

Raw take verified607 indexed frames /389 observations /76,890,464 bytes over
40.230221 s; original45/65 config preserved. Of133 engagement-blocked frames,
65 had estimated gap<=45 mm; all65 reported hand_in_viewfalse. Source gate
requires every21 projected landmark in frame, so these were visibility blocks,
not evidence that the distance threshold was too small. This does not identify
which landmark was outside or establish physical ground-truth pinch. Capture
ended and fresh observer reported camera/microphone inactive; playback remains
deferred at the human's request. Keep visibility and distance failure separate.
