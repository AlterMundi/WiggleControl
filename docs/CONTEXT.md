# Project context — 2026-10-13

## Active controller-evolution goal —2026-10-15

Nicolás authorized a new native Cuevanico goal to evolve the complete camera
controller through simulation, perception/model comparisons, useful modes,
multi-participant input and macros. The goal is active; it does not complete or
resume the earlier blocked Shared Resources goal. The maintained entry point is
[ROADMAP.md](ROADMAP.md), with implemented versus pending modes in [MODES.md](MODES.md)
and measured results in [the evolution report](research/controller-evolution-2026-10.md).

Implemented in this increment: opt-in speed-adaptive Rust cursor filtering,
2016-case analytic benchmark with explicit development/held-out seeds and a
reproducible profile selector; camera-free temporal image replay through actual
perception/direct-touch meaning; open-hand single/pair3D motion controls; bounded
typed macro hold/rearm/cooldown replay. The current paired render checks disjoint
evaluated skin-depth intervals; its earlier close-depth stress scene remains
unqualified. MediaPipe1.1.1 CPU was actually compared on the same retained images.
Native optimized processing was30.90ms median on the VPS versus64.19ms for Tasks'
sequential pass; this is not a Pi benchmark or universal backend ranking.

The TV's owned experiment browser now shows measured filter tradeoffs, actual
moving-reference processing, retained completion and explicitly saved replay.
No new camera/microphone/input session, runtime binary, live thresholds, power
mode or saved calibration was deployed by these experiments. The selected
45/65mm receiving pinch experiment remains as before. The full-workspace adaptive
candidate meets particular analytic budgets; its calibrated expansion fails them.
Real noise,
anatomical contact truth, occlusion ownership, physical comfort and application
acceptance remain distinct work. Macro visual classification and actual native
action adapters remain pending; replay action intents have `executed:false`.

Current source correction: live direct touch now shares the configured2D/3D
pinch conversion with pointer and image-sequence replay; a contradictory-metrics
two-contact regression verifies both modes. The calibrated5760-case benchmark
reproduces exactly and exposes failure of all tested filters against the fixed
rest/fast/fine budgets. The TV retains the calibrated scores and explicit failures.
No runtime/configuration adoption follows from that ranking. See the evolution
report's calibrated-workspace and direct-touch sections for evidence and limits.

Older dated checkpoints below preserve history; they do not override this roadmap.

