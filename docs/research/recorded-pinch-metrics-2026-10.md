# Recorded hand replay: projected versus estimated 3D pinch

Measured by CompAII on 2026-10-16. This continues [the real-clock worker](realtime-worker-2026-10.md),
[position bias](../POSITION-BIAS.md) and [onset permission](pinch-onset-veto-2026-10.md).
The question is whether missed natural pinches arise from identity loss or from
the chosen distance interpretation. Raw human imagery and exact recording paths
remain private. This report contains selected aggregate technical findings.

## Actual receiving baseline

Source `6ab609c38d867496baf78784e0208660b8d1f2d7`, successful CI run
`38833865550`, executes the first 60 seconds of an authorized retained recording
through the Pi's GPU worker and Rust controller. It republishes 905 original
frames at their original spacing: 797 accepted, 108 coalesced, zero expired.
Accepted cadence is 13.265 Hz; publication-to-meaning delay is 97.091 ms median,
140.306 ms p95. This excludes original exposure/acquisition and OS/application
output. Camera and system input remain off. Final neutral state, model close,
terminal view and successful native process termination are observed.

Twelve track IDs over the minute do not establish twelve failures or people.
Selected boundary frames show fast motion and partial visibility; the main
hand retains one ID continuously for about 25 seconds. Therefore identity churn
alone cannot explain missed pinches in that stable interval.

## Sparse appearance audit and identical-prediction comparison

Twenty-five original frames, about one second apart within that interval, were
visually inspected. Nine show apparent thumb/index contact, ten show an open
pinch and six remain uncertain (including cropped or unclear tips). These are
**assistant-reviewed image appearances**, not human annotations, measured 3D
contact or intended clicks. Predictions had been available before review; the
selection is neither blinded nor an independent validation set. Human annotation
files and original frames/indices are unchanged.

Eighteen reviewed frames were accepted by the measured worker. One reviewed
open frame was coalesced; it is reported as unavailable, not replaced by a nearby
prediction. Replay uses the actual Rust controller, original accepted hands,
IDs, host processing clocks and stale-release tick. The original 3D replay
reproduces every controller output exactly before comparing alternatives.

| Fixed interpretation | Contact on apparent-contact frames | Contact on open frames |
| --- | --- | --- |
| Measured 3D estimate, 45/65 mm | 3/9 | 1/9 |
| Existing projected palm-width default, 0.25/0.40 | 3/9 | 0/9 |
| Previously investigated projected wrist-to-middle-MCP length, 0.38/0.40 | 9/9 | 0/9 |

Parameters come from existing defaults and the earlier projected-metric probe;
no threshold search was performed against these labels. The current onset
permission is preserved. At several apparent contacts the estimated 3D gap is
49–63 mm, above the 45 mm onset threshold, while the same track remains stable.
This supports a useful projected interpretation for this view. It does not prove
that the 3D estimator is physically wrong from a single image, or that the new
interpretation is accurate across other views, people or intended actions.

## Explicit implementation option

Session configuration now supports:

```json
{
  "pinch_3d": false,
  "pinch_2d_reference": "wrist_middle_mcp",
  "pinch_close_ratio": 0.38,
  "pinch_open_ratio": 0.40
}
```

Controller gesture ratios must match the session, as before. Existing sessions
retain their 3D or palm-width behavior. A contradictory 3D selection plus the new
2D reference is rejected. The new dimensionless `pinch_distance_ratio` observation
is distinct from `pinch_distance_m`; supplying both is invalid and releases held
input. Omitted ratios preserve existing wire output. Degenerate reference
geometry invalidates eligibility. Pointer, one-contact and two-contact engines
share confirmation, hysteresis, open rearm and loss handling for this metric.
Recorded previews identify the projected option explicitly. Raw estimated world
points remain available independently; they are not rewritten into fake meters.

The typed ratio reproduces all 801 receipts of the prior offline projected probe
(including start, stale tick, Stop and EOF), without its temporary ratio-as-meter
encoding. This verifies implementation equivalence, not additional accuracy.

## Qualification boundary and next experiment

This is an opt-in comparison mode, not a universal correction or selected live
profile. The earlier articulated adverse-yaw control still misses all near-hold
frames with the same projected cue. Projection can also hide actual depth
separation. Those counterexamples rule out automatic global replacement of 3D.

Next run the explicit configuration through actual receiving replay, then compare
short continuous pinch/open/occlusion intervals with separate annotations,
preserving misses and early/late release. Test additional recorded positions and
views without fitting each one independently. Keep configuration selection and
uncertainty visible; require the owning finite human/application acceptance
before claiming comfortable system control. No new physical trial is started.
