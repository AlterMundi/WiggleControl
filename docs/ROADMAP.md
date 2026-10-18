# Camera controller roadmap

Current agreed delivery: the [one-hand Linux public preview](LINUX-PREVIEW.md),
tracked by [milestone 1](https://github.com/AlterMundi/WiggleControl/milestone/1).
Complete left/right/double click, gesture scroll and Escape; add a held detail
modifier selected before an action; package and qualify independent Linux use.
Pi 5/labwc is the initial receiving environment; Nicolás offers the ThinkPad for
Linux x86-64 testing. Changing detail during a drag/stroke is optional: include it if naturally
independent, defer only separate extra work. Second-hand modifiers, multitouch
and Windows/macOS ports remain later work.

The opt-in [held-detail implementation](DETAIL.md) modifies calibrated absolute
motion independently of left/right buttons. A separate sourcef21a48b Pi trial
completed90s; Nicolás reports very good detail use with intentional clicking in
the X menu. Small-target/rest/drawing, edge/held-action behavior and longer comfort
remain to qualify; the normal beta/calibration remain preserved. [Escape delivery](ESCAPE.md)
now connects an opt-in deliberate thumb-out gesture in source; actual keyboard
effects and human recognition/comfort remain pending.

The preview plan owns the current delivery scope and issue order. The reconciled
[goal specification](ONE-HAND-GOAL.md) carries prior reliability/comfort work into
the new human-requested goal after the old goal was cleared. The broader touchscreen roadmap
and dated implementation/receiving checkpoints below remain historical evidence,
not additional gates on this Linux preview.

Updated 2026-10-18. This is the current development entry point. The original
six-milestone design remains in [DEVELOPMENT.md](DEVELOPMENT.md); dated receiving
evidence remains in [QUALIFICATION.md](QUALIFICATION.md) and human acceptance in
[ACCEPTANCE.md](ACCEPTANCE.md). [CONTEXT.md](CONTEXT.md) preserves history.

Update2026-10-18: Nicolás confirms that the stuck-folder incident has not
recurred and asks to consider[#8](https://github.com/AlterMundi/WiggleControl/issues/8)
resolved. It no longer blocks the beta; the original cause remains unknown.
This supersedes pending-reproduction statements below. See
[the goal update](ONE-HAND-GOAL.md#sticky-desktop-drag-resolved-by-human-acceptance-2026-10-18).

Current software-only result: source2c533e5 is selected for next manual Start.
Coalesced writes, on-demand feedback and bounded timing pass camera-free checks.
A separate browser quota200% comparison reduces simulated whole-chain p95
~101→79ms while preserving host frequency/fan policy and the accepted model/
calibration/gesture settings. No new physical trial is claimed. See
[the measured receiving scope](research/one-hand-interaction-2026-10.md#receiving-software-only-comparison-and-scheduling-quota-2026-10-18).

Current authorized track: software-only optimization while camera/lighting setup
is deferred. Coalesce JSON writes on recording/input paths, skip unused feedback
preparation, bound diagnostic state, and qualify identical retained/synthetic
inputs before selecting a receiving candidate. Keep the accepted calibration,
model, gestures and power profile unchanged. Next isolate JPEG transfer/decode/
worker costs with retained timing evidence; adaptive cursor retuning remains a
separate measured tradeoff. Physical camera/input trials wait for fresh manual
Start. See [the current increment](BETA-1.md#software-only-hot-path-work-2026-10-18).

Implementation tracking: [#3](https://github.com/AlterMundi/WiggleControl/issues/3),
with existing runtime acceptance [#1](https://github.com/AlterMundi/WiggleControl/issues/1)
and shared calibration/prayer [#2](https://github.com/AlterMundi/WiggleControl/issues/2).
The separated-fingertip macro/contact counterexample is owned by
[#7](https://github.com/AlterMundi/WiggleControl/issues/7); paired overlap remains
in [#6](https://github.com/AlterMundi/WiggleControl/issues/6).

Current one-hand work adds opt-in bounded drag continuity, independent absolute
cursor output and brief click stabilization. The
[interaction comparison](research/one-hand-interaction-2026-10.md) records
synthetic smoothness/click gains, lag/overshoot costs and unchanged button counts
under stabilization on older retained observations. Explicit single-observation
onset improves short synthetic pinches but admits false closures; confirmation
remains40ms by default. The optional profiles are staged separately for finite
comparison; none is selected for WG1 or human-accepted.
Receiving issue[#8](https://github.com/AlterMundi/WiggleControl/issues/8), actual
double-click/drop behavior and integrated seated-use acceptance remain open.
The human subsequently freed the Pi and explicitly resumed the one-hand goal.
Camera-free receiving qualification has restarted; new camera trials still need
fresh human readiness.
The latest opening-evidence comparison also rejects universal release debounce:
its isolated-noise benefit has a measured deliberate-release/double-pinch cost.
Intent-labelled retained evidence and actual application trials remain necessary
before selecting that optional policy; immediate opening remains the default.
The next receiving step is prepared in the
[camera-free absolute drag trace](ABSOLUTE-DRAG-TRACE.md): compare explicit
release/Stop, producer silence/EOF and writer death using only owned empty folders
and the named virtual mouse. Local fixture/parser/protocol checks pass; actual
kernel/application traces now complete once per case on both desktop and PCManFM
window, with original folder moves and no floating folder in inspected live-cursor
hover images. Issue#8 remains unreproduced; capture actual input during a closer-to-
usage sequence rather than inferring a cause. Exact-position mouse pairs open the
owned folder at300ms but not500ms; pinch-to-double-click and spatial tolerance still
need actual qualification. See the trace report for limits.

A d8f9bb9 ARM64 candidate stages the same reference and optional smooth/one-second
drag-grace profiles with [manual finite trials and independent input observation](ONE-HAND-TRIAL.md).
Both receiving preflights pass; actual idle beyond89s and frontend Stop confirm
no acquisition and successful provider/observer teardown. The actual1920x1018
standby fits the display. A camera-free owned-folder case independently retained
the virtual mouse's down/up and device removal; issue#8 remains unreproduced.
The selected beta, calibration and disabled macros are unchanged. Next obtain
fresh readiness for a45s seated-use comparison, preserving private video and
semantic/kernel evidence. Standby and synthetic gains do not establish actual
camera throughput, motion cadence, natural double-pinch or human comfort.

Retained actual Beta journals now strengthen the offline comparison:4248 selected
frame button states exactly match the reference. Existing1000ms continuity
preserves78 press timestamps and replaces12 stale cancellations with later observed
opening;5 tracking-loss cases end by bounded expiry/owner change/disengagement.
[The interaction report](research/one-hand-interaction-2026-10.md#retained-beta1-session-comparison-2026-10-16)
keeps the counterfactual/intent limits. Old track IDs alone cannot test new
association retention, and rejected raw predictions prevent full-session replay.
Physical comparison still awaits the existing fresh-ready request; no trial
profile change or new receiving build is needed.

Later actual progress supersedes that pending request: the human manually ran
the smooth trial and gave positive overall feedback, but it ended before45s.
Issue[#9](https://github.com/AlterMundi/WiggleControl/issues/9) owns the receiver
Frame rejection, separately from unreproduced#8. ARM64 diagnostic228cc30 is now
selected only by the finite trial specifications, with unchanged profiles and
baseline WG1. CI, device-free receiving replays, both preflights and actual
standby/human-Stop teardown pass. Next: a fresh manual45s recorded repetition to
identify the failed predicate, a causal correction, then complete application
and human comfort qualification. Preparation does not establish that runtime
interruption is fixed. See the [current receiving checkpoint](BETA-1.md#first-finite-smooth-trial-and-receiver-diagnostics-2026-10-17).

The retained actual trace now confirms frequent device output:15.488ms median/
20.731ms p95 motion-report intervals versus78.757ms median hand-evaluation
intervals. [The cadence analysis](research/one-hand-interaction-2026-10.md#actual-finite-trial-pointer-cadence-2026-10-17)
keeps this distinct from display cadence and source-to-kernel latency. Perception
age and eligibility pauses remain separate constraints; full finite runtime,
natural double-click, actual drops and comfort acceptance remain incomplete.
The retained timing partition locates98.0% of summed source-to-meaning time before
server result pickup: source→lease and lease→result. It does not isolate device
buffering or pure inference. Collect the existing browser timing trace during the
next already-prepared diagnostic repetition before choosing a latency change.
No new model/profile/artifact or receiving readiness reset follows this analysis.

[Actual presence export](research/provider-presence-scores-2026-10.md) now retains
measured per-hand quality at the original0.5 SDK floor. On the receiving GPU,
100 executions/50 matched pairs retain exact public detector outputs, including
actual two-hand, rejected-ROI and deduplication cases. Source opt-in connects it
to Rust/classifier without changing0.8 macro gates; conditional contact outputs
remain exact. Full real-clock transport/recognized timed macro acceptance is next.

The [raw-provider macro connection](research/provider-visual-macros-2026-10.md)
now retains optional image depth and reuses the existing classifier/arbiter.
Without measured presence, SDK configuration must meet the macro confidence gate;
conditional retained-tensor parity passes. Actual receiving classifier cost is
0.423ms median, but raising the SDK floor loses useful near holds even with the
classifier disabled; that profile is rejected. Successful continuous image-based
macro recognition and physical acceptance remain open.
No everyday profile or prepared physical trial is replaced by this source work.

Current [real-clock transport](PROVIDER-TRANSPORT.md) implements a finite encoded-
frame→GPU-worker→Rust fixture with host-owned run/frame leases, latest-slot
coalescing, independent meaning ticks and timestamped release/expiry receipts.
Actual Pi runs now execute that full fixture. Fixed one-hand detection improves
matched one-contact control from6.97Hz/154.3ms to13.70–13.98Hz/81.4–84.5ms
median encoded-publication→meaning, retaining19/22 and18/21 accepted near frames.
Wall-time coverage is only about65% of near geometry;151.6–194.0ms first-contact
delay remains visible. Prepared two-hand control retains9/15 accepted dual near
holds and44% of expected near wall time. Delayed-result releases at302.739ms
(two hands) and303.779ms (one hand) retain the300ms limit.

[The receiving report](research/realtime-worker-2026-10.md) preserves initial
failures, controlled comparisons and reproducible scoring. No camera or OS input
is opened. The explicit finite camera source now reuses timestamp-validated capture/private
recording and the independent native writer through this seam, with a sensor-free
preflight and fixed graph selection. Source4385fdd receiving preflight, zero-capture missing-camera cleanup and
normal synthetic regression now pass; terminal feedback is acknowledged before
listener shutdown. Physical camera/input execution remains unqualified. Retained trial replay is now
measured below; next annotate representative intervals, broaden negative/ownership
coverage, then fresh-ready physical and human/application acceptance. Keep live calibration,
selected runtime and original prepared trial intact; do not stretch freshness
to hide processing cost.

Retained-footage replay now runs on the Pi as an explicitly recorded source
(source6ab609c), with separate original/new clocks and no capture/input. A first
60-second window publishes905 original frames and accepts797 at13.265Hz;
97.091ms median/140.306ms p95 publication→meaning, final neutral and both end
receipts. No independent labels exist, so detections/contacts are observations,
not accuracy. Next: annotate short presence/pinch/continuity intervals, compare
tracker identity and onset/release behavior on identical bytes, then replay the
recorded400ms gap. Preserve original recording and separate intent labels from
predictions. Full physical/application acceptance remains required.

[Sparse recorded-pinch inspection](research/recorded-pinch-metrics-2026-10.md)
now isolates missed onset with a stable hand identity. The prior projected
palm-length cue matches9/9 apparent-contact and9/9 open samples, versus3/9 and
8/9 for estimated3D; these are nonblinded assistant appearance labels, not physical
intent accuracy. An explicit optional dimensionless metric is implemented across
modes without changing defaults. Actual receiving replay now gives13.247Hz/99.373ms median,9/9 sampled apparent
contacts and0/9 open contacts, with both terminal receipts. Next qualify
continuous/other-view behavior; earlier adverse-yaw failures prevent unconditional
adoption. Keep raw3D and projected evidence separate.

Dense review of75 original frames now measures one3.412s uninterrupted projected
contact within3.596s of apparent closure, versus0.426s for the actual3D replay.
The80s next recorded window retains13.589Hz/99.522ms median transport but most
side-view appearance labels remain uncertain. The recorded scorer explicitly
includes coalescing/processing delay and refuses to bridge sparse/gapped labels;
these are scoped appearance findings, not physical intent or spatial accuracy.
Next qualify additional continuous views/negatives, owner identity and actual
human/application use; keep the full modes/macros/participant requirements below.

Earlier [raw provider/worker connection](research/provider-worker-2026-10.md)
feeds saved raw Tasks points through Rust's own spatial tracking and existing
meaning, retaining4/5 adverse near holds and20/20 separated-pair stable counts
without native IDs/quality/cursor. Actual classic-worker GPU processing on Pi is
131.4ms one-hand/102.6ms two-hand median; animation callbacks continue during
every inference and resulting offline Rust contacts match the main-thread replay.
Clock/age/order/loss/rearm/EOF controls are implemented. A live same-clock capture/
worker→Rust transport, broader ownership/negative coverage and whole-path timing
remain next; image-depth/visual-macro integration is explicitly unavailable.
These are component connections, not selected live control or full goal completion.

Earlier [palm correspondence/GPU comparison](research/palm-correspondence-gpu-2026-10.md)
fixes a false second-hand reconstruction without an additional inference. The
uniform palm/1.2 policy preserves all1042 retained mixed-model contact decisions
and recovers the adverse one-hand4/5 hold that the old palm policy lost. Actual
Pi hardware-WebGL comparison lowers Tasks processing medians221.6→132.2ms for
one hand and206.0→102.9ms for two, with expected detections on every input.
Conditional Rust replay retains the GPU metric's4/5 adverse hold. This is not an
integrated accelerated backend or accepted live control. Next: evaluate an
optional worker/IPC perception path with bounded timing/ownership, address
held-release/crossing uncertainty, then physical/application acceptance.

Earlier perception work: [fixed tensors and mixed models](research/fixed-tensor-mixed-model-2026-10.md)
now isolate crop/model effects and validate a converted Tasks landmark model on
286 exact native inputs. The default-off Rust/ORT secondary model reaches116/117
near frames at20Hz and17/18 on actual10Hz hard scenes, versus115/117 and12/18
for shared-model dual crop. Primary1042-frame observations remain exact, with
no V/open contacts. Separated pairs stay qualified on their existing synthetic
holds; crossings still fail, adding five unscored contact frames in one layout.
Do not adopt generally. Receiving ARM64 image-only measurement reaches138.5ms
one-hand/268.7ms two-hand under1.5GHz, exceeding the10FPS budget. Matched-policy
quality reproduces4/5, while the paired policy false-splits the single-hand case.
Next: reduce secondary cost without extending freshness, qualify acquisition/
release/ownership, then fresh human/application acceptance. Native macros,
participants and every broader roadmap requirement below remain active. Selected
runtime/calibration and the original prepared trial are preserved.

Latest#7 source candidate: [late onset permission](research/pinch-onset-veto-2026-10.md)
uses the previously tested0.38 pixel ratio to veto a new press while retaining raw
confirmation/rearm/release. It removes development26/12/13 negative contact frames
at20Hz/both10Hz phases and preserves every per-frame contact decision across nine
positives and another negative. Two new MCP-flexed/distal-straight references and
the native image connection are checked. Four new views reveal an adverse-yaw
regression1/9→0/9 near coverage at20Hz; both10Hz phases already fail. Six retained
pair trajectories stay exact, including their limitations. Reject unconditional
adoption; selected runtime/profile stay unchanged. Noisy cues, wrong held-release metrics,
broader hands/views, two-hand controls, macro recognition and physical acceptance
remain gates before adoption. Earlier rejected metric replacements below remain
historical evidence; this candidate changes where permission is applied.

Earlier#7 evidence: full-trajectory skin measurements and285 matched controller
replays reject the tested palm-length/dwell grid as an adopted fix. No scalar candidate
both removes separated-skin contacts and preserves baseline near-hold coverage
at20Hz and both10Hz phases. Continue geometry/perception comparisons before
changing the receiving thresholds; see the
[complete comparison](research/native-visual-macros-2026-10.md#full-trajectory-interpretation-comparison).
Two additional articulated pinches now add82 detected frames and18/18 near-hold
contact frames, with independent1.790/0.961mm distal skin gaps. The combined
six-positive-control benchmark retains53/54 baseline near contacts; tested
normalizations still lose useful coverage. These remain synthetic controls,
not physical acceptance or a resolved#7. See the
[new articulations](research/native-visual-macros-2026-10.md#additional-articulations).
An optional[WiLoR mesh comparison](research/mesh-reconstruction-2026-10.md) now
separates eight selected near/far inputs under known-crop/right-hand assumptions.
Its143-input continuation completed: conditional metric-only replacement removes
all26 negative contact frames at20Hz and preserves18/18 near-hold frames on two
positive trajectories, versus17/18 baseline. An eight-input retained-native-box
probe also preserves far/near separation. Full native-crop/remaining-view tests,
interactive latency, both-hand ownership and physical input remain open. CPU cost
is several seconds per frame and5GiB peak; no model/runtime/profile was adopted.
The subsequent[MobRecon SpiralConv comparison](research/mesh-reconstruction-2026-10.md#mobrecon-spiralconv-cpu-comparison)
completed307 native-box reconstructions across the negative and all six positives.
It removes negative contacts but reduces20Hz near-hold coverage53/54→23/54
(10Hz26/30→8/30 and23/24→10/24). VPS median114ms/p95512ms and roughly303MiB
cgroup peak are lighter, but useful pinch coverage rejects direct replacement.
No threshold/profile/runtime change follows; accurate low-latency reconstruction
and physical acceptance remain open.

The [CPU execution and screen-space continuation](research/wilor-cpu-execution-2026-10.md)
measures97% of WiLoR forward time in its backbone. Dynamic-int8 transformer
blocks preserve eight selected near/far distinctions but still cost2.89s/frame
median and5GiB load peak. A fixed pixel-gap/palm ratio removes the development
negative but loses useful near frames; a new side-position control retains9/9
at20Hz yet loses one of five near frames at10Hz phase0. A new yawed negative
already passes baseline. Neither CPU optimization nor this screen interpretation
is adopted. Reproducible stage/precision probes and fixed-config evidence remain
available; practical accurate perception, temporal/ownership and human gates stay open.

The [tracking-recovery comparison](research/tracking-recovery-2026-10.md) now
adds current-frame candidate diagnostics and an opt-in palm-point deduplication
policy. It preserves409 single-hand/controller/macro frames and separated-pair
holds, while improving some crossing detection coverage. New two-right-hand
references pass61/61 separated detection/41 stable counts; crossing improves
33→42/61 detections but retains0/13 dual near holds.10Hz limits and the corrected
duplicate-inference regression are retained. Default/runtime policies stay unchanged;
occlusion ownership, participant roles, Pi resources and physical acceptance remain open.

## Outcome and today's goal

Two moving hands should comfortably manipulate ordinary applications as deliberate
independent touch contacts. One-hand pointing remains useful on mouse-oriented
applications. Additional modes should reduce effort rather than require learning
an extensive gesture vocabulary. Runtime operation remains local and manually
enabled, with visible engagement, actual emitted targets and scoped Stop.

Nicolás explicitly authorized an active native Cuevanico goal on 2026-10-15:
iterate through simulation, parameters, modes, models and software to make the
best practical camera controller available to this setup. The goal includes
implementation, comparative evidence, receiving qualification and human comfort;
it is not complete merely because this roadmap or a synthetic benchmark exists.
No token budget or deadline was assigned. The earlier Shared Resources goal
retains its own blocked state; this new request does not rewrite its history.

The immediate work is camera-free. A real hand trial still requires a freshly
acknowledged, prepared task. A single RGB camera cannot make hidden anatomy or
absolute physical depth observable by declaration. Prefer reliable interaction
with explicit uncertainty over a plausible but unqualified 3D reconstruction.

## What exists

| Layer | Implemented / observed | Unfinished |
| --- | --- | --- |
| Perception | Native ONNX CPU palm/hand inference; ROI tracking; up to two tracks; confidence/ambiguity and offline rejection reasons; source-linked models and ARM64 measurements | Crossing/occlusion loses continuity and can briefly start a premature second contact before rejection; varying hands/backgrounds and pinching pose bias remain |
| Gesture meaning | Open-before-press; hysteresis/confirmation; drag/scroll; loss/stale/Stop release; independent contacts; optional native pose→macro connection with positive-open neutral/ownership/hold/cooldown and observed feedback | Comfortable precision/latency balance; receiving mode-arbiter/writer qualification; reliable neutral/action recognition and pinch exclusion, independent temporal macro labels and actual action adapters |
| Mapping | Shared diagonal calibration, guided review, saved profile/rollback, matching native overlay target | Complete corners/comfort acceptance; participant-specific workspaces |
| Delivery | Real virtual mouse on Chromium Wayland/VLC XWayland; type-B two-contact writer and touch-aware client scale/rotation; selected local compositor patch restores all five scripted stationary touch→mouse cases in owned Wayland Chromium | Camera-driven multitouch acceptance; other clients/locks/grabs and wheel-only handoff (#5); actual fallback per application |
| Operation | Finite and selected until-stop controls, private rolling recordings, existing local panel, independent physical input | Final receiving configuration, actual reboot qualification of the prepared input grant (#4) and sustained accepted-hand resources |
| Simulation | Static MHR controls; hand-only yaw;2016 full-workspace /5760 calibrated analytic cases;25-frame single/paired motion;41-frame closures across four additional views;61-frame independent paired pinches/crossing with geometry/ownership scoring and10/20Hz comparisons; pinned Tasks CPU comparison; TV progress/results | Full anatomy/human intent, robust held-contact ownership through occlusion, participants and broader coverage |

Implementation and deployment are distinct. The adaptive filter and new simulation
tools are source experiments; the selected receiving 45/65mm live pinch thresholds,
calibration and selected runtime binaries remain preserved. A separate ARM64
candidate now passes preflight and partial scripted combined-device qualification;
see [CONTROLLER-NATIVE.md](CONTROLLER-NATIVE.md). Stationary Chromium handoff is
now qualified with the separately maintained local compositor patch; live hand
control remains pending and the controller candidate is not the everyday runtime.

## Ordered work and decision gates

1. **Establish a trustworthy iteration loop.** Preserve independent intended
   trajectories/events, actual images, model observations and emitted intent.
   Classify invalid render inputs separately. Fix seeds; separate tuning from
   held-out cases; retain failures. Measure rest jitter, moving error/lag, contact
   precision/recall, onset/release delay, track continuity and cancellation. Show
   actual producer stages on the existing TV view, with explicitly labelled replay.
   First increment is implemented; expansion below remains active work.
2. **Improve one-hand control.** Compare the existing exponential filter with
   speed-adaptive filtering across input rates, noise, diagonal movement and
   workspace edges. Add pinch-aware target stabilization only if measured
   closure-induced motion justifies it; never silently freeze intentional drag.
   Compare2D scale-normalized intent, estimated3D gap and a calibrated hybrid on
   the same independently labelled frames. Reject fist/occluded/uncertain states.
   Select by a jitter/latency frontier, not one aggregate accuracy number.
3. **Qualify actual temporal vision.** Render plausible open/closing/contact/
   opening sequences with validated visibility and independently defined skin
   separation/penetration. The current MHR joint markers do not supply that truth.
   Add background/light/scale/view diversity and measured timing perturbations.
   Compare cold detection, recursive tracking and alternative crop strategies
   using identical inputs. Add real retained footage only with human intent labels;
   model predictions cannot label themselves. Current rigid open-hand motion is
   an early tracking benchmark, not a pinch benchmark.
4. **Two contacts and ownership.** Add two rendered hands with independent
   trajectories, asymmetric press/release, array reorder, approach, crossing,
   partial/full occlusion, exit and return. Score identity switches during an
   active contact separately from safe cancellation and useful coverage. Compare
   motion-predicted assignment and re-detection with today's ROI/ambiguity logic.
   Never recover a held contact under an uncertain new identity. Prove contact
   slots and mapping in the real compositor/client separately from perception.
5. **Useful modes and deliberate macros.** Implement explicit transitions that
   release the old mode before arming the new one. Share perception/ownership;
   avoid parallel competing input streams. Add visible held state, cancellation,
   dwell progress, cooldown and positive rearm. Keep gesture-to-action mapping
   receiving-configured. Qualify a small useful vocabulary before expanding it.
6. **Participants and specialist tools.** Start with explicitly claimed left/right
   workspaces and one hand each, then evaluate crossing/zone handover. Handedness
   is not participant identity. Two participants' four hands require expanding
   the perception/contact limits and performance budget, not renaming the current
   two-track implementation. Distinct OS cursors depend on seats/compositor/client
   support; two device files alone do not create independent application pointers.
7. **Choose perception architecture from evidence.** Keep the qualified local
   ONNX baseline while comparing a current MediaPipe Tasks backend and eligible
   temporal/reconstruction candidates. Use the common observation/replay seam;
   justify C++/Python adapters where the actual model/runtime requires them.
   Consider a stronger available compute host for offline comparisons, with
   verified access/capacity; do not silently make ordinary input depend on a
   network. New GPU/depth/stereo hardware is a separate optional branch, not a
   prerequisite or an authorized purchase.
8. **Deliver and physically accept the chosen increment.** Build the qualified
   ARM64 artifact away from the fanless Pi when supported, preserve binaries,
   configs/calibration and rollback, and inspect actual receiving versions.
   Prepare the displayed trial before asking for readiness. Follow existing
   [acceptance](ACCEPTANCE.md), including real apps, corners, intended clicks,
   drags, relaxed false actions, loss/Stop/rearm, comfort and sustained resources.
   Update the installed profile only from relevant evidence. Publish what works
   and its limits; keep incomplete branches explicit.

## Future inference backend without Chromium

Human decision — 2026-10-18: retain the working headless Chromium/MediaPipe GPU
backend for the current Linux preview, and explicitly pursue a Chromium-free
alternative with equivalent tracking quality and equal or better performance.
This is later work under [#3](https://github.com/AlterMundi/WiggleControl/issues/3),
not a prerequisite for milestone1 or authorization to replace the accepted runtime.

Reuse the timestamped observation seam, native Rust/ONNX implementation and
retained fixtures. Compare real model/runtime/GPU availability on the Pi and
actual x86 target rather than assuming a native implementation is faster. Measure
the whole frame-to-input path, initialization, CPU/RSS and sustained thermal
behavior under matched camera/scene, hand count and power policy. Include image
transport/decode overhead and detection/tracking quality, loss recovery, false
actions and operator comfort; model inference time alone cannot decide adoption.

A candidate must preserve source freshness, ownership, calibration, gesture
semantics, writer/watchdog cleanup and manual sensor lifecycle. Pin distributable
assets/licenses and demonstrate independent installation before selecting it.
Keep Chromium and rollback available until a matched source/device/human comparison
supports migration. No particular replacement runtime is selected by this decision.

## Interaction modes

These are a product roadmap, not claims that all modes are implemented.

| Mode | Interaction | Status / qualification need |
| --- | --- | --- |
| One-hand pointer | Move, pinch click/drag, thumb-middle scroll | Existing baseline; finish precision and real comfort |
| One-hand direct contact | One deliberate contact in the calibrated workspace | Existing writer/meaning; physical acceptance pending |
| Two-hand touchscreen | One deliberate contact per hand; client interprets pan/zoom/rotation | Two-contact implementation; temporal vision and physical acceptance pending |
| Precision / clutch | Temporarily lower gain or reposition the hand without moving the target; visible mode | Relative precision and opt-in native arbiter implemented; absolute mapping/receiving qualification and effort acceptance pending |
| Bimanual tools | Dominant hand selects/draws; other hand explicitly controls pan, scale or a modifier | Proposed; mutually exclusive with raw two-contact routing |
| Two participants, one hand each | Each claims an input/workspace; stable role until release/handover | Proposed; no identity inference from handedness; compositor support checked |
| Dwell / accessibility | Optional deliberate dwell selection, larger targets, configurable hold times | Proposed; compare false activations and fatigue; not default auto-click |
| Macro layer | Held deliberate poses/chords trigger configured named actions | Semantic hold/rearm and local mode/pause routing implemented in replay; visual classifier, chords and native action adapter pending |
| Four-hand / shared canvas | Two participants with two hands; application-aware canvas where useful | Later branch; expand two-track/two-slot limits and measure hardware capacity |

Candidate macros: recenter, toggle precision, enter/leave scroll, open launcher,
play/pause media, volume steps, switch selected mode, enter the existing calibration
flow and local controller pause. Prayer remains an optional entry into the
existing calibration workflow, pending physical qualification. No pose starts an
idle camera. Destructive arbitrary shell commands are not a useful default macro.
Local controller pause does not invoke coordinated global Stop.

## Qualification criteria

Hard invariants: no duplicate contact ownership; no held input after the selected
freshness/watchdog deadline; no press on closed reacquisition; no output during
calibration/mode transition; no macro repeat from a held pose, stale input or
dropout; no competing mouse/touch stream. Passing simulation does not waive the
actual writer/client and physical checks.

Engineering targets for comparison, not achieved acceptance: rest RMS at most 5px
on the 1920x1080 output; moving cursor RMS at most 20px on the documented analytic
trajectory; camera-to-event latency p95 below 100ms as a stretch and below 150ms as
an initial budget; no false actions during the existing 60-second relaxed trial;
at least 18/20 intentional clicks and ten successful drags. Targets must identify
input rate, scene, pixel/physical scale, labels and what timing includes. Do not
hide a dropped hand or cancellation to improve error metrics. Full physical
results and human preference decide the receiving configuration.

Iteration stops on a measured decision, a reproducible regression to fix, or an
external physical dependency. It does not repeat a successful run just to keep a
display moving. There is no defensible universal claim of perfection across
hands, lighting, hidden fingers and arbitrary applications.

## Current checkpoint

The initial filter experiment is recorded in
[controller-evolution-2026-10.md](research/controller-evolution-2026-10.md).
Reproduction and visible producers are in
[tools/synthetic/README.md](../tools/synthetic/README.md). The native Cuevanico
goal remains active. Simulation, model selection, mode development, receiving
deployment and human acceptance must be tracked individually as this plan evolves.