Latest visible-research checkpoint (2026-10-15): the offline workbench on
Cuevanico now follows actual VPS rendering/inference and retains completion or
delayed/error state, without sensors or a Pi model runtime. Nicolás's TV review
identified the negative-pitch fixture as body-occluded; the near scene's visible
surface was3.718cm away while the wrist marker was61.852cm away. Those three
inputs are not valid detector-failure evidence. A hand-only nine-angle yaw sweep
and a separate60-frame convergence comparison completed; see
[POSITION-BIAS.md](POSITION-BIAS.md#human-visible-review-corrects-the-render-interpretation--2026-10-15)
and [visible tools](../tools/synthetic/README.md#visible-experiments). This does
not deploy a pinch correction or resume the old physical-acceptance goal.

Nicolás selected the name **WiggleControl**, executable `wigglecontrol`, after
checking naming collisions. He requests a lightweight Rust camera-driven input
device usable over existing software as a mouse and later a multitouch screen.
He explicitly requested research before implementation and then creation of
AlterMundi/WiggleControl with the plan persisted here.

Cuevanico is the receiving Pi 5 with C920e and labwc/Wayland, now at 1080p60. Its Rust
status panel is deployed independently. Source-backed discovery lives in Shared
Resources resource `host-cuevanico`, docs/integrators/cuevanico.md; exact local
routes, operator credentials and media stay in that receiving private overlay.

Ani/Oliva's ProyectoRPi/Barehands prototype has one documented successful
physical pinch/selection passage on this Pi. That is the comparison baseline,
not a universal input adapter or sustained performance benchmark. HarMoCAP has
useful smoothing, timing, validity/replay methods; body pose alone is not finger
recognition. Do not copy their private history or assume permission from a repo.

Research completed in daimon-integrators at commit `208a698` and carried to this repository
with provenance. Read-only receiving inspection found labwc 0.20.2/wlroots 0.20.2
and a root-only uinput node; live virtual device creation is not qualified.
Current inference library heads have compiler requirements newer than the
panel's Rust 1.85 baseline; choose compatible pins or justify a separate toolchain.

Milestone 1 now has a Rust replay/gesture engine, observation contract v1, bounded
JSONL CLI, fictional fixtures and 14 passing behavioral/CLI tests on VPS Rust 1.95.0 and receiving ARM64 Rust 1.85.1.
Pinch confirmation, hysteresis, loss/stale/stop release and neutral reacquisition
are tested. Filtering is a timestamp-based exponential baseline, not yet a
human-qualified smoothing choice. Scroll mapping remains pending. CI targets Rust
1.85.1; its first run passed compilation/tests but failed an older-Clippy format
collection lint; the test was corrected. Corrected CI run 38387111298 passed on Rust 1.85.1.
The evdev virtual mouse adapter now
creates an ID_INPUT_MOUSE=1 device on cuevanico and removes it after finite expiry.
The temporary receiving uinput ACL applies only to the existing operator; original
ACL retained for reversal, no persistent grant installed. No camera or input
injection occurred. Application delivery/watchdog/SIGKILL qualification is pending.
Multitouch is a later distinct contract, not a promise of zoom in every app.

Owning implementation tracker: [WiggleControl #1](https://github.com/AlterMundi/WiggleControl/issues/1).
Historical discussion/parent: [daimon-integrators #3](https://github.com/AlterMundi/daimon-integrators/issues/3). Keep owning issues and this
context aligned as implementation progresses; research is not implementation.


## Native session integration checkpoint

The selected experiment uses ort rc.10 / ONNX Runtime 1.22 CPU, not the failing
tract palm candidate. Both models run on Pi 5; native geometry matches one
positive upstream frame within median 0.45/max 2.50 pixels against OpenCV.
Perception/model confidence is not human gesture accuracy. Latest-frame 640x480
MJPEG, monotonic driver timestamps, finite stop and no recordings are implemented.

The installed relative virtual mouse reaches Chromium through actual Linux
input, including drag/scroll/watchdog/process-death release. Integrators owns
manual `input SECONDS` preparation/start/Stop; input differs from recording.
A private task-owned browser page is available for hand qualification; one real
camera-only session accepted no hands. Do not infer human absence from that.
Source/account/private bindings remain in Shared Resources. No models at boot.
Next: direct touch replay/typing, additional application acceptance and a
human-present gesture session with real accuracy/comfort measurements.

Direct type-B touch now has a separate bounded intent contract and two stable
slots. Real trusted two-contact events reached Chromium native Wayland; the
touch-aware scratch page interpreted scale/rotation, and producer stall released
both contacts before device removal. This qualifies the writer, not camera-to-touch
or hand crossing. See TOUCH.md and QUALIFICATION.md for exact limits. The source
panel integration was published at daimon-integrators 662ff69 with successful
Actions run 38412337990; receiving start/Stop evidence is recorded independently.

The two-hand gesture/contact layer now reuses the pointer pinch engine, preserving
separate contact ownership, open-before-press and freshness. It has a bounded
camera-free touch-replay seam and fictional fixture. Producer-declared identity
ambiguity neutralizes contacts; actual native ambiguity detection/association
and camera-to-touch integration are still next work, not implied by these tests.

Native `run-touch` now joins score/NMS-selected palms, independently tracked hand
ROIs and the touch gesture/device path. `camera-check-touch` has no device output.
Paired public-reference inference detected/tracked two stable IDs on VPS and Pi;
actual physical crossing/occlusion, hand accuracy and comfort remain unqualified.
The panel offers finite `input SECONDS touch`, with mouse as the default. The
first expiry trial exposed a BrokenPipe deadline race; actual retest evidence
belongs in QUALIFICATION.md before claiming receiving completion.

Actual finite mouse/touch expiry retests passed; installation/permission reversal
and reinstallation now also passed on Pi. The maintained INSTALL.md explains the
selected original uinput baseline: the initial install snapshot included the
temporary test grant and cannot by itself establish full pre-test permission
reversal. Real restoration matched the retained original ACL, preserved access
and the independent panel, and reinstallation opened no sensor/model session.

Whole-session CPU and separate runtime/child RSS peaks now accompany fresh FPS
in bounded summaries. Actual receiving mouse/touch trials completed, about 64%
of one CPU core and 45 MiB main-process peak under the silent cap. No hands were
accepted, so tracked-hand cost and accuracy remain pending. Source-qualified
counter semantics and exact context are in QUALIFICATION.md, not a speed claim.

VLC Qt on XWayland now has real native virtual-mouse delivery proof: pause click,
time-bar drag, volume wheel and fullscreen/windowed double-click. It complements
Chromium Wayland, using read-only observations and no application-dispatched input.
Owned test process/device removed afterward; no sensor opened. Physical hand
accuracy, comparative tracker scenes and comfort remain separate pending gates.


Palm-stage localization: native and original OpenCV with the same ONNX/image
both return one palm. Original left-region score 0.1628 is below threshold 0.5;
native has one proposed ROI and no extra skip/rejection. Browser pipeline accepts
two, but this is a model/input coverage difference reproduced upstream, not a
proved Rust-port rejection bug. No thresholds/production perception were changed.
The image diagnostic now exposes bounded last-palm/current-skip ROIs. Actual
human camera accuracy, model/input coverage assessment and comfort remain pending.

Installed runtime completed inside an offline network namespace after dropping
to normal operator identity/groups; the host network/access stayed unchanged.
A missing-camera startup failed as expected and released its writer/device, with
real camera/mic inactive afterward. These qualify offline local lifecycle and
startup failure cleanup, not physical USB hotplug or human gesture acceptance.

Controlled receiving driver loss/return now also passed: selected UVC interface
detached only after confirming no foreign camera endpoint users, owned session
failed with actual ENODEV and removed its virtual device, then rebind plus a new
finite session restored normal operation. Host access/other peripherals remained
intact; camera/mic inactive after test. This covers driver-visible disconnect and
recovery, while electrical hotplug and human held-hand behavior remain distinct.
See exact observations and summary-field semantics in QUALIFICATION.md.

## Source improvements while physical acceptance is paused

Nicolás is unavailable for acceptance while the integrator room is occupied by a
sleeping human. Work remains on the VPS: no receiving build, deployment, camera,
microphone, synthetic input or desktop changes. The native goal stays paused.

The native config now exposes the existing direct-touch workspace mapping as
optional `touch_workspace`, validated before acquisition. Existing configs preserve
their default; example bounds are not a qualified comfort calibration. Relative
mouse gain remains separate. Camera-free regressions exercise calibrated contact
edges/ownership and invalid/legacy configs.

Session teardown now retains writer-wait errors until the camera thread has been
joined and rejects late nonzero writer exit instead of silently returning success.
Expected SIGINT/SIGTERM writer exit requires a matching parent interruption;
SIGKILL and nonzero exit remain failures. Tests cover these terminal outcomes
without opening sensors or input devices. Deployment and physical acceptance of
these changes are deferred until the room is available.

The camera-free `config-check CONFIG` command now permits validation before a
session, without opening paths referenced by the config or initializing inference.
It reports bounded settings, not access routes. Native CLI help now lists the
available image/camera/run commands and their effects. A CLI regression uses
nonexistent camera/model/runtime paths to verify schema validation is independent
of acquisition; invalid workspace bounds return failure without a success report.
This source-only increment also remains undeployed while the room is unavailable.

TV-first correction: the human reported that the qualification page exceeded the
actual display. Primary panels now use a viewport-height grid with a reserved
right strip for the compact overlay. Real Chromium at 1920x1080 reports page
scroll dimensions 1920x1080; all four dynamically populated panels and footer
fit, and a compositor screenshot confirms the mini panel remains visible. Only
the explicit scroll exercise scrolls internally. AGENTS.md records TV-first
layout, viewing-distance readability and dynamic viewport checks for future work.
No sensor or input session was opened for this layout correction.

Human feedback requires live spatial feedback: the native preview now displays
the mirrored frame with correctly co-located hand skeleton, thumb/index pinch
and palm anchor, plus upper-right engine state/confidence/ratio and timings.
It reuses the finite camera-only path, creates no input writer and retains no
frames on disk. Existing gesture decisions/thresholds remain unchanged.
The human corrected trial coordination: prepare and describe one test, then
wait for an explicit ready reply before starting each acquisition. Earlier
foreground presence never substitutes for the new readiness acknowledgment.

The first explicitly acknowledged live preview received human confirmation
that the skeleton aligned perfectly over the hand. The initial camera-on /
blank-preview interval remains a presentation issue with unmeasured duration
and cause. Pinch intent acceptance remains pending; do not reinterpret aggregate
press counts as confirmed deliberate gestures.

Pinch comfort feedback led to optional validated close/open ratio config;
0.35/0.5 is a diagnostic candidate, with default 0.25/0.4 preserved. Physical
intended/relaxed-hand acceptance remains pending. Preview timer metadata now
accounts for the age of its cached snapshot to avoid extending countdown expiry.

Live-trial correction: the human saw the initial image only with ~20 seconds
remaining. The preview now waits for a first-painted-frame acknowledgment to
start its measured test, with finite preparation and total capture bounds.
The reported ten intents included deliberate extra/fast pinches and are not
a counted five-attempt result. Human-perceived skeleton fluency improved.
Skipped close poses need engine-block diagnostics rather than an assumed tracking
failure. The next local candidate relaxes release/rearm to 0.45, keeping close
0.35; no production default adoption or new capture is implied.

Proposed prayer-pose calibration is tracked in [WiggleControl #2](https://github.com/AlterMundi/WiggleControl/issues/2): deliberate palms-together activation during an already human-enabled session, guided horizontal/vertical comfortable bounds, corner verification and receiving-local profile adoption/retry/cancel. Reuse native perception/overlay; relative-pointer recentering and absolute-touch mapping require distinct qualification. No prayer detector or calibration ceremony is implemented by recording this proposal.

Current test profile restored to original 0.25/0.4 after human-reported lower
responsiveness of 0.35/0.45. Keep the improved preview/timing/block explanations.
The human prioritizes advancing calibration/system interaction over prolonged
pinch tuning; one further baseline diagnostic trial is requested, readiness
still required. No wider-tolerance candidate was adopted in production.

The human selected the four-zone qualification page without camera video, then
requested moving hand feedback directly to the real desktop overlay rather than
maintaining a temporary web renderer. daimon-integrators now owns a transparent
GTK3/layer-shell hand surface and separate scoped Stop button; WiggleControl
retains native perception, gestures, input and the bounded `/state` projection.
The JavaScript hand renderer was removed. The page remains an ordinary trusted
input observer; live camera-preview is reserved for image alignment diagnosis.

Hand feedback currently maps mirrored palm position into the output workspace
and preserves camera-pixel bone lengths. It is not coincident with the existing
relative mouse pointer. Guided calibration and a qualified cursor mapping remain
pending. No new acquisition starts until the next explicit human ready reply.

## Calibration before further click qualification

The first acknowledged native-overlay trial ran its visible measured window for
30.35 seconds. Nicolás reported a flickering hand and a disconnect from the
actual mouse/click representation, then requested calibration before continuing
click tests. The mismatch is architectural: mirrored palm-to-output mapping and
relative mouse deltas are separate coordinate contracts. Issue #2 now prioritizes
a shared qualified screen target, explicit native calibration control, robust
comfortable bounds/corner verification, local profile adoption/rollback and visual
continuity independent of input freshness. Prayer-pose entry follows the manual
workflow. Pinch thresholds remain 0.25/0.4. No calibration is implemented yet.
The human cleared the earlier end-to-end goal; no replacement has been created.

Calibration implementation checkpoint: an independent absolute virtual mouse
now reached six exact normalized screen positions in real Chromium/labwc, plus
click and watchdog release without camera. `absolute-stream` is separate from
relative mode. The core manual sampling/profile logic reuses Workspace mapping
and has bounded fresh median/stability tests. It is not wired to native controls
or perception yet; full calibration, matching overlay anchor, persistent rollback,
visual continuity and prayer entry remain active goal work. Production services
and device permissions were not changed.

## Current active calibration goal

Nicolás explicitly activated the new end-to-end calibration goal after clearing
the earlier goal. Preserve its full scope: shared input/overlay mapping, visual
continuity, local profiles/rollback, physical corner/pinch/drag/loss/Stop acceptance,
then prayer-pose activation. Physical acquisition still requires a new ready reply.

Source now has four-corner verification requiring 500 ms of continuous fresh
observations per corner; stale/lost/invalid observations break the dwell. Profile
storage validates bounded owner-local files, publishes atomically at mode 0600,
retains the previous profile and supports rollback. Invalid profiles, corrupt
current data and symlinks do not overwrite the current profile in tested cases.
The finite loopback projection has a single-slot calibration command seam for
start/next/cancel/accept/rollback, disabled by default unless its producer selects
calibration mode. HTTP queue acknowledgment is not adoption; producer state must
report the actual result. Source tests cover origin/enablement/backpressure.

These are source components, not completed native UI or live calibration. Next
wire the session controller: neutral input while collecting; hover-only candidate
review; acceptance only after four corners; atomic adoption plus neutral rearming;
shared emitted absolute target in metadata and native overlay. Add explicit native
controls and independent bounded visual freshness before deployment and new
human trials. Prayer activation remains after manual workflow qualification.

Runtime/control wiring checkpoint: `run-calibrated` now joins the separately
qualified absolute writer with the calibration coordinator, validates an explicit
receiving output/profile before models, keeps collection neutral and review
hover-only, requires verified corners to adopt, and releases/rearms on each
control. It publishes the actual emitted screen target and calibration result.
A full camera-free lifecycle test covers collection, refused early acceptance,
corner hover, adoption, active mapped input, stale release, cancel and rollback.
The native overlay adds manual controls, stage/sample/corner guidance and the
shared-target anchor. Visual retention is independently bounded at 300 ms with
strict 150 ms input/press eligibility. Receiving builds/layout qualification
are separate from still-pending real calibration/pinch/drag/loss/Stop and prayer
activation. No camera has opened during this implementation increment.

Prayer-entry preparation while manual human acceptance is pending: a camera-free
2-D candidate detector now requires two confident, opposite-handed, stable-ID
upright hands, close palms/fingertips and one second of fresh continuous evidence.
It latches one activation and requires 400 ms of observed separation to rearm;
loss/ambiguity does not rearm it. Tests include disabled input, stale/future data,
occlusion, low confidence, folded fingers, identity replacement, array reordering
and uncertain/same-side estimates. Thresholds are an unqualified prototype, not
physical accuracy or proof of palm contact. No existing pinch thresholds changed.

This detector is not wired/enabled in receiving sessions. Manual calibration
acceptance still precedes gesture activation. Existing infer_many marks centers
closer than 0.8 palm widths ambiguous and suppresses nearby new ROIs; actual
prayer-like occlusion/model coverage is an explicit receiving uncertainty.
Do not weaken touch identity safeguards or silently accept an ambiguous pose.
Human manual calibration/comfort/loss/Stop remains the current pending action;
no additional camera/model session was started by this source preparation.

Prayer wiring checkpoint: explicit `prayer_enabled` (default false) now joins
existing native multi-hand perception and the candidate detector in calibrated
sessions. Non-calibrated modes reject opt-in before inference. A gesture can
request neutral calibration only from an active accepted profile, after visible
window acknowledgment and before expiry. It cannot adopt a profile or interrupt
sampling/review. The detector latch remains observed across modal stages, so
occlusion is not treated as rearming. Existing engine release/open-first rules
protect pointer replacement and every calibration entry. Feedback reports its
state. Source tests cover absent/active/modal profile gating and disabled legacy
config/default/pre-inference rejection. Receiving manual configuration stays
unchanged with prayer disabled; manual and prayer physical acceptance still wait
for the human's next explicit ready acknowledgment in the required order.

Manual trial correction: the human was distracted; no physical calibration/profile
was accepted. Native presentation felt less fluid. Confirmed-bound-only drafts
now allow short independently acknowledged trials, instead of fitting chat and
all four bounds/corners into one countdown. Resumption retains no observations or
corner approval, and never activates a camera itself. Start/Cancel restart/discard
it; adoption still requires complete fresh review. See QUALIFICATION.md.

## Two-diagonal comfort calibration — 2026-10-14

The human confirmed four manual bounds in separate finite sessions. Corner review
passed upper-left but the human could not reach upper-right, describing a barrier
before the corner. No profile was accepted; review was stopped and observed camera
closure retained. Separately sampled extrema may be unreachable together: this
is a hypothesis, not a diagnosed cause. The captured point after the report was
not a held-at-limit sample and cannot establish the barrier's coordinates.

Nicolás selected a shorter two-diagonal trial prioritizing comfort and precision.
POST calibration/diagonal samples a stable comfortable upper-left position then
lower-right, both axes together. Both samples retain existing freshness/stability
checks; each confirmed pose survives expiry. The candidate maps an interior
rectangle with 5% margin per side to the screen. Four fresh corner checks and
explicit acceptance are still required; the margin is a trial choice, not a
qualified universal default. Classic Start remains supported. Native CALIBRATE
selects the diagonal route; prayer entry remains the previous dormant route.

Use a separate receiving-local profile/draft for this trial to preserve the old
four-bound evidence. The human must not chase screen edges during raw-coordinate
sampling. Position the whole hand comfortably within the camera view; any tracking
failure invalidates pending samples. Camera-exit-based auto-calibration and frozen
last-hand visual feedback are deferred, not implemented by this change.

## Retain completed review across finite expiry — 2026-10-14

The human confirmed both diagonal poses and subsequently reported completing
corner review. By the time the operator read state, the finite listener had
closed and no accepted profile existed. The old runtime retained neither final
review metadata nor completed corner evidence on disk. Keep the human report;
do not invent a producer completion receipt. The confirmed diagonal draft stays.

Completed fresh corner review now writes an owner-only atomic `.verified`
candidate before expiry. It remains distinct from accepted profile and input
activation. Explicit camera-free `accept-calibration PROFILE` validates that the
saved candidate matches the unchanged confirmed draft, adopts with the existing
rollback store, and removes pending draft/verification. Start/Cancel remove pending
verification too; a changed draft, missing proof or malformed proof cannot adopt.
No camera restart or automatic acceptance occurs. Older trials cannot retroactively
produce this evidence. Physical replay of review is still needed once the prepared
release is installed; the two measured poses do not need repeating.

Receiving two-diagonal adoption now passed with the relocated camera: both poses
were confirmed, all four fresh corner checks completed, saved candidate existed,
explicit camera-free acceptance succeeded and camera/mic closed. See QUALIFICATION.
This is the first accepted calibration profile, not completed mouse/prayer goal.
Next human-ready work is pointer alignment, intentional pinch and drag, then
loss/Stop and the deferred visual-continuity improvement/prayer acceptance.

Expanded workspace request: after accepting the first diagonal profile, Nicolás
requested a larger movement rectangle. New upper-left confirmed; lower-right
collected zero usable samples during a 20-second operator wait, and was not
confirmed. Camera was stopped/observed inactive; accepted profile preserved. The
cause is unknown because the sampled summary did not expose detection confidence
or full-hand in-view status. Native calibration feedback now distinguishes delayed
frame, no detected hand, low confidence, partially out-of-view landmarks and
visible stable sampling, separately from ordinary pinch engine diagnostics.
Next finite human trial must inspect that actual reason rather than guessing.

## One-pass guided calibration — 2026-10-14

Nicolás requested one continuous guided interaction rather than chat confirmations
per pose, with twenty seconds per step and setup excluded. New explicit
calibrate-guided CONFIG PORT PROFILE is camera-only: no uinput writer, no clicks,
no prayer activation. Each pose gives fifteen seconds to move and five to hold;
only the hold interval enters robust sampling. Four corners each allow up to
twenty seconds and advance after the existing fresh dwell requirement. Every new
step waits for the native UI's actual painted, fresh, full-hand feedback and output
dimension match before its clock starts. Setup initially waits up to thirty
seconds; total capture is bounded at 150 seconds. The global writer/normal preview
limits remain unchanged. Failed/insufficient samples stop with a retry result;
completed review saves evidence but never automatically adopts. Old accepted
profile remains intact. Stop remains available, manual stage controls disabled.

Source tests cover per-step acknowledgment, setup exclusion, move/hold gating,
invalid-step same-origin refusal, countdown aging, insufficient-sample refusal,
all-corner completion and saved-proof-only outcome. Guided physical acceptance
still pending. Overlay lifetime is independently finite up to 180 seconds.
Native feedback also now accepts null blocked_reason when pinch is pressed;
previous parser expected text and could discard that valid metadata. Its regression
preserves input freshness and rejects invalid guidance. This is a source finding,
not measured physical pinpoint accuracy or completion of the larger goal.

Guided receiving adoption completed: human reported COMPLETE and saved all-corner
candidate existed. Initial exact-match refusal was reproduced as JSON float
parsing drift, corrected with serde_json's existing float_roundtrip feature
(9dce3da), without weakening matching. Receiving corrected build accepted the
retained proof camera-free; former profile preserved for rollback. New workspace
is about 1.80 times as wide and 2.96 times as tall as the previously accepted one.
Camera/microphone inactive afterward. No repeated gesture capture occurred. This
qualifies one guided completion/adoption, not pinch/drag accuracy or exact perceived
setup timing/fluency, which still require human-directed physical checks.

## Full web/native baseline review — 2026-10-14

See [WEB-BASELINE-REVIEW.md](WEB-BASELINE-REVIEW.md) for the source comparison
requested after failed held-pinch feedback. Recognition is shared Rust; the web
page is a renderer. Source-frame timing cuts and a stricter native display-age
gate are separate findings. Do not claim the failure is solely before drawing.
Preserve the baseline, qualify shared state semantics and actual release reasons
before further physical tuning. No capture or threshold change during review.

## Human-reported position-dependent 3D scale — 2026-10-14

Nicolás reports that estimated fingertip distance changes with image position:
at the center, an intended pinch can remain at60–70 mm rather than reaching
the45 mm threshold. An open pose reads approximately130 mm at the center versus
90 mm at the sides. These are human observations, not a controlled measurement
of constant physical pose or a confirmed cause.

Source inspection: infer_roi retains model output3 directly as world landmarks;
world_pinch_distance_m computes Euclidean distance between landmarks4 and8.
There is no explicit image-position scale multiplier in that distance path.
This does not establish correctness of the model output contract or preprocessing.
Possible model/crop/pose scale bias remains a hypothesis. Do not tune global
45/65 mm thresholds or introduce an image-position correction from this report.
Next diagnostic should compare raw gap, stable 3D bone lengths, crop geometry
and screen position for matched poses, using retained recordings first. A3D
anatomical normalization is a candidate only if the apparent scale drift is
shared by those lengths; preserve raw estimated meters and distinguish derived
unitless measurements. No new capture or runtime change for this report.

Offline investigation now has a reproducible Rust position-sweep generator and
two actual-model controlled pose comparisons. See POSITION-BIAS.md. Synthetic
translation with corresponding ROI is exactly invariant; cold detection and
tracking recrops alter estimates. An experimental proximal ROI is not uniformly
better and remains probe-only. No live correction or new capture was deployed.
The human clarified only horizontal real-camera variation was observed.

Human-requested AI human pose/animation research is retained in
research/ai-human-motion-2026-10.md, with official availability, output/rig
limits and a concrete controlled-render experiment. Research is not an installed
renderer, generated human motion, or a qualified fingertip correction.

A first actual MHR articulated-rig CPU render experiment now exists (2026-10-15):
eleven geometry-controlled frames, ten cold detections, one retained miss. The
rig-marker gap is invariant while model estimates vary across view/position.
Portable generator and asset provenance live under tools/synthetic/; see
POSITION-BIAS.md for results and limitations. No live correction is qualified.
