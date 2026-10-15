# Pinch onset permission without losing temporal confirmation

Investigation by CompAII, measured2026-10-15. This continues [#7](https://github.com/AlterMundi/WiggleControl/issues/7)
and the [screen-space comparison](wilor-cpu-execution-2026-10.md#screen-space-hypothesis-and-the-cancellation-trap).
It is an opt-in source experiment, not selected receiving configuration or
physical acceptance. No camera, microphone or OS input is used.

## Question and implementation

Can a projected-distance cue reject the separated-V counterexample without
erasing useful open rearm or restarting the existing3D confirmation clock?

Replacing the metric with pixel gap/palm length delayed useful pinches. Setting
`pinch_eligible=false` destroyed armed state and suppressed every positive.
The new optional observation field `pinch_onset_allowed` instead vetoes a new
button-down after the raw metric has satisfied its existing confirmation time.
It leaves observed-open rearm, the confirmation clock, release hysteresis,
cursor movement and existing holds unchanged. Eligibility still cancels genuine
loss/invalidity. Omitted permission defaults to true and is omitted from output
when true, preserving the legacy default wire output.

Session option `pinch_onset_max_screen_ratio:0.38` derives that permission from
native pixel distance thumb-tip→index-tip divided by wrist→middle-MCP distance.
The value0.38 is retained from the earlier screen experiment, not adjusted to
the following outcomes. This ratio is dimensionless, not corrected physical
distance. A degenerate/nonfinite palm estimate invalidates pinch eligibility;
it cannot be a valid open or held observation. The option is absent by default.
Pointer, direct contacts and controller image-sequence replay share the same
observation conversion. Existing45/65mm thresholds are unchanged.

This design deliberately confirms the raw metric continuously, then checks
permission at onset. It does **not** require the projected cue to pass throughout
the confirmation interval. A single allowed projected sample after a long raw
close can therefore admit a press; this is an experimental tradeoff, not a
calibrated probability of intent. An onset veto also does not release an already
held contact whose raw metric remains incorrectly close. Macro recognition and
its neutral/input-priority policy remain separate unfinished work.

## Broader articulation controls

Two new asset-bound version1 recipes keep index MCP flexion at1rad while its
PIP/DIP are0.1/0.1 or0/0rad:

- `poses/mcp-distal-light-v1.json`: minimum selected distal skin gap2.343mm.
- `poses/mcp-distal-straight-v1.json`: minimum gap1.502mm.

"Distal straight" does not mean an entirely straight index aligned with the
palm: the MCP remains flexed. The other three fingers stay open. A distance-only
thumb-marker objective within the existing seven-joint recipe found these
candidates; an earlier fixed displacement-vector objective was unnecessarily
restrictive. Candidate selection used geometry before observing detector output.
Both41-frame trajectories pass framing, signed forward-index motion and no
intersection of the independently sampled distal skin patches. An independent
world-space rest-palm normal also checks nonnegative thumb2/3/tip displacement
through every frame: normalize `(index-MCP − pinky-MCP) × (middle-MCP − wrist)`
at rest, then dot each marker's displacement from rest with that normal. These
are selected-surface and signed-motion checks, not
whole-hand collision freedom, certified joint anatomy or human intent.

The asset is the existing MHR FBX with SHA256
`d66fbca815bcde6532f728f1f63071003c5d43ff44f56e263a0807baec1ae055`;
Blender4.5.3 CPU and the pinned FCL contact environment are reused. Both rendered
trajectories detect41/41 frames, retain one ID, emit one contact interval and
cover9/9 near-hold frames under the existing raw3D interpretation. No completely
open hold emits a contact. The index-backbend references rejected by Nicolás
remain invalid historical controls; these recipes do not revive them.

## Matched development replay

Eleven trajectories comprise nine positive controls and two separated-V controls:
the previous six pinches, left-position pinch, these two new articulations,
the original tilted negative and the newer yawed negative. Total491 input frames.
All run through the actual Rust controller at20Hz and both10Hz source-clock
phases, with baseline reproduction and Stop/EOF release checks. The original
negative uses independently measured labels whose full world-marker trajectory
matches exactly. Other source predictions and images are retained unchanged.

| Metric | Raw3D | Onset veto0.38 |
| --- | --- | --- |
| Original negative contact frames20Hz /10Hz phase0 /phase1 |26 /12 /13 |0 /0 /0 |
| Newer yawed negative contact frames |0 /0 /0 |0 /0 /0 |
| Nine positives: near-hold coverage20Hz |80/81 |80/81 |
| Nine positives: near-hold coverage10Hz phase0 |39/45 |39/45 |
| Nine positives: near-hold coverage10Hz phase1 |35/36 |35/36 |

For all nine positives and the yawed negative, **every per-frame contact decision**
matches baseline at all three cadences, including onset and release. This is
stronger than matching aggregate coverage, but retains baseline misses.
The metric-replacement alternative0.38/0.40 loses one of nine near frames on each
new articulation at20Hz and one of four on10Hz phase1; the onset veto avoids
that extra delay in these controls. All fully open holds remain contact-free.
These development counts are not independent human click rates.

The actual configured image→native inference→observation→controller path also
completed on the61-frame original negative and41-frame new distal-straight
positive. It produces zero negative contacts and the same complete controller
output as baseline for the positive. After excluding the optional permission
field, accepted hand observations match the earlier inference exactly. The
negative's geometry replacement has zero marker error. This verifies the native
session connection rather than relying only on edited JSON observations.

## Additional fixed-parameter configurations

Before rendering or inferring them, a private registration fixed the same0.38
onset permission, raw45/65mm/40ms confirmation and four new views of the new
distal-straight recipe: yaw±35° and pitch±25°,41 frames each. Geometry-only checks
precede rendering. Both10Hz phases are included. These broaden configuration
coverage on the same rig; they are not independent anatomy or physical intent.
All164 rendered inputs detect one hand; independent geometry matches exactly,
selected skin remains in frame/disjoint, and projection transports report no
errors. Detection success does not imply usable pinch interpretation:

| New distal-straight view | Near contacts20Hz: raw→veto |10Hz phase0: raw→veto |10Hz phase1: raw→veto |
| --- | --- | --- | --- |
| Yaw−35° |1/9→0/9 |0/5→0/5 |0/4→0/4 |
| Yaw+35° |9/9→9/9 |4/5→4/5 |4/4→4/4 |
| Pitch−25° |9/9→9/9 |5/5→5/5 |4/4→4/4 |
| Pitch+25° |9/9→9/9 |4/5→4/5 |4/4→4/4 |

The adverse yaw changes the raw model gap35.87–55.00mm and screen ratio
0.3413–0.6045 during the same1.502mm skin-clearance hold. The raw engine accepts
only its last near-hold frame; the veto removes that remaining onset. All
10Hz samples already miss it. This is a retained regression, not a successful
general correction. No threshold is tuned afterward. Anatomical and visibility
limits of this synthetic pose still apply; it is a configuration stress case,
not independently labelled real-human error.

At negative pitch the veto delays20Hz onset50ms, from a20.686mm selected skin
gap to10.510mm; maximum-hold coverage remains complete. Other new-view contact
trajectories match baseline. Fully open holds remain contact-free. The fixed
new configurations therefore reject unconditional adoption despite the promising
development result. A robust interpretation needs view/pose/temporal evidence
beyond a single projected threshold. Preserve raw and candidate results.

The finite native connection plus four geometry/render/inference comparisons
complete successfully in406.741s wall/418.257s CPU,1GiB cgroup peak and zero task
swap under a200%CPU/2GiB limit. The separate earlier82-frame articulation
experiment is also terminal. No model/renderer remains scheduled afterward.

## Existing paired-contact controls

The same veto also runs through actual Rust replay on six retained61-frame
paired sequences: separated, narrow separated, same-side separated, same-side
crossing, crossing and wide/deep crossing, at20Hz/both10Hz phases. Every contact
ID/coordinate trajectory is exactly unchanged.20Hz raw baselines reproduce the
retained outputs; Stop/EOF release checks pass. These use the earlier optional
palm-point recovery/experimental separation guard where recorded, not a new
selected receiving policy. Their retained partial holds, occlusion cancellations,
premature onset and unknown ownership are **not repaired** by this permission.
An unchanged failure is not successful multi-participant control.

## Reproduction and remaining qualification

Use the existing pinned synthetic/contact environment and exact asset. Generate
each new recipe with `render_hand_reference.py ASSET NEW_DIRECTORY --hand-only
--closure 1 --closing-steps 41 --skin-check --pinch-pose POSE`, first adding
`--measure-only` in a separate directory. Add `--closing-view yaw--1`, `yaw-1`,
`pitch--1` or `pitch-1` for the fixed-view controls.

For identical retained predictions, run
`compare_pinch_interpretations.py RESULTS... --binary BINARY --settings SETTINGS
--onset-screen-ratio 0.38 --output NEW_REPORT`. The optional permission is added
to a copy; the raw distance and all original source data remain unchanged.
Sources must be ungated; the helper rejects already-vetoed observations rather
than mislabelling a gated replay as the raw baseline.
Use `--geometry MATCHED_LABELS` for a single result lacking sampled skin.
The native source path uses `controller-sequence SEQUENCE SESSION SETTINGS`,
with the optional session ratio above and matching gesture settings. All these
commands are camera/device-free. The existing visible-motion observer projects
actual rendering/inference and retains explicitly labelled saved replay on TV.

Remaining: repair the new adverse-yaw failure, broader views/articulations/backgrounds, noisy
projected cues, held-contact release under conflicting metrics, macro neutral
and action recognition, receiving ARM64 resources and fresh acknowledged physical
acceptance. Preserve selected runtime, calibration, recording policy and original
pending trial; this source increment alone selects none of them.
