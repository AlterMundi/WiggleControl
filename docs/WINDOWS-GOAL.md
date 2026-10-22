# Goal: deliver the shared-core Windows 11 x64 one-hand preview

Implement and deliver the independently usable Windows 11 x64 one-hand preview
specified by [WINDOWS-PREVIEW.md](WINDOWS-PREVIEW.md) and
[milestone 2](https://github.com/AlterMundi/WiggleControl/milestone/2), issues
[#18](https://github.com/AlterMundi/WiggleControl/issues/18),
[#19](https://github.com/AlterMundi/WiggleControl/issues/19),
[#20](https://github.com/AlterMundi/WiggleControl/issues/20),
[#21](https://github.com/AlterMundi/WiggleControl/issues/21) and
[#22](https://github.com/AlterMundi/WiggleControl/issues/22).
Nicolás explicitly requested creation and activation on 2026-10-22. The native
harness goal is the live execution state; this file is its maintained specification.

## Objective and approach

Use the port to make WiggleControl modular: one common Rust implementation of
raw-landmark interpretation, identity/freshness, gesture meaning, calibrated
screen/detail motion, filtering/prediction, scroll/Escape and delivery safety.
Move OS-dependent acquisition, input APIs/event translation, display/DPI,
clock/storage primitives, process lifecycle and launch/packaging behind a small
platform layer. Inspect existing configuration, helpers, APIs and tests first;
extract/reuse actual implementations, not another Windows controller.

Target only Windows 11 x64 on Legion, initially using the Logitech C920e at its
negotiated native Full HD/wide format. Reuse the selected pinned inference/assets
where compatible. Verify actual Windows RTX use and resources; Linux acceleration
evidence and hardware presence do not establish Windows behavior. A Chromium-free
backend, other OS versions and multitouch are not requirements for this goal.

Carry the merged #16 detail pinch-posture/drag correction and #17 recording-off
input-startup fix into the shared path. Neither was physically accepted in the
successful Linux Legion trial. Keep current gesture/calibration contracts,
positive rearm, bounded same-hand drag grace, no-jump detail transitions, source
freshness, click stabilization, bounded prediction and independent writer/process
watchdog cleanup. Do not introduce action locks solely to enforce a plan.

## Deliverable and verification

Produce a complete source-bound Windows candidate with simple ordinary-user
launch, camera/output selection, full-screen guided calibration with visible
camera framing and all four corners, saved profile/rollback, manual Start/Stop,
truthful status and optional camera/skeleton feedback independent of inference.
Provide left/right/natural double click, sustained normal/detail drag/release,
comfortable screen/detail pointing and stable rest/tiny motion, two-axis
content-grab scroll with selected release inertia, deliberate one-shot Escape
and robust per-button failure cleanup. Preserve physical input fallback.

Normal use/calibration must not record video. Opening/installing never starts
camera or input, and closing feedback must not silently terminate tracking.
Prefer a portable ZIP/executable without WSL, developer tooling or Daimon/private
services; document any actual browser dependency. Pin/hash/license distributed
assets; verify install/update/removal/profile preservation and rollback.

Add meaningful Windows-native CI, shared-core replay and adapter tests while
keeping relevant Linux native/replay/package checks green. Distinguish source
proof, package/preflight checks, installed behavior, application receipt, measured
performance and human acceptance. Reuse valid evidence; do not repeat unchanged
checks or misrepresent source tests as physical qualification.

Qualify on Legion with fresh human readiness after preparation, scratch tasks and
no change to display configuration or power/fan policy. Use existing proposed
18/20 intended clicks, ten completed drags, 60 relaxed seconds without unintended
actions and five-minute seated comfort targets; explicitly exercise detail drag,
right/double click, both scroll axes, Esc, calibration retry/cancel and loss/Stop/
producer/writer failure. Retain labelled intent and user comfort separately from
predictions. Actual GPU renderer, camera cadence, whole-pipeline CPU/RSS, timing
definitions and sustained thermals inform optimization and published limits.

Update maintained roadmap/context/architecture/acceptance and receiving runbooks
through already-authorized owning routes. Deliver checksums, licenses, current
quickstart/known limits and the experimental prerelease after real acceptance.
Never claim Windows support or goal completion from CI artifacts alone.

## Continuity and limits

The prior Linux goal was explicitly cleared, without completing milestone 1.
Preserve its open release/physical work and accepted profiles; keep #8 closed
unless the human reports recurrence. Linux release is not a dependency of this
Windows goal. Do not silently resume that cleared goal or mutate Linux receiving
state merely to advance Windows. Linux source compatibility is required.

Use the ordinary checkout/caches and existing authorized infrastructure. Windows
remote access is not established by historical Linux SSH; use an actual supported
receiving or direct package-delivery route. Prepare source/CI/package work while
physical acceptance awaits the human. Preserve pending/uncertain effects.
No automatic camera startup, unattended input, global Stop, background peer
attention, driver/global GPU/power changes or other-machine deployment is implied.
