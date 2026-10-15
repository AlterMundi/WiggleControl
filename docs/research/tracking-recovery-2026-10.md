# Second-hand recovery and duplicate inference

CompAII,2026-10-15. Continues [paired closure](controller-evolution-2026-10.md#paired-closure-overlap-and-input-cadence-2026-10-15),
[#6](https://github.com/AlterMundi/WiggleControl/issues/6) and the [roadmap](../ROADMAP.md).
This is an opt-in source experiment, not a receiving runtime or physical acceptance.

## Observe the actual rejection path

Camera-free sequence entry points now collect bounded current-frame tracking
diagnostics: input tracks, whether palm search ran, attempted crops/confidence/
points, skipped proposals and duplicate matches. Rejected candidates stay separate
from accepted `perception`/`hands`; they cannot become contacts or improve those
counts. Normal acquisition does not collect these candidate arrays. A missing
source image emits null diagnostics rather than a previous frame's evidence.

Diagnostics reproduce all accepted hands, raw perception, contacts and ambiguity
on the original separated/crossing122-frame baseline. The final config factory
also reproduces the61-frame crossing after the policy plumbing changes. At
crossing frames39/44 the legacy path skips two detector proposals before hand
inference while retaining only one hand. At frame25 it attempts one and skips
another. This identifies actual broad-crop suppression, not proof that every
skipped proposal represents a real second hand.

The old rule compares expanded crop centers within0.4 times the larger crop side.
Those crops include much more than a palm. A new optional session setting,
`"palm_deduplication":"palm_landmarks"`, instead compares the detector's wrist
and four MCP points with the existing hand's corresponding points. Their ordering
comes from the [pinned OpenCV Zoo adapter](https://github.com/opencv/opencv_zoo/blob/47534e27/models/handpose_estimation_mediapipe/mp_handpose.py).
RMS error is normalized by maximum palm width/wrist→middle-MCP length; below0.25
matches an existing prediction. It does not align away a real spatial difference.

The first version exposes a regression in the yawed-V single-hand control: a
redetected crop reconstructs the existing hand again, causing three `pair_overlap`
cancellations. The two predicted21-point sets differ by only0.028–0.040 palm units.
The corrected candidate rejects that duplicate inference when all-point RMS is
below0.10 and handedness-score difference below0.2. Handedness is a consistency
check, never participant identity. These are development heuristics, not universal
anatomical bounds or independent physical identity proof. Unit controls distinguish
nearby same-side hands, wrong point correspondence and degenerate scale.

Defaults remain `"roi"`; pair separation remains0.8 unless explicitly configured.
`config-check`, controller preflight, replay metadata and future recording/session
metadata report the selected policy. Existing positive-open rearm, identity checks,
pair-overlap cancellation, freshness and Stop/EOF behavior are unchanged. The new
policy does not preserve held state or transfer contacts through an uncertain gap.

## Original paired controls

At the already experimental1.2-palm separation guard:

|61-frame scene | Legacy two-hand frames | Candidate two-hand frames | Stable-contact result |
| --- | --- | --- | --- |
| Separated |61 |61 |All41 stable counts preserved |
| Narrower separated |61 |61 |All41 stable counts preserved |
| Crossing |35 |40 |Dual-contact near hold remains0/13 |
| Wider/deeper crossing |36 |44 |Dual near remains0/13; later open/near improves0/4→4/4 |

At the unchanged0.8 guard, original crossing coverage is36→41 frames, but the
premature57.60mm second-contact onset remains. Changing search alone does not fix
#6. The separate1.2 guard prevents that onset in these controls; no global threshold
or receiving policy is changed. More accepted detections and more ambiguity
cancellations coexist; neither implies uninterrupted useful control.

At10Hz, source images are retained exactly with both sampling phases; clock and
geometry association are preserved. Original-crossing dual coverage improves
18→20/31 in phase0 and remains17/30 in phase1. Wider/deeper coverage improves
19→22/31 and18→22/30. No dual near hold is recovered. Its later open/near20Hz
improvement does not survive either10Hz phase (both remain0/2). Separated/narrower
stable counts match baseline at each cadence. Do not hide this cadence cost in an
aggregate detection count. These are offline source-clock tests, not measured
camera-to-event latency or sustained Pi resources.

The corrected candidate preserves accepted hands, contacts, ambiguity, full
controller state and visual-pose results across **409 single-hand frames**:
six pinches, the separated macro and the two recent yaw/position configurations.
The first version's yawed-V regression is retained; the corrected409-frame run
has no accepted-output differences. The macro's26 false-contact frames still
exist: this is a tracking change, not a solved pinch/pose interpretation.

## Two same-side hand references

The renderer adds `--pair-chirality same`; default remains `opposite`. The new
configuration clones a right hand without reflecting it, allowing two same-side
references as a prerequisite for two participants each using one hand. It does
not identify participants or prove real people can operate together.

Two new61-frame separated/crossing scenes use unchanged forward-flex geometry,
independent closure phases and disjoint evaluated skin-depth intervals. Geometry-only
checks precede rendering. Full trajectories of **both** reference hands match
their measured geometry exactly; all skin stays in frame with no selected distal
patch intersection. The existing matcher previously checked only root/first-hand
world points. It now rejects changes or missing skin evidence in the second hand,
with regressions for changed markers/IDs. Individual joint limits, whole-hand
collision and human intent remain separate qualification.

At1.2 guard, the separated same-side scene retains61/61 dual detections,41/41
correct stable counts and no observed ownership changes, matching legacy. The
crossing improves33→42/61 dual detections, with ambiguity1→3, while the dual near
hold remains0/13. Six of seven near/open holds match in both policies.16 returned
track-association frames remain unscorable through overlap, and tracks receive
new IDs after the gap. Zero observed contact-owner changes does not establish
hidden identity or held-contact continuity.

At10Hz same-side crossing, dual coverage improves18→22/31 and16→20/30; dual near
holds remain0/6 and0/7. Same-side separated counts match legacy at both phases,
including the phase1 partial holds (near/open3/4, near/near6/7). Perfect temporal
coverage is not claimed. These new configurations retain the same rig/skin/model;
they broaden chirality/geometry coverage, not anatomical or human generalization.

## Operation and remaining work

All tests are finite, camera/device-free, on the VPS and visible through the owned
workbench during baseline, candidate and new-scene processing. Additional headless
controls verify the same private artifacts. The two new geometry/render/native
scenes finish in401.918s wall/399.346s CPU, roughly1GiB cgroup peak under2CPU/2GiB/
zero-task-swap limits. Replayed suites are bounded separately. Shared-host CPU
contention and differing output transport preclude a new performance ranking;
Pi cost is unmeasured. No model worker or new service is left running.

Retain this as an opt-in search candidate. Next qualify actual receiving resources,
broader hands/backgrounds/occlusion and explicit useful recovery/repress behavior.
Participant roles still require manual claims/workspaces and real acceptance;
two same-side references are not person identification or two OS cursors. Keep
the working binaries/calibration and the prepared original human trial separate.
No new camera, microphone, OS input, power-policy or selected-runtime change follows.
