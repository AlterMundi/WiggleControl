# Mesh reconstruction as a pinch-perception comparison

CompAII,2026-10-15. This continues[#7](https://github.com/AlterMundi/WiggleControl/issues/7)
and the[controller roadmap](../ROADMAP.md). It does not qualify a replacement
runtime or complete the active controller goal.

## Question and candidate

Can an articulated mesh estimator distinguish the separated-fingertip macro
from real near-finger geometry where both existing landmark backends underestimate
the gap? Earlier scalar normalization/dwell trials reduced useful coverage.
The[preceding evidence](native-visual-macros-2026-10.md) remains preserved.

[WiLoR's official repository](https://github.com/rolpotamias/WiLoR) links its
[WiLoR-mini inference package](https://github.com/warmshao/WiLoR-mini), which
supports known bounding boxes and reconstructs MANO joints/vertices. Mini is a
packaging simplification, not a small-weight guarantee. This comparison pins
source`ebec42f94c389070cdd7dda6fd1bf0b4a659c960`, public asset revision
`b00adea9a6843bbb4c9042109c5eb29ab2a59dea`, and exact hashes in
`tools/synthetic/wilor-assets.json`. The2,564,989,533-byte checkpoint is local
research data, not a redistributed repository artifact. Original WiLoR model and
MANO licenses remain separate. The MANO wrapper maps its regressed/vertex-tip
points into the21-point order; these remain model estimates, not measured skin.

[HaMeR](https://github.com/geopavlakos/hamer) and
[Fast-HaMeR](https://github.com/hunainahmedj/Fast-HaMeR) remain alternatives;
neither was executed here. The latter documents student architectures, but
availability and performance of an eligible student checkpoint remain unqualified.
No local GPU or receiving Pi performance is inferred from those projects' claims.

## Reproduced method

`tools/synthetic/wilor_probe.py` calls the unchanged provider model/crop API on
explicitly known right-hand skin bounding boxes. This isolates reconstruction;
there is no YOLO/palm detector, confidence estimate, stable identity, gesture
adapter or OS input. All input images are the exact previously retained MHR PNGs.
Independent labels supply skin clearance, bounds and camera projection; complete
world-marker matching verifies the geometry-only macro labels against the images.
Model outputs never define the reference distance.

The crop expands the known box by the provider's2.5 factor and uses its affine
crop/antialias helpers. BGR is supplied because the inspected `WiLor.forward`
itself reverses channels before RGB normalization. The package README's RGB
example differs; numerical parity with a separate official demo remains open.
Two declared focal settings compare provider default5000 with known-renderer
focal scaled to the256px model input. No crop, score or threshold was fitted to
these eight cases. Per-frame crops/transforms, source-image hashes, all21 points,
778 vertices and loading-key differences are retained privately.

The isolated CPU environment pins Python3.10.20, PyTorch2.5.0+cpu, NumPy1.23.5
and the provider-compatible dependencies in `wilor-requirements.lock`.
Python is justified by the actual provider implementation; the ordinary Rust
runtime is unchanged. Chumpy's build initially failed because it imports
undeclared pip; the accompanying uv build-dependency override resolves that
specific error without global changes. All trained model parameters loaded;
47 unused checkpoint keys include training discriminator state. The initial
input-preparation attempt failed before model loading because geometry-only
labels have null image names; the corrected preparation uses original image
names and independently matched geometry. Failed evidence remains retained.

## Completed eight-input probe

Eight known crops, two focal settings each; all16 forward passes completed.
Values below are model joint/vertex-tip gap, compared with selected reference
skin surfaces. Their absolute values need not coincide.

| Input | Independent skin gap | WiLoR default focal gap | Known-renderer focal gap |
| --- | --- | --- | --- |
| Macro open |81.21mm |108.12mm |108.12mm |
| Macro baseline false onset |79.64mm |110.62mm |110.62mm |
| Macro maximum pose |87.52mm |99.45mm |99.45mm |
| Macro opening |78.57mm |108.40mm |108.40mm |
| Near pinch, yaw−35° |2.00mm |24.82mm |24.82mm |
| Near pinch, pitch−25° |2.00mm |25.17mm |25.17mm |
| Near pinch, curled index |1.79mm |32.02mm |32.02mm |
| Near pinch, PIP-dominant index |0.96mm |32.72mm |32.72mm |

Both declared settings separate these selected far/near cases across the existing
45mm comparison boundary, whereas the current landmark estimate at the baseline
false onset is38.21mm. This is promising sample evidence, not a held-out result,
physical contact calibration, measured gesture accuracy or a successful controller.
The near-identical focal outputs are observed here, not a general invariance claim.

Loading took12.74s. Per-pass processing was3.73–5.69s on two CPU threads; peak
reported RSS was5,377,388,544bytes. The finite user task completed under an observed
5GiB memory ceiling, zero task swap and200% CPU quota. These costs are unsuitable
as current interactive Pi qualification; actual GPU/accelerated deployment remains
unmeasured. No Pi model, device, live threshold or calibration was selected.

## Full-trajectory continuation

A separate finite143-input experiment completed after the probe: all61
separated-macro frames,41 yaw−35° positive frames and41 PIP-dominant positive
frames. It uses provider-default focal once per image, preserving source/frame
association. The original first probe and both focal branches remain retained.
All61 macro reconstructions have now completed: estimated gap99.45–110.62mm.
`compare_mesh_metric.py` keeps actual native confidence, eligibility, identity,
cursor and source time while replacing only the matched gap. It verifies all
source-image hashes, complete group/clock association and baseline reproduction,
then uses the existing actual Rust replay/Stop/EOF checks. This conditional hybrid
emits zero contacts on the negative at20Hz and both10Hz phases, versus26/12/13
baseline contact frames. Oracle crops remain an unqualified runtime dependency;
no WiLoR confidence or identity was invented. Offline source-clock replay does
not establish interactive latency.

Both positive trajectories also completed. Their combined20Hz near-hold coverage
is18/18 frames versus17/18 baseline, with zero contacts in all36 fully open
positive-hold frames. At10Hz phase0 coverage is9/10 versus8/10, and phase1 is8/8
versus7/8. PIP phase0 retains the baseline's4/5, so universal perfect coverage is
not claimed. Yaw onset advances to frame15 at13.42mm skin separation, versus the
baseline's frame17; earlier activation is not independently labelled human intent.
Each positive retains one contiguous contact interval. Stop/EOF release passes.
These are synthetic frame counts on two current positive trajectories, not the
complete six-positive corpus, independent human clicks or crossing acceptance.

The143 forward passes took11min33s as a bounded CPU task, with4.356s median
processing and reported peak RSS5,399,281,664bytes. The model process completed
successfully. An observer separately exited while decoding progress written
non-atomically; the completed model/result was not restarted. Final projection
was recovered from retained outputs. The producer now replaces progress atomically,
with a deterministic open-reader snapshot regression test.

## Retained native crops, not oracle bounds

A subsequent eight-input probe replaced only each bounding box with the extrema
of the actual retained native21-landmark predictions. Known right-hand side is
still supplied independently. All pixels/models/settings remain pinned; there
is no new detector or fresh camera capture. Default-focal estimated gaps are:

| Input | Mesh gap with retained native box |
| --- | --- |
| Macro open / false onset / held / opening |107.98 /104.81 /98.89 /105.20mm |
| Near yaw / pitch / curled / PIP-dominant |19.22 /24.39 /31.97 /30.18mm |

All four far and four near examples therefore remain separated across45mm.
This removes oracle bounding boxes from that selected static probe only; full
native-crop trajectories, inferred handedness, acquisition/identity/confidence
and resource feasibility remain unqualified. No runtime or live profile was adopted.

The TV view distinguishes known-crop reconstruction from hand detection and
keeps contact/identity unknown. Independent static crops cannot silently replay
a previous temporal result. The subsequent trajectory producer projects actual
model stages and measured gaps on the owned workbench. Private operator records
hold exact finite task/asset/result paths; no startup service, camera, microphone,
system input or autonomous peer activity was added.

Next: extend to full native-crop trajectories and the remaining positive views,
qualify actual detector/crop and both-hand ownership, then select a feasible
latency/resource architecture and fresh acknowledged physical trial. A plausible
mesh or this selected probe alone does not close any of those gates.

## MobRecon SpiralConv CPU comparison

[MobRecon's author repository](https://github.com/SeanChenxy/HandMesh), pinned at
`3c87e958d4855f890e3884ec94bcfc0f99422c3d`, recommends its2023 SpiralConv
variant for freely captured images. The[CVPR2022 paper](https://openaccess.thecvf.com/content/CVPR2022/papers/Chen_MobRecon_Mobile-Friendly_Hand_Mesh_Reconstruction_From_Monocular_Image_CVPR_2022_paper.pdf)
reports a mobile CPU result; that is neither this variant's measured Pi speed nor
our own validation. The author-linked public download supplied the49,090,033-byte
`mobrecon_densestack.pt`, SHA256
`2b634114dadb324802c9e55a8b7a9236f0e7b476157ac0b175f0eb258d9d276c`.
Source is MIT; no separate checkpoint license was identified in the inspected
download. Assets remain local and MANO provenance stays separate.

The real inference path imports only the provider model and preprocessing;
training graph/mesh packages are unnecessary. One-time topology preparation
uses unchanged `extract_spirals`, sequence9/dilation1 and the pinned template
transforms/joint regressor. Linux OpenMesh1.2.1 has no compatible Python3.10
wheel: a binary-only resolution failed without changing that environment.
An isolated Python3.9.25 preparation uses the published wheel instead, while
model inference reuses the unchanged Python3.10.20/PyTorch2.5 CPU environment.
The complete checkpoint loads strictly; no random trained parameters remain.
Scripts, asset manifest and preparation lock are under `tools/synthetic/`.

The128px RGB crop expands each **retained native-landmark** box1.3 to a square,
following the provider's non-augmented training crop/normalization convention.
Known right-hand side is still supplied independently. Provider0.20 output
scaling and its21-point MANO→MPII regressor produce the joint gap. Its optional
camera registration fits a rigid translation and is omitted because translation
cannot change the Euclidean gap. This is a declared crop comparison, not parity
with an independently executed full demo/detector or a physical scale calibration.

Eight selected static inputs completed first: all four far gaps were67.61–128.55mm,
while yaw/pitch/curled/PIP near gaps were12.36/70.60/44.79/52.26mm. Thus two of four
near examples already fail the existing45mm close boundary. A subsequent complete
307-frame run covers the61-frame negative and **all six**41-frame positive
trajectories, using matching native boxes and original image hashes. The macro
geometry is independently matched with zero world-marker error. The actual Rust
replay preserves native confidence, eligibility, identity, cursor and source time,
replacing only the gap at unchanged45/65mm thresholds:

| Positive trajectory | Baseline near-hold contacts | MobRecon near-hold contacts |
| --- | --- | --- |
| Yaw−35° |8/9 |9/9 |
| Yaw+35° |9/9 |0/9 |
| Pitch−25° |9/9 |0/9 |
| Pitch+25° |9/9 |9/9 |
| Curled index |9/9 |5/9 |
| PIP-dominant index |9/9 |0/9 |
| Total at20Hz |53/54 |23/54 |

Negative contact frames decrease26→0 at20Hz,12→0 at10Hz phase0 and13→0 at
phase1. Positive near coverage decreases26/30→8/30 and23/24→10/24 at those10Hz
phases. Positive fully open holds retain zero contacts (108/60/48 scored frames).
Baseline reproduction and Stop/EOF release pass on every group. These are
conditional synthetic frame counts, not independent human intent or real-time
execution. They reject this unchanged-threshold reconstruction as a direct
replacement; suppressing false actions by losing useful pinches is insufficient.

The full run completed successfully in65.887s wall/78.377s CPU under two-thread,
200% CPU,2GiB memory and zero-task-swap limits. Median forward/postprocessing
was114.18ms, p95512.04ms; provider load took0.61s. Cgroup peak was303.3MiB;
the process's retained `ru_maxrss` reports403,570,688bytes, a distinct measurement
whose difference is not resolved here. Timing excludes image crop/decode,
detection, camera registration and event delivery. VPS contention/quota effects
remain in the measurements; no Pi or optimized-runtime speed is inferred.

Actual model stages/images/gaps were projected during the finite run. The owned
TV page was observed at1920×1080 with a loaded source image during inference and
after completion. It reports reconstruction with unknown contacts rather than
inventing detector or ownership outputs. Both finite model/projector processes
exited; no camera, microphone, system input or Pi runtime was activated.

Decision: keep the working local ONNX controller and its profile. WiLoR remains
the stronger tested accuracy comparison on its smaller corpus; MobRecon is
substantially lighter but fails useful coverage. Next compare an actually
available accurate/accelerated candidate and qualify complete native crops and
ownership before proposing live adoption. Do not retune thresholds to these seven
trajectories or declare every mesh estimator accurate because it reconstructs a
plausible hand. Physical comfort and crossing/occlusion remain separate gates.
