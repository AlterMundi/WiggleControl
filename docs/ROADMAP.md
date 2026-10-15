# Camera controller roadmap

Updated 2026-10-15. This is the current development entry point. The original
six-milestone design remains in [DEVELOPMENT.md](DEVELOPMENT.md); dated receiving
evidence remains in [QUALIFICATION.md](QUALIFICATION.md) and human acceptance in
[ACCEPTANCE.md](ACCEPTANCE.md). [CONTEXT.md](CONTEXT.md) preserves history.

Implementation tracking: [#3](https://github.com/AlterMundi/WiggleControl/issues/3),
with existing runtime acceptance [#1](https://github.com/AlterMundi/WiggleControl/issues/1)
and shared calibration/prayer [#2](https://github.com/AlterMundi/WiggleControl/issues/2).

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
| Perception | Native ONNX CPU palm/hand inference; ROI tracking; up to two tracks; confidence/ambiguity; source-linked models and ARM64 measurements | Temporal model accuracy, crossing/occlusion, varying hands/backgrounds; pinching pose bias remains |
| Gesture meaning | Open-before-press; hysteresis/confirmation; drag/scroll; loss/stale/Stop release; independent contacts; replayable named macro hold/rearm/cooldown | Comfortable precision/latency balance; receiving mode-arbiter/writer qualification, visual macro classification and actual action adapters |
| Mapping | Shared diagonal calibration, guided review, saved profile/rollback, matching native overlay target | Complete corners/comfort acceptance; participant-specific workspaces |
| Delivery | Real virtual mouse on Chromium Wayland/VLC XWayland; type-B two-contact writer and touch-aware client scale/rotation; selected local compositor patch restores all five scripted stationary touch→mouse cases in owned Wayland Chromium | Camera-driven multitouch acceptance; other clients/locks/grabs and wheel-only handoff (#5); actual fallback per application |
| Operation | Finite and selected until-stop controls, private rolling recordings, existing local panel, independent physical input | Final receiving configuration, actual reboot qualification of the prepared input grant (#4) and sustained accepted-hand resources |
| Simulation | Static MHR controls; hand-only yaw;2016 full-workspace /5760 calibrated analytic cases;25-frame single/paired rendered trajectories through actual perception/gestures; pinned Tasks CPU comparison;41-frame forward-flex skin-clearance/closure replay; TV progress/results | Full anatomy/contact intent, diverse articulated views, held-contact ownership, participants and broader coverage |

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
