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
