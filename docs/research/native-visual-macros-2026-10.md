# Native visual macro connection and qualification failures

CompAII,2026-10-15. Continuing the
[trained-classifier investigation](gesture-classifier-2026-10.md) within the active
[controller roadmap](../ROADMAP.md). This source deliberately separates a working
software connection, synthetic model behavior, named-pose UI fixtures and physical
acceptance. None completes the full controller goal.

## Implemented connection

An explicit optional `visual_macros` mode setting selects the existing530KiB
converted model, a named neutral and a minimum score0.8..1. Missing/null retains
the previous disabled behavior. Config-only validation requires macro bindings
and rejects a neutral also bound to an action, without loading models/sensors.
The native camera loop and new `controller-sequence` share the actual classifier
adapter, hand selection and policy. No second hand detector/runtime is added.
Startup loads the model before acquisition; all camera trials still require
fresh human readiness and private recording.

Unknown/unsupported/low-scoring poses are absent evidence, never neutral.
Positive neutral additionally requires the actual observed hand to be eligible,
confident and open. Active/pending input takes priority. Ownership follows the
existing primary/claim in one-hand modes; two-contact mode keeps the visible
macro owner across array reorder. A missing/replaced/ambiguous hand must rearm.
The one-hand claim remains inactive in two-contact mode, matching its UI.

The real engine now exposes observation-derived neutral/hold progress, cooldown
and cancellation. A two-second historical action receipt retains its actual
resulting mode/running state; it is separate from one-shot output and cannot
repeat delivery. Native feedback shows uncertainty, pinch priority and applied
or unavailable actions, and clears progress when delayed. External media/launcher
adapters remain unimplemented. Source local mode/pause handling is not an OS effect.

## Actual image controls

All experiments use the same exported model,0.8 score floor and45/65mm estimated
3D pinch interpretation. The saved/live receiving thresholds/profile were not
changed. File replay explicitly uses source time as its observation clock; model
processing time is retained separately and is not camera latency.

The four earlier corrected-pinch views contribute164 frames. Every original
hand observation and contact set matches the retained pre-macro experiment
exactly, with zero macro actions. Per-view `input_busy` counts are15/16/20/15.
The negative-pitch view achieves positive-neutral arming on12 frames, but ordinary
pinching still takes priority. These are controlled negative examples, not a
real-world accidental-action rate or proof of intended macro recognition.

A separate single-rig61-frame open→two-finger pose→hold→open sequence was added.
Index/middle remain extended; ring/pinky flex toward the palm and thumb folds.
The first parallel-finger recipe is retained as a diagnostic, not accepted V
geometry. The next recipe explicitly spreads index/middle by−0.20/+0.15rad.
Both reject dorsal long-finger displacement beyond1micrometre; this guard does
not certify joint anatomy, soft tissue or whole-hand self-collision. All evaluated
skin remains in frame. `--finger-spread 0` retains the earlier diagnostic branch.

| Actual native image path | Pose result | Neutral/input result | Macro actions |
| --- | --- | --- | --- |
| Initial parallel-finger reference | Open_Palm36/61; None25/61 | No score≥0.8 Open_Palm; never armed | 0 |
| Spread V /front view | Victory17/61, including13≥0.8 | Open neutral remains approximately0.58–0.59; never armed | 0 |
| Same spread V /pitch−25° | Open_Palm42/61; None19/61 | Open_Palm35≥0.8 and rearm works, but the V is missed;31 input-busy frames | 0 |

The original full MediaPipe Tasks implementation was also run on the initial
open/parallel-finger stills: Open_Palm0.623 and None0.940. This does not isolate a
cause or prove native equivalence; it shows that failure is not unique to the
converted-classifier route. Native front spread-V hold scores are approximately
0.878–0.886. The selected tilted view was chosen from earlier open-palm evidence,
not an unseen held-out accuracy test. No score floor was lowered to force success.

### Separated-fingertip contact counterexample

The tilted V sequence emits26 semantic contact frames,20..45. Its13 maximum-pose
hold frames all remain contacts. Rig thumb/index marker separation there is
128.44mm, while estimated model gap is44.54–47.55mm after an earlier onset below
45mm. This is a synthetic contact counterexample, not a measured human error rate.

An independent9-frame FCL probe of the same recipe measured selected distal
thumb/index skin patches. Their maximum-pose surface separation is87.52mm,
without selected-patch intersection. That probe's maximum-pose decoded image is
pixel-identical to frame30 of the61-frame sequence, so the independent distance
refers to the actual model input. The nine sampled skin gaps span77.63–87.52mm;
do not treat those nine samples as full whole-hand collision qualification.
The existing [skin adapter](../../tools/synthetic/skin_clearance.py) remains
python-fcl0.7.0.11 with its retained triangle/transform controls.

This makes pinch/pose exclusion a concrete next problem. Do not call a copied
recognizer class an independent intent label, blame a missing coordinate factor
without evidence, or suppress more input and claim comfortable control. Preserve
coverage and ordinary pinch detection when evaluating2D/3D/hybrid interpretations.

## Feedback and lifecycle qualification

The actual native page passed the1920x1080 TV check with document height1080 at
armed, candidate hold, applied Precision, held-input priority and unknown rearm.
No console exceptions were observed. Its finite fixture used the actual Rust
controller/delivery encoders with explicitly fictional named pose evidence and
retained synthetic images. It was labelled **UI TEST / sensors off / system input
off**, stopped, and the owned research workbench restored. This proves the view
and semantic controls, not image recognition or compositor/camera effects.

Rust tests cover missing/low/ineligible neutral, array-order ownership versus
replacement, inactive one-hand claims in two-contact mode, observation-based
progress and a historical receipt without repeated actions. Existing release,
freshness, mode, delivery and preflight cases remain. Node tests include delayed
progress, actual versus unavailable action receipts and manual-mode changes that
must not rewrite a past action's displayed result.

One local existing test failed an immediate global-port assertion after owned
preview drop. The replacement check asserts scoped Stop, cleared owned snapshot
and release of worker/owner clones, without assuming that no other parallel
preview can reuse the port. The original failure output is retained; its exact
cause was not independently established. The corrected full native suite and
Clippy passed. No global Stop or unrelated desktop/service restart was used.

The TV projection separately encountered an actual SSH timeout during rendering.
The render completed; the observer terminated before inference. Its partial
directory/log remains. Only missing inference was resumed against the retained
render. The projection helper now retains transport errors and continues local
model work; a broken pipe on cleanup cannot mask an original model failure.
Source-independent orchestration controls reproduce offline display with both a
successful fake model and a failing fake model, without network/models/sensors.
Those controls validate orchestration only. Transport writes are not receiving
display acceptance; reports explicitly keep that qualification separate.

## Next required qualification

1. Resolve pinch output on separated-fingertip macro geometry. Compare candidate
   interpretations on identical retained predictions and independently measured
   controls; score transition false contacts and useful held coverage separately.
2. Qualify both neutral and action recognition in the same continuous hand
   sequence across views. Current image controls do not establish a successful
   timed macro. Keep strict-default failures and development choices visible.
3. Test independent real neutral, intended macro, relaxed-hand and pinching
   sequences, then actual receiving feedback/action delivery and comfort. Require
   fresh prepared human acknowledgment before acquisition; retain private footage
   and separate predictions from human intent.
4. Keep selected runtime/config/calibration, the separately prepared8c569ae
   camera-only trial and all experimental candidates distinct. A source-connected
   feature or UI fixture is not accepted live adoption.
