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

## Actual receiving execution of the projected option

Source `50f4041ca5e06979f4f1d1a4b0b824ffb3b2811d`, CI `38835471950`,
passes replay, native and ARM64 jobs. The source-bound artifact is installed only
in the manual experiment slot; the running process executable digest is verified.
A separate session/settings copy enables the projected option. Existing selected
runtime, physical calibration and power policy remain unchanged.

A fresh actual GPU execution of the same full original 60-second window publishes
905 frames, accepts 796, coalesces 109 and expires none. It measures 13.247 Hz,
99.373 ms median/141.165 ms p95 publication-to-meaning. Nine of nine accepted
apparent-contact samples have contact, and zero of nine accepted open samples do;
one reviewed open sample remains coalesced. Every hand carries a dimensionless
ratio and no meter-valued primary metric. Counts, host clock ordering and original
frame provenance validate. Final state is neutral; worker close, terminal view and
successful unit termination are observed. Running and final views fit 1920×1080
and label recorded playback, projected pinch, sensors off and input off.

This is a new inference run: its coalescing and predictions may differ from the
first baseline. The earlier identical-prediction comparison isolates metric choice;
this run qualifies the receiving connection. Neither turns sparse appearance
labels into independent physical/intent accuracy or uninterrupted hold coverage.

Several brief contacts between the sparse one-second samples initially require
inspection. Thirty-six additional original frames around those transitions show
actual rapid finger closure/reopening. Their number alone is not a false-positive
count or evidence of tracker instability. Original imagery, model observations
and these assistant visual judgments remain separately retained.

## Typed synthetic controls

The new typed ratio also runs on three existing independently measured synthetic
trajectories, with the same 0.38/0.40 thresholds at 20 Hz and both 10 Hz phases.
These use retained predictions rather than new rendering/inference: 286 selected
observations across nine actual Rust replays, with neutral Stop/EOF receipts.

| Control | Near contacts 20 Hz | Near contacts 10 Hz phases 0 / 1 |
| --- | --- | --- |
| Distal-straight pinch | 8/9 | 4/5, 3/4 |
| Adverse yaw of that pinch | 0/9 | 0/5, 0/4 |
| Separated-V negative | No contacts | No contacts in either phase |

All fully open holds remain contact-free. The positive loses one near sample at
each cadence relative to its earlier 3D baseline. This reproduces the known
projected-metric tradeoff instead of hiding it behind the favorable recorded
appearance samples. The new metric is not selected as a universal default.

## Original recording dropout and closed return

A second actual GPU replay selects original 140–170 seconds, with unchanged
projected configuration. It publishes 444 frames, accepts 403, coalesces 41 and
expires none. The original 400.040 ms gap is preserved as a 400.175 ms publication
gap. Unlike a synthetic delayed-result control, this interruption already exists
in the recorded source. Its cause remains unknown.

The accepted frame immediately before the gap holds contact. Independent Rust
ticks release it at **301.300 ms source age**, before another source image arrives.
The returning closed hand receives a new track ID, has no contact and explicitly
requires observed open. No subsequent closed frame presses before the first open
observation. Final neutral, model-close/view receipts and successful native
termination pass. No camera or system input is opened.

This is a successful release/rearm control, not uninterrupted drag recovery.
The 300 ms policy necessarily ends that drag. Predicting through occlusion or
resuming ownership without accidental presses remains a separate interaction
problem; do not hide it by extending freshness or claiming touchscreen continuity.

## Qualification boundary and next experiment

This is an opt-in comparison mode, not a universal correction or selected live
profile. The earlier articulated adverse-yaw control still misses all near-hold
frames with the same projected cue. Projection can also hide actual depth
separation. Those counterexamples rule out automatic global replacement of 3D.

Receiving replay now passes its scoped connection checks. Next compare
short continuous pinch/open/occlusion intervals with separate annotations,
preserving misses and early/late release. Test additional recorded positions and
views without fitting each one independently. Keep configuration selection and
uncertainty visible; require the owning finite human/application acceptance
before claiming comfortable system control. No new physical trial is started.

