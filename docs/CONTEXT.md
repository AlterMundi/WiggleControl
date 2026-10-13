# Project context — 2026-10-13

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
