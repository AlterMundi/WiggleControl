# Implemented pinch fusion and native dual-crop qualification

CompAII,2026-10-15. Continues [crop/Tasks evidence](roi-stability-and-tasks-2026-10.md)
and [#7](https://github.com/AlterMundi/WiggleControl/issues/7).
All experiments below use retained synthetic images/observations, without camera,
microphone or OS input. Source implementation is experimental and default-off;
selected receiving runtime, calibration, thresholds and original trial remain intact.

Subsequent [fixed-tensor and native mixed-model work](fixed-tensor-mixed-model-2026-10.md)
adds an optional converted landmark Session, actual10Hz improvement and new
unscored crossing contacts. This report retains the earlier shared-model result.

## Result and decision

Actual Rust fusion now exists. A native shared-model dual-crop producer improves
20Hz near-hold coverage107/117→115/117 with the same0.38 onset cue and zero
contacts across two negative trajectories. It is **not ready for general adoption**:
fresh10Hz tracking still loses an adverse-yaw hold, held-release estimates can
disagree, and paired crossings still lose both contacts. Continue efficient
perception/model comparison and new release/conflict controls before receiving
selection. This is evidence of progress, not a completed touchscreen controller.

## Meaning and lifecycle

`Hand.secondary_pinch` is one bounded, matched same-hand/frame hypothesis, not a
second hand or participant. Each estimator independently requires positive-open
rearm and continuous40ms close confirmation. Alternating estimates cannot add
their close durations. An OR of independently held states produces one button
transition/contact per hand, retaining the primary cursor and continuous contact
ID during an overlapping estimator handoff. Missing/weak/ineligible evidence or
changed estimator identity resets that estimator. Invalid values or a timestamp
different from the parent frame cancel interaction; primary loss, stale, Stop,
ambiguity and mode/identity transitions retain their cancellation behavior.

Pointer readiness accepts any usable open estimate. Macro neutrality requires all
usable estimates open: an unarmed closed secondary cannot silently permit a macro.
This distinction has a controller regression test. Seven additional fusion tests
cover independent confirmation, overlap handoff, changed estimator, loss/quality,
malformed/wrong-frame evidence, Stop/stale and four estimates still producing only
two stable contacts. These are semantic/replay tests, not physical evidence.

This policy can prolong a hold while either estimate remains wrong and closed.
The onset cue never releases an existing hold. Unknown ownership cancels rather
than transferring the held contact. Neither fusion nor model confidence certifies
anatomical contact or intent.

## Conditional Tasks replay is now actual Rust meaning

`replay_pinch_fusion.py` attaches matched retained Tasks distances/onset permission
to original native observations, then executes actual Rust `mode-replay`. Native
ownership, cursor, eligibility and quality stay native; inherited quality is
explicitly **not Tasks confidence**. The helper verifies full matched geometry,
one contact, continuous held ID, separately confirmed component-union equality,
legacy baseline and Stop/EOF. It is a conditional single-hand metric experiment,
not a complete live Tasks backend or qualified multi-hand provider matching.

| Conditional fusion | Near contact frames | Negative contacts | Open-hold contacts |
| --- | --- | --- | --- |
|655 source inputs,20Hz |117/117 |0 |0 |
|20Hz observations subsampled to10Hz, phase0 |61/65 |0 |0 |
|20Hz observations subsampled to10Hz, phase1 |52/52 |0 |0 |
|Fresh10Hz tracking, two hard positives/both phases |17/18 |0 |0 |

The last row is143 actual10Hz input frames across six sequences, including the
two phases of the negative V. It differs from subsampling20Hz tracking. A late
held release still yields one contact above50mm skin separation in each
negative-pitch phase; negative-contact zero refers to the separated-V controls,
not every separated frame during positive opening transitions.

## Native producer: two crops, one model and owner

Optional session `dual_crop:true` requires `pinch_3d:true`. It reuses the existing
ONNX Session/weights: primary normal bounds plus an independent proximal-palm
recursive ROI. Discovery seeds from the primary result; subsequent secondary
inferences use their own preceding proximal ROI on the same RGB frame. This adds
landmark inference, not another model stack or independent statistical evidence.
Corresponding palm cost below0.25 and the existing handedness/identity check bind
the secondary to the primary owner. A secondary also matching another current
owner is dropped. Primary ambiguity/crowding/loss rules still apply.

Secondary confidence, eligibility, world gap and optional screen cue come from
that secondary inference. Raw secondary points and bounded attempt diagnostics
remain separate from accepted hand identity. Conversion is shared by native
pointer/touch/controller paths and camera-free sequences. No receiving camera
session has executed this candidate.

Source follow-up: review found that the legacy single-pointer path still called
single-crop `infer` when prayer was disabled. It now selects the existing guarded
`infer_many` path when dual crop is explicitly enabled, without enabling prayer.
Controller/touch/offline image results below already used that path and are
unaffected; this fixes optional live-source wiring, not physical qualification.

Thirteen positive41-frame and two negative61-frame trajectories reproduce655
native image/controller inputs. Existing source geometry/images and45/65mm
thresholds are retained. The0.38 cue applies separately to each estimate.

| Same20Hz image set | Near contact frames | Negative contact frames |
| --- | --- | --- |
|Primary raw baseline |108/117 |26 |
|Primary with0.38 onset cue |107/117 |0 |
|Actual native dual crop with0.38 cue |115/117 |0 |

Every open-hold frame remains contact-free. Two adverse yaw trajectories each
retain8/9 near frames; remaining positives retain9/9. Across655 inputs, primary
observations and ambiguity remain exact after removing deliberately added
secondary/onset fields. VPS processing median42.262ms,p9583.410ms includes image
decode/classifier; it is not Pi performance or a matched universal backend ranking.

A prior temporary proximal-only probe gave8/9 adverse-yaw and9/9 negative-pitch
near contacts, but raw separated-V produced25 contacts; the cue removes them.
Its irregular10Hz subsampling motivated independent fusion rather than replacing
the primary cursor/metric. No standalone proximal-only option remains in source.

## Actual10Hz and paired image-path qualification

The native dual-crop producer was rerun from images at actual10Hz source times,
with independent recursive histories and unchanged cue:

| Hard scene | Phase0 near contacts | Phase1 near contacts |
| --- | --- | --- |
|Adverse yaw |0/5 |3/4 |
|Negative pitch |5/5 |4/4 |
|Separated V |0 contact frames |0 contact frames |

All open holds stay contact-free. Negative-pitch opening still has one held
contact above50mm separation in each phase. Together these results reject treating
the20Hz improvement as robust low-cadence perception. Changing source cadence
changes recursive crops; semantics alone cannot repair absent evidence.

Four61-frame paired sequences use their retained experimental1.2 pair guard and
`palm_landmarks` deduplication, not the selected live policy. Opposite- and
same-side separated hands retain61/61 dual detections, two stable track IDs and
all41/41 expected stable-hold contact counts each. No associated open-hand contact
or observed contact-owner transfer occurs. Opposite crossing retains40/61 dual
detections; same-side crossing42/61. Both have0/13 dual near-hold counts, unknown
intervals and reacquisition with new IDs. Unscored occlusion is not ownership
success. At most two contacts occur; all387 qualification input observations
retain their original primary hand/ambiguity evidence. Paired median processing
ranges61.7–78.5ms on the VPS. No Pi latency, participant recognition or physical
acceptance follows.

Actual processing progress/results were projected through the owned experiment
browser. A subsequent1920x1080 DOM check observes the completed final61-frame
same-side crossing, loaded image, measured summary and explicit saved replay.
This proves receiving page rendering, not continuous human attention or sensor
activation. No synthetic input was injected into the desktop.

## Reproduce and continue

The image experiments use native release SHA256
`c7337c9097aa1665008e5786b32d4f0fcc0d392f3a82818ab9fb9b4c1ca59979`;
its per-file source manifest and reports are retained with the experiment.
The final source additionally exposes secondary evidence in future live recording/
feedback metadata; this metadata change was tested without another image run.

Use `dual_crop:true` in an experimental copy of the existing native3D session,
optionally `pinch_onset_max_screen_ratio:0.38`. Run existing
`controller-sequence SEQUENCE SESSION SETTINGS` or `sequence-check SEQUENCE SESSION`;
these commands open no camera. The native source records selected policy and raw
secondary evidence. `replay_pinch_fusion.py NATIVE TASKS --binary BINARY --settings
SETTINGS --output NEW_REPORT` reproduces conditional meaning with pinned provider
inputs; it does not execute a model or input device. Public source/model/license
pins remain in the preceding report and synthetic manifest; raw evidence stays
in the authorized experiment store.

Full native-feature tests, clippy and focused Python attachment/mismatch tests
pass. Remaining work: new conflicting hold/release trajectories, broader hands/
backgrounds, crop/model comparison at actual target cadence, macro exclusion and
recognition, receiving resource/latency measurement and acknowledged physical
calibration/application acceptance. The broader modes, macros and participant
roadmap remains active. Neither#7 nor the overall goal is complete.