## Dense continuity and another recorded window —2026-10-16

A follow-up inspects every original frame from 51.543–56.471 seconds: 75 images,
55 apparent-contact, 15 open and five uncertain transition/blur frames. Predictions
were already available; this remains assistant appearance review, not independent
human intent or calibrated physical-contact ground truth. All raw bytes/hashes and
annotations remain private and separate from predictions and human label files.

Only adjacent original ordinals with the same reviewed appearance are interpolated,
with a 150 ms maximum original-time gap. Sparse labels and missing-time gaps cannot
become continuous holds. Scoring includes every published source interval, including
coalesced frames, and actual host publication-to-meaning delay. The duplicate earlier
cycle stamp of a frame transition is not counted as an earlier semantic decision.
Independent watchdog ticks retain their own time.

| Actual first-minute replay | Reviewed apparent-contact time | Contact during it | Longest uninterrupted contact within it | Contact during reviewed open time |
| --- | --- | --- | --- | --- |
| Estimated 3D, source6ab609c | 3597.498 ms | 425.510 ms | 425.510 ms | 444.243 ms |
| Projected palm length, source50f4041 | 3596.290 ms | 3412.285 ms | 3412.285 ms | 116.262 ms |

The projected run holds one contact ID throughout that admitted segment. Its
normalized contact trajectory spans x0.721–0.816 and y0.325–0.625; this is movement
intent, not observed desktop delivery or a spatial-accuracy measurement. The open
score covers 872.386/867.811 ms respectively. Both runs have one admitted held
segment within the reviewed pinch, rather than fragmentation after onset. Most
of the projected deficit is initial admission/processing delay. The runs use the
same original video but different actual inference/coalescing and publication
clocks; the prior identical-prediction comparison remains the metric-isolation
control. These selected intervals are not a general pinch success rate.

A fixed next window, original180–260 seconds, was selected before its new GPU
predictions, with unchanged0.38/0.40/300ms/one-hand settings. Actual source50f4041
publishes1202 frames, accepts1087, coalesces115 and expires none:13.589Hz,
99.522ms median/132.389ms p95 publication-to-meaning. Final neutral, model/view
receipts, provenance and successful unit termination pass. Most side-view samples
have visually hidden tips and remain uncertain. Two in-window apparent contacts
and two accepted opens match; another open is coalesced and a nearest boundary
sample falls outside the window. Neither is silently substituted or counted as
a correct observation. Twelve IDs and295 contact observations are diagnostics,
not ownership or physical accuracy. Raw acquisition remains off.

The reusable camera/device-free scorer is:

```sh
python3 tools/score_recorded_replay.py JOURNAL --annotations APPEARANCE_JSON --output RESULT_JSON
python3 tools/test_score_recorded_replay.py
```

Annotations declare `schema:wigglecontrol.recorded-appearance/v1`, an
`annotation_kind` of `assistant_visual_review` or `human_visual_review`, explicit
`limits`, and `frames` containing original ordinal `index`, original `source_us`
and `appearance` (`open`, `apparent_tip_contact`, `uncertain`). Original timestamps
must match the publication provenance. Omitting annotations reports only transport
diagnostics. The tool does not read images, infer labels or score synthetic geometry;
outputs must not overwrite source evidence. Tests cover coalesced-frame/delay
accounting, sparse/gapped labels, provenance/source/clock separation and final-view
termination. The existing geometry scorer still rejects recorded input.

A monolithic base64 image transfer timed out. The finite indexed reader itself
completed successfully. Staging exact JPEGs separately on the receiver and streaming
only missing files with per-frame hashes recovered all75/10,714,656 bytes in33.463s,
without rerunning inference or modifying the original recording. The cache is a
transfer aid, not another recording or serving deployment.

The manual receiving artifact855c22b/CI38837953792 now also fixes the projected
reference label and guide units in standalone feedback and identifies the option
in camera feedback. This changes presentation, not the measured interpretation.
Its source-bound binary is verified; existing configuration/calibration remain
selected as before. Full physical/application acceptance and broader angles,
negative controls, ownership, macros and participant modes remain open.
