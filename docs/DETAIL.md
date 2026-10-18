# Held detail precision — source candidate

Owner: [issue11](https://github.com/AlterMundi/WiggleControl/issues/11),
[Linux public preview](LINUX-PREVIEW.md). Implemented in source on 2026-10-18;
disabled in the Pi candidate and not accepted with a physical hand. The matching
binary was later staged separately for a right-button-only trial; see BETA-1.md.

## Selection and meaning

Controller settings may explicitly enable `"detail": {}`. Omission/null keeps
it disabled; accepted receiving settings and calibration are unchanged. Defaults:

| Setting | Default | Meaning |
| --- | --- | --- |
| gain | 0.25 | Pointer travel / calibrated hand travel in detail |
| rest_radius | 0.001 | Radial play in normalized calibrated hand-workspace units |
| confirm_us | 100000 | Fresh positive pose duration for entry/exit/rearm |
| folded_ratio | 0.55 | Maximum finger chord/path ratio establishing folded |
| extended_ratio | 0.8 | Minimum ratio establishing extended |

The shared native/provider bridge evaluates ring and little world-landmark chains
13–16 and 17–20. Each ratio is MCP-to-tip distance divided by the three bone
lengths. Both folded establishes held; both extended establishes released. Mixed,
intermediate, nonfinite or degenerate geometry is unknown. This ratio is invariant
to rigid rotation and uniform scale of exact geometry; that does not make the
model's estimated pose invariant or physically accurate. No classifier or extra
inference stream is loaded. Thumb/index and thumb/middle remain independently
available for left/right pinch.

Fresh extended evidence must first establish screen mode. Both entry and exit
require confirmation, matching selected hand/source, engagement, input eligibility
and existing confidence. Unknown evidence cancels a pending transition but retains
the confirmed selection. A source gap at least as large as the existing freshness
budget cannot count toward confirmation, even during the longer drag-loss grace.
Existing invalid/stale/ambiguous-owner, Stop and explicit mode/hand transitions
neutralize selection. New/replaced ownership requires extended rearm. Modifier
selection does not upgrade tracking or extend any writer/process deadline.

Controller replay frames accept optional `detail` evidence:

```json
{"hand_id":7,"source_us":250001,"held":true}
```

Native/provider paths supply it from raw landmarks automatically. Controller output
`detail` is the active configuration or null, separately from the exclusive `mode`.
The held modifier applies to calibrated absolute pointer output; session preflight
refuses it on the relative path. Legacy exclusive `precision` mode is unchanged.
Preflight reports the configured candidate; optional feedback labels active Detail
without requiring video/skeleton rendering in mouse-only use.

## Continuous mapping

The modifier operates after saved workspace mapping and before the existing
independent cursor estimator. It changes motion only: no mode transition, button
edge, rearm lock, touch or macro is emitted. Entry and exit anchor to the last
published visible cursor estimate and current interpreted hand position; prediction
velocity is cleared, while source time and click-lock deadlines remain intact.
Changing detail during left/right drag is supported by this same independent path.

In detail, incremental hand displacement is multiplied by gain. A small radial
play region rejects bounded rest noise; slow motion accumulates against the last
accepted hand position rather than being discarded on every frame. Default play
corresponds to at most 0.00025 normalized pointer displacement (0.48 horizontal
pixels at 1920) before motion, with no extra temporal dwell in the motion filter.
Existing cursor smoothing/limits still apply. This is a candidate tradeoff, not
measured human jitter suppression or added camera resolution.

Screen exit uses a continuous piecewise-linear map through the exit hand/cursor
anchor and both screen edges, separately per axis. It preserves the current point
and recovers full edge access; sensitivity after exit can differ on either side
of the anchor. If exit occurs exactly at a workspace boundary while the pointer
is inside the screen, preserving that point and immediately mapping the same
boundary to the edge are incompatible. The first inward move reanchors that axis;
returning to the boundary then reaches the edge. No calibration profile is edited.
Explicit controller reset/owner change restores ordinary absolute mapping.

Detail saturation clamps each emitted position and advances the hand anchor,
preventing edge windup; reversing moves promptly. Screen exit is not a queued
future change, and uncertainty is not interpreted as opening the hand. Existing
same-hand drag holds freeze cursor estimation; new data retain original source
freshness, and the independent writer still owns expiry/release.

## Evidence and remaining acceptance

Device-free regressions exercise source/owner/eligibility checks, positive rearm,
entry/exit confirmation, missing evidence, source gaps, Stop, zero-radius finite
behavior, rest noise, accumulated subpixel motion, saturation/reversal and screen
edge access, including exact-boundary exits. Absolute controller/publisher/writer
regressions cover no-jump entry/exit with smoothing/prediction enabled and disabled,
left/right held continuity, release and brief missing-frame recovery. These produce
event values without opening camera or uinput.

Physical gesture orientation/comfort, unintended detail activation while pinching,
small-target/text selection, slow drawing, actual loss behavior and application
feel remain unqualified. Scroll/Escape are separate pending work. Shared desktop
trials, performance comparison, packaging and human acceptance remain required;
source tests do not complete the milestone.

Local full native-feature validation: 206 tests passed, zero failed, one existing
opt-in benchmark ignored. Native all-target Clippy, format, preview recovery/timer
and controller-workbench checks passed. The aggregate includes unchanged coverage.

Paired private offline replay of two retained trials (248 + 393 accepted
observations) preserved all controller mouse events, interpreted cursor, pointer
state/readiness, held/button selection and primary identity. Eight left press
intents remained eight; neither take selected detail. Raw ring/little geometry
was extended in 609 hand observations and intermediate in two; no folded sample
was found. This supports compatibility on those takes, not deliberate detail
recognition or a measured physical false-activation rate. Original data, derived
outputs and input/binary/source hashes remain private. The replay does not include
intermediate writer ticks or application effects; simulation covers mapping.

## Lightweight control status follow-up — 2026-10-18

Source `/provider/config` now reports confirmed detail selection, configured
bindings and the producer's finite acquisition countdown without reading `/state`
or camera images. The normal mouse controls use this route for SCREEN/DETAIL and
finite/until-Stop feedback; preparation has no acquisition countdown. Opening
camera/input and teardown-unknown states remain distinct from confirmed closure.
Optional `#setup` keeps a scratch task's manual Start disabled until its complete
UI is prepared; unlocking never requests acquisition. Source tests cover these
states and the no-image/manual-Start boundary. Local native-feature suite:207
passed, zero failed, one existing opt-in benchmark ignored; native all-target
Clippy, format, separate-control and preview recovery/timer checks pass. Receiving
standby preparation and the physical detail trial are tracked below.

Receiving follow-up: exact-source258d53e / CI39174893219 passed all three jobs. A
separate manual90s Pi trial is prepared with detail enabled, matching accepted
workspace and private recording; normal beta/profile/power settings are preserved.
Actual1600x950 controls/task content fits within1600x881. Start stayed locked until
the complete task UI was checked; final source state confirms camera/input off,
no preparation and no acquisition countdown. The first task is three extended ↔
ring/little-curled cycles without pinching, checking observed mode and visible
cursor continuity. No physical detail result is claimed by standby preparation.

## First physical attempt and retry — 2026-10-18

The separate source258d53e trial acquired 330 interpreted observations, including
138 with confirmed detail. It entered detail once and returned to screen once;
this is prediction evidence, not human acceptance of precision or comfort. No
image/metadata feedback snapshots were prepared. The trial ended early when the
strict device writer rejected an observation 336251us old against its 300000us
freshness limit (packet transit84us). The retained final report confirms camera
and system input closed, writer reaped and detector closed. This establishes the
cause of this take's early termination; the earlier unrelated failure in issue9
remains undiagnosed.

The provider now checks observation age again immediately before publication.
An ordinary observation with less than20ms of freshness remaining is replaced
with a neutral/rearm transition; no stale press/position is delivered. The
existing device deadline is unchanged, and fresh open evidence is required to
resume. Tick-only output and existing bounded loss holds retain their separate
contracts. This addresses expiry before publication, not arbitrarily long
post-publication scheduling stalls. A distinct discard receipt separates actual
neutral delivery from the earlier interpreted observation record.

Nicolás requested shorter practice poses: hold each pose for1.5seconds instead
of3seconds. This is the task instruction; recognition confirmation stays100ms.
Mouse-only status now polls lightweight configuration every250ms instead of1s
so displayed SCREEN/DETAIL follows selection more promptly without image reads.
The next finite trial retains manual Start, private recording and scoped Stop.


Retry receiving standby: sourcef21a48b / CI39176422738 passed all three jobs;
210 local native-feature tests pass (one existing opt-in benchmark ignored),
Clippy and control-page checks pass. The exact ARM64 artifact is staged in a
separate manual90s trial, with source/observer standby and camera/input off.
Start was locked through task preparation; the1.5s instructions and1600x857
content fit the1600x950 viewport. Accepted baseline/profile hashes match the
prior take. No retry acquisition or physical acceptance is claimed by this
preparation. Private recordings and all original failed-take evidence remain.


## Positive physical detail use — 2026-10-18

Nicolás reports that detail worked very well and confirms that he deliberately
combined clicking and detail in the X menu, with good results. He accidentally
closed the task window. This is positive application-use feedback for the held
modifier with mouse actions; it does not identify the exact X client/desktop menu
or establish whether a mode change occurred during a held drag.

The matched sourcef21a48b retry completed90s successfully:1286 captured frames,
1271 interpreted observations,478 confirmed detail observations and four observed
entry/exit cycles. Selected-node evidence contains three left and two right press/
release pairs, all balanced, with no overflow or partial events. Nicolás explicitly
confirms all five clicks were intentional, including trying detail with clicking.
No prediction is substituted for his statement. The earlier pre-publication expiry
failure did not recur in this take; no delivery-source discard occurred. One
perception lease expired; no image/metadata feedback snapshots were prepared.

Camera and system input closed; detector/writer/provider/observer were reaped,
the virtual mouse was removed and owned test units are inactive. Video and
landmark journals remain private. The terminal report had no error. No automatic
capture/restart or baseline/profile/power change occurred.

Basic detail recognition and deliberate click use now have positive human
feedback. Still qualify rest jitter, small targets, text selection/slow drawing,
edges, cursor continuity, longer comfort and any changes during held actions.
These results do not complete the public preview or a measured false-action rate.


Retained timing for this take (1271 interpreted frames): exposure-source timestamp
to Rust meaning p50=87.043ms, nearest-rank p95=121.427ms, max=179.099ms. Capture
source intervals (1285 gaps) p50=67.982ms, p95=68.025ms, max=208.004ms over89.209s.
These exclude writer/compositor/display/human response and include no CPU/thermal
sample. No matched baseline task was run, so they do not establish a speedup.
