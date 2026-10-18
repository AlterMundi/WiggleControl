# Held detail precision — source candidate

Owner: [issue11](https://github.com/AlterMundi/WiggleControl/issues/11),
[Linux public preview](LINUX-PREVIEW.md). Implemented in source on 2026-10-18;
not installed on the Pi or accepted with a physical hand.

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
