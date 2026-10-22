# One-hand Windows 11 x64 preview

Human-selected on 2026-10-22. [Milestone 2](https://github.com/AlterMundi/WiggleControl/milestone/2)
owns this delivery; [WINDOWS-GOAL.md](WINDOWS-GOAL.md) is the native-goal specification.
This is the delivery plan. Shared-core and adapter Windows CI builds now exist;
the user-facing preview and actual Legion acceptance remain pending. Current
source boundaries and qualification evidence: [PLATFORM-LAYER.md](PLATFORM-LAYER.md)
and [WINDOWS-INPUT.md](WINDOWS-INPUT.md).

## Scope and starting evidence

Target only Windows 11 x64 on the offered Legion, initially with the Logitech C920e.
Nicolás reports that the Linux Legion controller worked very well. Its accepted
Full HD calibration and qualitative use are valuable evidence, but do not establish
Windows camera, GPU, input, display mapping or calibration acceptance.

The detail-drag correction in [#16](https://github.com/AlterMundi/WiggleControl/pull/16)
and video-off input-startup correction in [#17](https://github.com/AlterMundi/WiggleControl/pull/17)
are merged and source-tested; neither was physically validated in that successful
Linux trial. Carry both into the shared implementation. Normal use and calibration
must never need video recording for mouse input to work.

The previous Linux native goal was cleared by the human on 2026-10-21, without
completing [milestone 1](https://github.com/AlterMundi/WiggleControl/milestone/1).
Its unfinished release/physical checks remain discoverable; Windows does not wait
for that release, and Windows completion cannot close Linux receiving evidence.
On 2026-10-22 the human reports Legion running Windows 11 x64. Existing Linux SSH
access is not a Windows access route. Prepare delivery through an actually supported
route; do not silently boot Linux, change drivers or alter the display arrangement.

## Shared software and platform boundary

Use this port to improve modularity, as explicitly requested by the human. Keep one
Rust controller, not a separate Windows implementation of gesture mathematics.

| Shared core | Platform adapters |
| --- | --- |
| Observation validation, hand identity, landmark-to-pinch/posture interpretation | Camera enumeration/acquisition, negotiated format and transport |
| Engagement, left/right/double click, drag/release and same-hand bounded grace | Native input API/event conversion and observed API results |
| Screen mapping, filtering/prediction/stabilization, held detail, scroll/inertia and Escape | Actual monitor geometry/DPI and explicit target selection |
| Guided sampling, four-corner verification and adoption/rollback policy | OS file opening/ownership/atomic replacement implementation |
| Logical output packets, source freshness, sequencing, held-state/rearm and watchdog policy | Monotonic clock access, writer/process lifecycle and platform cleanup |
| Camera-free fixtures, replay and useful portable controls | Launch/browser discovery, optional integration and packaging |

The #18 extraction is merged: shared observation, controller, calibration,
profile-store policy and generic delivery state compile on Windows and Linux.
Linux evdev/uinput implementations remain behind compatible imports. Windows
provides QPC, profile-file primitives, physical display mapping, SendInput conversion
and independent writer supervision. Camera/session/worker launch integration
still belongs to #20; output receiving acceptance remains part of #19/#22.
OS event codes, `/dev` paths, Unix file semantics, V4L, systemd and compositor
commands stay out of the reusable gesture/calibration/delivery core.

Interfaces should express actual needs, with small modules/traits where useful.
Do not build a generic plugin framework or replace a proven algorithm for symmetry.
Retain Linux replay/command/config behavior and relevant native/package checks.

## Implementation order

1. [#18 — Shared core](https://github.com/AlterMundi/WiggleControl/issues/18):
   extract the coupled portable logic; establish shared replay and Windows core CI.
2. [#19 — Windows output](https://github.com/AlterMundi/WiggleControl/issues/19):
   implement mouse/key output and selected-display mapping behind that boundary.
   [Win32 SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
   is the starting API to evaluate; test results and limitations are not assumed.
3. [#20 — Capture and calibration](https://github.com/AlterMundi/WiggleControl/issues/20):
   adapt actual camera/worker/lifecycle to Windows and feed the same raw-meaning core.
4. [#21 — Distribution](https://github.com/AlterMundi/WiggleControl/issues/21):
   build a complete source-bound portable package, quickstart and update/removal path.
5. [#22 — Legion acceptance](https://github.com/AlterMundi/WiggleControl/issues/22):
   validate actual Windows use and publish the experimental prerelease with evidence.

Reuse pinned MediaPipe/provider assets and the working Chromium GPU approach where
Windows qualification supports them. Inspect existing helpers and upstream APIs
before choosing a dependency or writing an equivalent. Browser or native camera
transport remains an adapter choice, not a reason to duplicate the controller.
A future Chromium-free backend remains separate work, not a Windows prerequisite.

## Required experience

- Ordinary Windows 11 x64 use without WSL, Rust/Python development setup, private
  Daimon services or a required AI connection. Prefer a portable ZIP/executable.
  If an installed browser remains required, make its qualified dependency explicit.
- Manual Start/Stop, actual camera/input/video status and physical mouse fallback.
  Opening/installing the application never starts sensors or system input.
- Full native-aspect camera acquisition: request/report the C920e Full HD format
  and actual frame cadence, without a square crop or an unverified resolution claim.
- Guided calibration shows what the camera sees, useful progress/countdowns and all
  four actual full-screen corner checks. Retry/cancel/close work after failure;
  only explicit Save adopts a fully verified replacement. Keep previous profiles.
- Screen pointing, left/right/natural double click, sustained drag/release and held
  detail use the same shared behavior. Detail is independent of button state where
  natural; do not add artificial action locks. Include the formerly failing detail drag.
- Fist grabs content in both axes; moving open-hand release may coast, stationary
  release does not. Preserve the selected scroll policy and explicit one-shot Escape.
- Mouse-only use can hide camera/skeleton rendering without terminating inference.
  Qualify Windows occlusion/minimize/background behavior and avoid CPU-cost claims
  before measurement. No video recording by default; explicit dataset capture, if
  provided, retains [RECORDING.md](RECORDING.md)'s bounded opt-in policy.
- Preserve source timestamps, identity, bounded queues/prediction, no press on
  reacquisition, independent writer watchdog and release of owned buttons/keys on
  loss/Stop/failure. Do not loosen these checks to make a physical test pass.

## Qualification and delivery

Native Windows CI and camera-free tests establish source/build behavior; a complete
package and no-acquisition smoke check establish preparation. Neither proves
camera access, event receipt in an application, RTX inference or user comfort.
Report those as separate results with the exact source/artifact/settings.

Prepare the full receiving candidate before asking the human to be ready. Live
camera/input trials require a new ready acknowledgment for that test. Use scratch
applications, preserving actual files, physical inputs, display configuration and
power/fan policy. Explicit until-Stop use is allowed when requested, not inferred
from availability. Never start capture as a side effect of installation.

Use existing proposed acceptance targets: at least 18/20 intended clicks, ten
completed drags, 60 relaxed seconds without unintended actions and five minutes
seated use. Record right/double clicks, detail drag, small targets/rest/tiny motion,
scroll/Escape, calibration cancel/retry and failure cleanup separately. Human
comfort and labelled intent are distinct from generated event counts.

Measure actual GPU renderer/delegate, inference versus transport/output timing,
source cadence, whole-pipeline CPU/RSS and sustained thermals. Linux RTX timings
are reference evidence, not proof of Windows acceleration. Report monitor/DPI and
browser/camera/runtime scope honestly; publish no universal FPS/latency claim.

Deliver source-bound artifacts/checksums, licenses, quickstart, known limits and
update/removal/rollback. Publish a clearly labelled experimental prerelease only
after receiving acceptance. Keep private host bindings, serials, calibration and
recordings out of Git and packages. Preserve Linux regressions and its open work.

Other Windows versions/architectures, macOS, two-hand modifiers, multipoint touch,
prayer, infrared/wearables, kernel-driver input and game/anti-cheat compatibility
are outside this milestone. Windows integrity-level limits require explicit
qualification; broad elevation is not a default substitute for a working adapter.
