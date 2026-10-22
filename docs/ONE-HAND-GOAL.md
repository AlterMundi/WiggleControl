# Goal: deliver the one-hand Linux screen/detail public preview

Native execution status —2026-10-22: the human cleared this Linux goal on
2026-10-21 without completing it. Its requirements remain milestone1 work.
The newly requested active goal is [WINDOWS-GOAL.md](WINDOWS-GOAL.md); do not
resume Linux receiving tests from this historical specification.

Deliver the independently installable experimental Linux one-hand mouse defined
in [LINUX-PREVIEW.md](LINUX-PREVIEW.md) and
[milestone 1](https://github.com/AlterMundi/WiggleControl/milestone/1).
This goal supersedes the open-ended one-hand goal from 2026-10-16, carrying forward
its unfinished reliability/comfort work and actual evidence. That earlier goal
and this file's history remain preserved in Git/native history; they are not
retroactively completed.

## Deliverable

- Comfortable calibrated screen pointing, reliable left/right click, natural
  double click, sustained drag/release and gesture-controlled scroll.
- A deliberate one-shot Escape gesture that releases controller-owned input,
  cancels pending actions and emits Esc once until positive rearm; scoped Stop
  remains independently available. Esc is application-dependent, not a global Stop.
- Held detail precision: smaller pointer displacement for the same hand movement,
  stable rest without creep, useful very small intentional motion and no cursor
  jump on entry/exit. Start with the ring/little-finger-flexed candidate, preserving
  pinch freedom, but qualify actual gesture comfort and nonconflicting bindings.
- Standalone Linux installation/configuration, saved guided calibration, manual
  Start/Stop, optional feedback separate from inference, private/explicit recording
  behavior, pinned/licensed assets, upgrade/uninstall/rollback and a tagged GitHub
  prerelease with honest supported environments and limits.
- Initial physical scope is Pi 5/labwc/Wayland. Prepare/test Linux x86-64 on the
  offered Legion with its actual resources and coordinated human readiness.
  On2026-10-21 the human withdrew Cata because of its camera quality; preserve
  its measurements/recordings, uninstall there, and continue qualification on Legion.
  Windows/macOS and multipoint remain later work.

## Implementation judgment and retained work

Use the functioning headless GPU observation -> Rust meaning -> Linux device
path and existing APIs/helpers. Replacing the perception backend is not a release
prerequisite. Prefer small coherent changes, meaningful regressions and measured
comparisons over new parallel infrastructure or repeating passed checks.

Detail should be an independent movement modifier if that integrates naturally
with button/drag state. Nicolás clarified on 2026-10-18: do not build artificial
isolation or action locks merely to enforce a plan. If changing detail during a
held drag/stroke requires separate added work, defer that extension; selecting
detail before the action is an acceptable release baseline. In either design,
mode changes must not jump the cursor, invent clicks or unintentionally release
a held button. Document and test the chosen behavior; do not queue surprising
changes. Missing modifier evidence is not a confirmed open hand.

Carry forward the prior goal's temporal interpretation: distinguish deliberate
open release from missing/uncertain samples; preserve established drag continuity
within measured bounded same-hand grace, never transferring it to an uncertain
new hand or re-pressing after completed release. Keep source freshness truthful
and the independent writer/process watchdog effective; Stop remains immediate.

Carry forward cursor anchor/click stabilization, adaptive filtering and independent
output cadence. Compare bounded prediction/interpolation, reversals, small motion,
rest jitter, overshoot and lag without buying smoothness through excessive delay.
Reuse unchanged source/device evidence; compare candidates on identical retained
private recordings, journals and independently defined synthetic trajectories.
Keep predictions separate from human intent labels and actual OS/application effects.

Issue #8 (folder remained attached) is closed on Nicolás's explicit acceptance:
it has not recurred; its original cause remains unknown. It is not a blocker or
an instruction to reproduce it unless it recurs. #4 and #9 remain scoped release
dependencies to qualify/disposition from observed evidence, not assumed causes.
#10–#13 own actions, detail, packaging and acceptance; broader umbrella issues do
not impose prayer, multitouch or arbitrary macro completion on this release.

## Operation and collaboration

Preserve accepted calibration/profile/rollback, physical input fallback and
existing power/fan policy. Build away from the Pi when supported; reuse existing
checkout/caches and infrastructure capabilities. Do not infer current screen or
camera availability from an old Farscape restriction or an old ready message.
Coordinate new desktop injection with owned scratch tasks, never human files.
Nicolás reports concurrent realtime-conversation tests on the Pi: continue
camera-free development/replay off-Pi and leave receiving tests/deployment to a
coordinated window; do not pause the whole goal or interrupt the other project.
Serialize resource/performance tests even when sensors differ. Camera acquisition
requires fresh human readiness/manual Start. The 2026-10-21 recording policy in
[RECORDING.md](RECORDING.md) supersedes automatic trial video retention: ordinary
mouse use and calibration default to no video; dataset recording needs an explicit
request and a finite budgeted sample. Research/replay stays camera-free. Do not
leave a sensor session running after its requested scope or install automatic
capture/restarts.

Show useful experiment progress when the desktop is available, without spending
work solely on demonstrations or stealing the screen from the human. Keep optional
video/skeleton rendering out of the mouse-only hot path. No unsolicited peer
contact, private-history export or coordinated global Stop. Preserve origin,
licenses and private host/recording bindings outside public GitHub artifacts.

## Completion and delivery evidence

Maintain a concise current acceptance record, README/INSTALL/BETA/ROADMAP, owning
issues and receiving runbook with implemented, deployed, observed and unqualified
status. Consider substantial reusable findings for collective research without
turning routine delivery into publication bureaucracy or exporting private footage.

Complete only when the scoped interactions pass relevant reproducible regressions,
real application tasks and human comfort acceptance. Reuse the working targets:
at least 18/20 intended clicks, ten completed drags, 60 relaxed seconds without
unintended action and at least five minutes of seated use. Record right/double
click, scroll, Esc and detail small-target/text/slow-drawing outcomes separately.
Verify calibration retry/cancel and four-corner review; preserve previous profiles.

Measure actual capture cadence, latency definitions, jitter/lag, CPU/memory and
sustained thermal behavior. Compare the prepared software-only increment in human
use; distinguish synthetic evidence from physical performance. Camera/lighting
changes are separately coordinated comparisons, not an implicit hardware purchase
or permission to change the current setup. Document demonstrated limits honestly.

Verify clean standalone Linux installation, permissions after a coordinated boot,
update/config preservation and removal/rollback. Publish exact source-bound
artifacts/checksums/licenses and the qualified prerelease after acceptance. A
source commit, CI artifact or attractive hand skeleton alone does not complete
the goal. Do useful camera-free work while a specific human test is pending.
If external input is required, retain the concrete remaining test and evidence
without declaring the whole project complete or reviving resolved incidents.
