# Offline articulated hand references

This optional research tool renders actual3D geometry using Blender's Python API.
Python is used for the renderer's supported scripting interface; the input device,
perception and gesture runtime stay Rust. It is not installed on the integrator,
opens no sensors and does not change the production models or camera calibration.

## Optional pose classifier

The [pose investigation](../../docs/research/gesture-classifier-2026-10.md) records
actual converted-model parity, native image probes and the remaining macro gates.
Python is used only for the providers' model-format/reference APIs; Rust inference
reuses the already selected ONNX Runtime1.22.0. No TensorFlow dependency, second
hand detector, model training or receiving installation is needed for that route.

Download the versioned task bundle from `BUNDLE_URL` in
`export_gesture_classifier.py` into an explicit research directory. The exporter
requires SHA256`97952348cf6a6a4915c2ea1496b4b37ebabc50cbbf80571435643c455f2b0482`;
it rejects other bytes and existing output directories. Preserve MediaPipe's
Apache-2.0 source license/attribution and the downloaded artifact's provenance.
No model or example photograph is committed to this repository.

```sh
uv run --no-project --python 3.11 --with-requirements tools/synthetic/gesture-requirements.lock python \
  tools/synthetic/export_gesture_classifier.py gesture_recognizer.task NEW_MODEL_DIRECTORY
uv run --no-project --python 3.11 --with-requirements tools/synthetic/gesture-requirements.lock python \
  tools/synthetic/gesture_provider_reference.py gesture_recognizer.task NEW_REFERENCE.json IMAGE.jpg OTHER_IMAGE.jpg
uv run --no-project --python 3.11 --with-requirements tools/synthetic/gesture-requirements.lock python \
  tools/synthetic/check_gesture_classifier.py NEW_MODEL_DIRECTORY NEW_PARITY.json --fixture NEW_REFERENCE.json
```

The reference tool uses explicit local images, records misses and retains the
provider's own landmarks/category. The parity checker compares original LiteRT
and converted ONNX embeddings/probabilities on256 seeded random tensors, three
constant controls and optionally those reference tensors. This proves particular
tensor/preprocessing equivalence, not independent gesture accuracy or macro intent.
The two unused declared embedder ports are confirmed by graph dependencies and
perturbation; do not generalize that finding to other model versions.

Build with `--features native-vision` for these sensor-free Rust commands:

```sh
wigglecontrol gesture-check IMAGE CONFIG NEW_MODEL_DIRECTORY/gesture_classifier.onnx
wigglecontrol gesture-landmarks NEW_MODEL_DIRECTORY/gesture_classifier.onnx RUNTIME < normalized-image-xyz.jsonl
wigglecontrol sequence-check SEQUENCE CONFIG --gesture-model NEW_MODEL_DIRECTORY/gesture_classifier.onnx
```

`gesture-landmarks` accepts bounded16KiB JSONL records with `width`, `height` and
exactly21 `landmarks` triples. They are the provider's **unmirrored source-image
normalized XYZ**, not metric world points, raw ROI coordinates or2D points with
invented zero Z. Preprocessing currently qualifies full images at zero rotation.
Native hand inference preserves projected image Z in source-pixel X units;
the original world-gap/contact calculations are unchanged.

`sequence-check --gesture-model` adds `pose_prediction` to each raw perception
hand. No macro engine, sensor or OS output is enabled. `run_visible_motion.py`
accepts the same optional `--gesture-model`; the existing TV workbench shows its
actual category/score during inference and explicitly saved replay. Its report
pins the model bytes and rejects changes during a run. Unknown `None` is never
positive neutral evidence, and `Open_Palm` alone does not establish an open pinch.

### Optional native macro connection

`wigglecontrol controller-sequence SEQUENCE CONFIG SETTINGS` uses the actual
controller/ownership/positive-neutral policy over finite images, with explicitly
simulated source time and no sensor/device. The mode settings' `visual_macros`
select the model and neutral, as documented in
[CONTROLLER-NATIVE.md](../../docs/CONTROLLER-NATIVE.md). Actual file probes have
not yet established a successful timed macro; keep the qualification failures in
[the integration report](../../docs/research/native-visual-macros-2026-10.md).

`run_visible_motion.py --controller-settings SETTINGS` selects this full path
instead of direct-touch replay. It pins the settings/model bytes and projects
actual controller status alongside predictions; it cannot combine separate
gesture-model or perception overrides. A display transport failure is retained
in `projection-errors.json` and `results.json`, without discarding local model
completion. Cleanup cannot replace a real model error with a broken-pipe error.
The sink can still delay a write until its bounded SSH timeout; this is not a
fully asynchronous projector. Transfer success is not receiving display proof;
use actual viewport inspection separately.

`render_hand_reference.py --gesture victory --closing-steps 61 --closure 1 --hand-only`
generates a separate research-only two-finger articulation, with explicit flexion,
V abduction and signed long-finger displacement guards. `--finger-spread 0`
reproduces the earlier parallel-finger diagnostic. `--closing-view pitch--1`
selects the existing−25° control. Use `contact-requirements.lock` plus
`--skin-check` for independent selected-patch distances. These recipes do not
qualify anatomical V intent or whole-hand collision and must not be counted as
natural-pinch references. The selected tilted maximum pose has87.52mm distal
skin clearance yet emits contact intent under the current3D-only interpretation.

## Articulated reference geometry

The selected MHR LOD1 FBX includes an articulated body and fingers. Fetch the
public release in `mhr-asset.json`, verify the archive hash, and extract only
`assets/lod1.fbx` and its `assets/LICENSE.txt`. Verify the member hash too; retain
that license with generated asset bundles. No model binary is tracked here.
The hashes identify the observed downloaded release; they are not signed claims.

The experiment used the isolated, cached Python3.11/bpy4.5.3 route:

```sh
uv run --no-project --python 3.11 --with-requirements tools/synthetic/requirements.lock python \
  tools/synthetic/render_hand_reference.py PATH_TO_LOD1.fbx NEW_DIRECTORY
```

This is an explicit optional dependency installation/render command, not a skill
startup action. Reuse an existing compatible Blender environment when available.
No system packages, GPU or cloud rendering account are needed for the qualified
CPU run. The output directory must be new; dependencies and generated scenes,
frames and labels remain outside source Git.

The script retains editable `source.blend`, eleven PNG frames, camera projection,
world/camera marker coordinates, asset hash and render settings in `labels.json`.
It varies horizontal/vertical position, distance, yaw and pitch independently,
keeping finger articulation and reference marker gap fixed. It checks that the
known gap remains invariant within1 micrometre across these rigid transforms.
Blender evaluates the FBX scale into world-space metres; do not read a bone's
parent-relative `head` as an armature/world coordinate. Use evaluated pose heads
and explicit object/camera matrices, as this script does.

Pass the generated frames through the existing `image-check IMAGE CONFIG` command
for cold-detection measurements, or use `--repeat 10` for a cold-to-warm
trace on identical input. The portable batch helper retains missed detections:

```sh
python3 tools/synthetic/evaluate_reference.py OUTPUT_DIRECTORY CONFIG \
  --binary target/debug/wigglecontrol --repeat 10
```

Use `position-sweep` separately to distinguish
exact-pixel translation from changed camera projection and tracking recrops.
Inspect detection misses and visible framing; don't discard difficult cases.

The first actual eleven-frame CPU run completed and all known rig markers stayed
inside the image. A detector accepted an initial rendered hand at confidence0.993.
Actual aggregate model measurements belong in `docs/POSITION-BIAS.md`.
This controls geometry, not photorealism or a physical camera's optics. Joint
markers require independent anatomical/skin-tip mapping before contact claims.
The default keeps an open pose. Optional `--closure VALUE` in the range0..1 now
uses the forward-flex recipe described below. The original frozen solver candidate
was rendered at eleven views but was later rejected for dorsal index bending.
Retain its dated numerical outputs as diagnostic history, not valid pinch labels.
Applied joint angles and actual marker coordinates remain explicit. Neither the
new geometric guards nor this tool qualify a live correction or change45/65mm
thresholds.

## Visible experiments

The TV view accompanies actual work; it is not a separate presentation project.
`build_workbench.py INPUT_ROOT NEW_DIRECTORY` packages the retained three-pose
sweeps, raw source/measurement JSON, source hashes and MHR license into a private
offline bundle. It expects `reference-sweep`, `middle-sweep`, `near-sweep` and the
original asset license. Open its `index.html` in the receiving browser: it needs
no web server, camera, model runtime or input device on the display host.

`run_visible_probe.py --help` runs a bounded central/yaw comparison using60
identical inferences per case. `run_visible_yaw.py --help` renders9 new hand-only
yaw views and evaluates10 identical inputs per view, using the existing cached
Blender environment and Rust inference binary. Exact SSH host, identity and
display directory are explicit receiving-local arguments, never repository
defaults. The user authorizes the chosen display and operation; these tools grant
no access and start no attention service.

The producer writes actual progress to an owned SSH projection process. The RPi
reads an atomically replaced local `run-state.js` and renders images/metrics.
Pending inference, retained results, new renders, completion and delayed updates
are distinguished. The new yaw sweep retains the correct final image/reference
with its measurements and a chart across measured angles. This is a small
experiment view, not a new Field/research-browser product or universal activity
dashboard. The independent local sensor/activity panel remains visible.

`run_visible_yaw.py --resume` reuses the completed render manifest and valid
ten-frame model outputs in an interrupted directory; it evaluates only missing
cases. Completed result directories are not overwritten. Partial model output
requires explicit reconciliation. Keep raw JSONL, render labels/source scene,
transport failures and final results private and separate from source Git.

For new controlled sweeps, `render_hand_reference.py --hand-only --yaw-steps 9`
keeps the rest of the body out of the experiment. See the dated correction in
`docs/POSITION-BIAS.md`: the old negative-pitch images are body-occluded invalid
hand-visibility inputs, not valid detector-miss cases. Joint-marker distances
alone never qualify visibility or physical contact.

## Controller iteration and temporal images

Current scope, mode expansion and evidence gates: [the roadmap](../../docs/ROADMAP.md).
`cargo run --locked --bin wigglecontrol-simulate -- NEW_DIRECTORY` runs2016
camera-free observation scenarios through the actual Rust gesture engine, across
fixed/adaptive filters, input rates, noise amplitudes and seeds. Its output
contains raw trajectories and separate rest/motion error and pinch timing.
This is analytic observation simulation, not a rendered vision accuracy test.
It includes fast/fine motion and a speed-floor variant of the adaptive cutoff;
the floor does not suppress small spatial movement. Development seeds11/29/101
and the separately evaluated313 seed are explicit in the retained report.
`select_controller_profile.py REPORT` selects using development rows and fixed
rest/fast/fine error budgets, then reports the selected candidate's held-out rows.
It does not alter live settings. The original432-case and intermediate1080-case
revisions remain separately retained evidence.

`render_hand_reference.py ASSET NEW_DIRECTORY --hand-only --motion-steps 25` renders an open
hand's lateral/yaw trajectory and writes a bounded `sequence.json` manifest with
20Hz source time. Optional `--pair-motion` instead renders independently
transformed mirrored open hands crossing in image space. Their evaluated visible
skin camera-depth intervals must not overlap; this rules out interpenetration
between the two meshes. It does not certify joint anatomy or physical contact.
The original close-depth pair was retained as an unqualified stress fixture;
only the corrected separated pair has that explicit geometry check.

`wigglecontrol sequence-check SEQUENCE CONFIG` uses native-vision to consume those
images sequentially with one perception instance and the real direct-touch
meaning. It never opens a camera or virtual input device. Source timestamps are
simulated; processing time is measured separately. Manifests are version1 with
1..10000 strictly ordered frames, at most1MiB, relative image paths confined to
the manifest root and at most4096 pixels per image axis. An image value of null
is missing acquisition and releases gesture state. The EOF record is neutral.
Raw observations, landmarks, world estimates, ambiguity and contact intent remain
distinct from renderer truth.
Optional `--rediscovery N` on `sequence-check`, or the visible runner, varies the
native missing-hand detector interval within1..30 frames. The ordinary runtime
default remains five; this is an experiment setting, not a new live default.

`build_controller_workbench.py REPORT NEW_DIRECTORY --asset-license LICENSE`
packages the retained filter report with `controller.html`, its result data,
the3D asset license and a ready state. Copy that owned offline directory through
the receiving host's established authorized route and open its `index.html`.
The same display/transport scope and private-artifact rules above apply.
Optional repeated `--vision-report LABEL REPORT` adds source-hashed retained
comparison summaries. The original filter report hash stays in the package.

`run_visible_motion.py RENDER_DIRECTORY CONFIG NEW_RESULT_DIRECTORY --help`
documents exact receiving-local SSH/display arguments. It can accompany an
already-started owned render, then evaluates its manifest once. It bounds render
observation to ten minutes and retains local errors. Rendering publishes actual
completed images; inference publishes each actual model result. Completion
persists and optional replay explicitly says it uses saved frames. The transport
is an owned finite stdin writer, not a new background service. Source images and
the full report are retained privately; `run-results.json` holds the current run.

For the optional independent MediaPipe Tasks CPU comparison, the exact pinned
Python3.11 environment is `mediapipe-requirements.lock`. This use of Python wraps
the provider's native supported API for comparative research; it does not replace
the Rust runtime. Cache that environment with uv, then pass
`--mediapipe-model TASK_FILE --model-sha256 EXPECTED_SHA256` to the visible runner.
The script checks version1.1.1 and the explicit model hash. The model download
and source manifest are kept in the private experiment directory; no assets are
committed. This backend reports raw detector arrays, not stable identity,
ambiguity classification or Rust contact semantics. Those unavailable fields
stay unknown. Compare the same retained images and do not confuse debug-build
cost, model stack or sequential VPS load with a controlled Pi benchmark.

### Calibrated analytic benchmark

Pass an optional read-only profile after `NEW_DIRECTORY` to exercise actual
workspace mapping. The calibrated expansion runs5760 cases and retains the
profile, camera-space observations, explicit noise space and development/held-out
seed split. The report stays private when it contains a receiving profile.
`select_controller_profile.py REPORT` ranks development data and reports held-out
failure separately; it never adopts settings. The workbench builder includes that
selection and shows its trace and budget outcome. The current calibrated grid
fails the fixed gates; see the owning evolution report. No sensor or OS output is
needed for these commands.

### Articulated closure and independent skin clearance

Use the optional pinned `contact-requirements.lock` environment for mesh distance:

```sh
uv run --no-build --no-project --python 3.11 --with-requirements tools/synthetic/contact-requirements.lock \
  python tools/synthetic/test_skin_clearance.py
uv run --no-build --no-project --python 3.11 --with-requirements tools/synthetic/contact-requirements.lock \
  python tools/synthetic/render_hand_reference.py PATH_TO_LOD1.fbx NEW_DIRECTORY \
  --hand-only --closure 1 --closing-steps 41 --skin-check
```

This uses the renderer's native Blender API and FCL's supported triangle-distance
API; neither is installed in the Rust controller or on the display host. The
qualified optional package is python-fcl0.7.0.11 (CPython3.11/x86_64 Linux wheel).
`--closing-steps9..121` is separate from rigid/yaw/pair trajectories. Source time
is20Hz; the open/near holds and per-frame articulation are retained explicitly.
`--skin-check` measures each reference hand separately. It selects distal
thumb/index skin triangles, records witnesses/intersections and does not certify
whole-hand collision, anatomical limits or intended input.

The old frozen recipe permitted dorsal index flexion and was rejected by human
TV review. `forward-flex-v2` instead uses retained positive-flex bounds and a
signed displacement check against the rest palm plane. The previous pose fails
that check. Every exported frame records its signed displacements and optional
skin clearance. Static/rigid transforms still check marker-gap invariance;
articulated sequences retain changing gaps instead. They need human plausibility
and independent acceptance beyond these geometric guards.

The existing visible-motion runner consumes the new manifest without sensors or
OS output. Use an explicit offline config matching the intended2D/3D mode;
legacy configs default to2D. The view shows selected skin gap, estimated model
joint gap and emitted contact count separately. The corrected central41-frame
result and its limits are in the owning evolution report.

For the same articulated sequence from another controlled view, pass
`--closing-view yaw--1`, `yaw-1`, `pitch--1` or `pitch-1`. These reuse the existing
static transforms (yaw±35°, pitch±25°) for every frame, with unchanged articulation
and20Hz timing. Horizontal±12cm, vertical±7cm and depth±10cm cases are also
available under their static case names. This option requires `--closing-steps`;
it does not combine the articulation with a changing camera trajectory.
Hand-only single-reference exports now retain evaluated skin depth/image bounds
and full-skin framing, in addition to the joint markers. Full framing does not
establish visibility of every occluded fingertip or anatomical correctness.

The visible runner's optional `--label` identifies the actual finite experiment
on the TV and in its retained report/replay. New runs clear the preceding run's
final counters and disable its replay until new measurements complete.

```sh
python3 tools/synthetic/summarize_closure_views.py CENTER_RESULTS.json VIEW_RESULTS.json
```

This camera-free summary rejects mismatched frame order/times. It reports the
model's contacts during the renderer's open/maximum-closure holds, track IDs,
geometry/framing and estimated gaps. Holds are defined by articulation, not
predicted contact state; they are not human intent labels. Source report digests
allow the aggregate to be traced back to full private predictions and references.

### Independent paired pinches and overlap probes

`--pair-closing` with `--closing-steps 61 --closure 1 --hand-only --skin-check`
renders two mirrored reference hands with independent articulation phases. Use
ordinary separated hands or `--pair-layout crossing`:

```sh
uv run --offline --no-build --no-project --python 3.11 --with-requirements tools/synthetic/contact-requirements.lock \
  python tools/synthetic/render_hand_reference.py ASSET NEW_DIRECTORY \
  --hand-only --closure 1 --closing-steps 61 --skin-check --pair-closing --pair-layout separated
python3 tools/synthetic/summarize_pair_closure.py RESULTS.json --config EXACT_OFFLINE_CONFIG.json
python3 tools/synthetic/test_pair_closure.py
```

The first hand closes during normalized sequence phase0.15–0.25 and opens during
0.65–0.75; the second closes during0.35–0.45 and opens during0.80–0.90.
Each frame retains both reference IDs, marker projections, forward-flex checks,
optional distal skin distances and full-skin framing. The existing14cm camera-
depth offset must keep evaluated skin depth intervals disjoint. This establishes
separation of the two meshes, not per-finger visibility through screen overlap.

The paired summary checks the exact configuration digest and accepts only the
full uncalibrated workspace. Contacts use the native mirrored-X convention.
Counts during open/near holds are distinct from observable ownership. Palm-center
matching has explicit distance/margin guards; overlapping or uncertain associations
are unscored. The independent scorer controls include stable counts/IDs attached
to the wrong physical hands across overlap, so constant ID counts cannot pass an
ownership swap. Onsets retain independently measured skin gaps, including failures
that occur during transitions rather than a fully open/closed hold.

`sequence-check` now emits `ambiguity_reason` (`pair_overlap`, `track_identity`
or null). Its optional `--pair-separation WIDTHS` accepts0.8..2.0 palm widths;
the session default remains0.8 when the field is omitted. The offline option
overrides session `pair_separation_widths`; without it, replay uses the same
configured policy as acquisition. It can be combined with
`--rediscovery N`; repeated/unknown options and nonfinite/out-of-range values fail.
The visible runner exposes the same native-only probe and retains its value.
This controls rejection of a crowded pair, not pinch thresholds or identity
recovery. Raising it can cancel sooner; fewer contacts alone do not prove a better
controller. No probe changes the live policy. See the owning evolution report
for the early-contact failure, measured coverage and unchanged receiving state.

`--pair-half-spacing METERS` (0.03..0.16, default0.105) varies the paired closure's
separation/crossing span. `--pair-depth-offset METERS` (0.08..0.30, default0.14)
varies the rear hand's depth. Actual disjoint skin-depth checks remain mandatory;
an offset alone does not establish collision-free geometry. Frame labels retain
both values. The measured near-separated/deeper-crossing extension is in the
evolution report. Use immutable binary/config copies for comparison: the visible
runner records launch digests and rejects changes detected during inference.

### Geometry-only and matched temporal pinch comparisons

`render_hand_reference.py --measure-only --skin-check --closing-steps N` evaluates
the same articulation, framing and selected distal skin surfaces without rendering
images. It requires the optional contact environment and produces only
`labels.json`, with `images_rendered:false`, null image names and no image-sequence
manifest. Use the original asset, closure, gesture, view and spread arguments;
do not substitute a different geometry as truth for retained images.

`compare_pinch_interpretations.py RESULTS... --binary BINARY --settings SETTINGS
--output NEW_JSON` reuses retained raw predictions through the actual Rust
`mode-replay` controller, without inference or input devices. It compares raw
estimated meters with six exploratory world-gap/wrist-to-middle-MCP ratios,
three confirmation durations and20Hz/each10Hz phase. The ratio probe encodes
unitless values into the existing replay field; those are explicitly not
corrected physical meters. No runtime setting is adopted. It verifies complete
ordered predictions, reproduces baseline contact presence, checks Stop/EOF and
pins unchanged input/binary digests. Scores retain all contact frames, onset skin
distances, independent near/open holds and contacts at>=50mm selected separation.

Optional `--geometry LABELS` applies to one results file and requires matching
asset/recipe/target articulation/camera, source times and full world-marker
trajectory within1µm. This permits measuring the entire original trajectory
without repeating rendering/inference. A successful marker match is not full
anatomical or unseen-view validation. See the
[matched comparison](../../docs/research/native-visual-macros-2026-10.md#full-trajectory-interpretation-comparison)
for the rejected candidates and actual limits.

The visible motion runner also accepts `--geometry LABELS`, using the same
full-trajectory match and unchanged-file digest check. It retains original
input-image names and displays measured skin separation beside either backend's
raw gap. Tasks still has unknown contact/ownership semantics; projection never
turns those unknowns into zero contacts or stable identities.

### Additional pinch articulations

`--pinch-pose poses/curled-v1.json` or `poses/pip-dominant-v1.json` selects an
explicit seven-joint recipe for single-hand closing sequences. Paths here are
relative to `tools/synthetic/`. The version1 file pins the exact FBX SHA256 and
all seven named angles. The loader rejects unknown joints, nonfinite/boolean
angles, asset mismatch and values outside the retained forward recipe bounds.
Those bounds are research constraints, not certified anatomical joint limits.
Defaults remain `forward-flex-v2`; custom labels use `forward-flex-custom-v1`
and retain the loaded recipe's byte digest. The geometry matcher also checks it.

Before rendering a custom sequence, run `--measure-only --skin-check` with the
same inputs and inspect the full skin trajectory. Then remove `--measure-only`
and use a new output directory to render it. Positive forward flexion and distal
clearance do not certify whole-hand collision freedom or human intent. These
two source recipes are retained controls, not model predictions or human labels.
See the [diversity results](../../docs/research/native-visual-macros-2026-10.md#additional-articulations).

Two further recipes, `poses/mcp-distal-light-v1.json` and
`poses/mcp-distal-straight-v1.json`, retain MCP flexion1rad with PIP/DIP0.1/0.1
and0/0rad. The latter's distal segments are straight; its MCP is still flexed.
They use the same loader and unchanged renderer format. Independently sampled
minimum distal skin gaps are2.343/1.502mm. Full41-frame signed-motion/framing
checks and actual controller outcomes are retained in the
[onset-permission report](../../docs/research/pinch-onset-veto-2026-10.md).
`compare_pinch_interpretations.py --onset-screen-ratio 0.38` compares the optional
late onset veto against the raw controller on identical predictions at20Hz and
both10Hz phases. It keeps raw distances/confirmation/release rather than
encoding the pixel ratio as physical meters. This option is separate from the
earlier `--screen-ratio CLOSE OPEN` metric-replacement comparison.

### Optional WiLoR known-crop reconstruction

This research-only CPU comparison uses a separate Python3.10 environment because
the pinned provider/Chumpy/NumPy stack differs from the Blender environment.
Install `wilor-requirements.lock` with the PyTorch CPU extra index and
`--config-file tools/synthetic/wilor-uv.toml`; its extra build dependency is needed
for Chumpy's undeclared pip import. No system or Pi environment is changed.

Clone the provider repository at `wilor-assets.json`'s exact source commit and
obtain its three listed assets at the pinned public URLs/hashes, preserving the
separate WiLoR and MANO licenses. The probe validates source/assets and does not
download them or load a detector automatically. Model assets stay outside Git.

`wilor_probe.py SELECTION NEW_OUTPUT --repo SOURCE --assets ASSETS [--focal both]`
accepts a bounded version1 selection with1..512 cases. Each case declares `name`,
relative `image`, pixel `bbox:[left,top,right,bottom]`, independently known
`right:true`, `source_frame`, `skin_gap_m` and actual image `focal_px`.
Input paths must stay inside the selection directory. `crop_source` defaults to
`independent_reference_skin_bounds`; `retained_native_landmarks` explicitly marks
provided native-landmark boxes. Right-hand side remains independently specified.
This does not qualify detection/identity/confidence.
`--focal provider_default` or `known_renderer` selects one of the two declared
branches. Outputs retain transforms, source hashes, raw joints/vertices, measured
wall time and complete/error producer state. No gesture semantics are evaluated.
Use an actual resource ceiling for the heavy optional experiment; the initial
two-thread run reached5GiB. See the[model comparison](../../docs/research/mesh-reconstruction-2026-10.md)
for the completed sample/trajectory/native-box probes and acceptance limits.

`compare_mesh_metric.py NATIVE_RESULTS MESH_JSONL SELECTION --group NAME --images
ORIGINAL_IMAGES --binary BINARY --settings SETTINGS --output NEW_JSON` substitutes
only a complete, image-hash/source-clock-matched group's estimated gap. Retain
`reference_case` and `source_us` in each grouped selection case. Optional
`--geometry LABELS` requires the full independent geometry match. It reuses actual
Rust controller replay and scores20Hz/both10Hz phases, baseline reproduction and
Stop/EOF release. Native confidence, eligibility, ID and cursor remain unchanged;
they are not WiLoR confidence or ownership evidence. Partial producer appends are
read only through complete lines and incomplete groups are rejected. This is
conditional offline evidence, not a real-time runtime adapter or human intent.

`wilor_probe.py --profile` records CPU backbone/refinement/MANO call times;
`--execution dynamic_int8` converts only transformer-block Linear layers through
PyTorch2.5's x86 dynamic qint8 API. Default remains float32. All original trained
weights still load first and no quantized checkpoint is published. This is an
optional x86 CPU execution comparison, not Pi support or live adoption. The
[execution report](../../docs/research/wilor-cpu-execution-2026-10.md) records exact
static output reproduction, accuracy changes and still-unsuitable latency.

`project_mesh_probe.py SELECTION PRODUCER_RESULTS --evidence NEW_DIRECTORY
--backend LABEL --ssh-host HOST --ssh-identity KEY --display-dir OWNED_WORKBENCH`
can be started before the finite producer. It follows actual atomic progress and
complete JSONL appends, sending the current source image and measured gap. It
starts no inference and sends a small measured summary rather than raw mesh
vertices on completion. Explicit finite `--timeout` defaults to600s; timeout ends
only the observer, never proves the independently launched model stopped. Inspect
the actual model task before any retry. New evidence directories prevent mixing
observations. Reuse the owned receiving browser/SSH binding, not another tab.

`compare_pinch_interpretations.py ... --screen-ratio CLOSE OPEN` compares raw
meters with one fixed pixel-gap/pixel-palm interpretation instead of the default
world-ratio/dwell grid. It preserves actual native eligibility/confidence/ID and
open-before-press semantics. The encoded field is ratio×0.1, **not meters**;
selected source-clock20Hz/both10Hz phases are offline only. Keep developmental
parameter selection separate from fresh configuration/anatomy/human acceptance.

### Optional MobRecon SpiralConv comparison

`mobrecon-assets.json` pins the author source/checkpoint and locally prepared
topology. No weights or MANO-derived topology are redistributed. The lightweight
comparison is **not selected for live use**: its full307-frame replay loses
useful pinches despite removing the separated-macro contacts. See the same
[comparison report](../../docs/research/mesh-reconstruction-2026-10.md#mobrecon-spiralconv-cpu-comparison).

Prepare the topology once using the provider's unchanged `extract_spirals`:

```sh
uv run --no-project --python 3.9.25 \
  --with-requirements tools/synthetic/mobrecon-prepare-requirements.lock \
  python tools/synthetic/mobrecon_prepare.py --repo HANDMESH --output NEW_TOPOLOGY.npz
```

Python3.9 is needed only for the published Linux OpenMesh1.2.1 wheel; no source
compilation or training dependency stack is required. The pinned arrays include
four spiral levels, sparse upsampling entries and the author's21-point regressor.
The probe checks their exact retained digest. The existing Python3.10.20 WiLoR
CPU environment then runs:

```sh
python tools/synthetic/mobrecon_probe.py SELECTION NEW_OUTPUT \
  --repo HANDMESH --checkpoint mobrecon_densestack.pt --topology TOPOLOGY.npz
```

The version1 input selection/group association is the same as WiLoR's. Known
right-hand side remains independent. The declared box is expanded1.3 to an
axis-aligned square, with the author's integer/affine crop and RGB/0.5
normalization. The unchanged128px model loads strictly. Provider0.20 scaling
and MANO→MPII ordering produce the joint gap. Camera registration is omitted:
the provider fits a rigid translation, which cannot change this norm. Raw UV,
vertices, landmarks, image hashes and real forward/postprocessing time are
retained; timing excludes crop/decode, detection and camera registration.
Use `compare_mesh_metric.py --focal not_required_for_gap` for conditional controller
replay. It never adds mesh confidence, handedness inference or stable identity.

### Current-frame tracking diagnostics and palm matching

Camera-free `sequence-check` and `controller-sequence` add `tracking_diagnostics`
to each acquired frame: input tracks, actual palm-search occurrence, at most four
model attempts and two skipped proposals. Attempts include ROI, confidence,
predicted points and any duplicate match. These candidates are **not** accepted
`perception`/`hands` or contact/identity evidence. Missing images report null
diagnostics. Normal acquisition does not collect the candidate arrays.

An experimental session config can select `"palm_deduplication":"palm_landmarks"`.
Default `"roi"` preserves the old expanded-crop rule. The candidate matches five
corresponding palm points, then filters duplicate21-point reconstructions with a
handedness-score consistency check. It never identifies participants, preserves
input through ambiguity or changes positive-open rearm. Preflight/replay/future
recording metadata retain the selected policy. No receiving adoption follows
from setting it in an offline file. Keep the existing calibrated profile intact.

`render_hand_reference.py ... --pair-closing --closing-steps 61 --closure 1
--hand-only --skin-check --pair-chirality same` creates two same-side references;
default `opposite` retains the mirrored pair. The flag also supports paired rigid
motion and rejects unpaired use. Measure-only geometry precedes a new rendered
run. `matched_geometry` now checks both hands' IDs/world trajectories and requires
each measured hand's skin field; downstream scorers validate clearance values.
Unchanged first-hand markers cannot conceal
a changed second reference. The pair summary retains declared chirality or null
for old sources that did not report it.

See [tracking recovery](../../docs/research/tracking-recovery-2026-10.md) for the
retained first-version regression, corrected409-frame single-hand equality,
paired/cadence comparisons, new same-side controls and occlusion/resource limits.

### Stationary tracking and conditional Tasks metrics

[The crop/Tasks investigation](../../docs/research/roi-stability-and-tasks-2026-10.md)
verifies stationary decoded RGB despite changing PNG bytes, fixed-crop tensor
invariance, a rejected fresh-palm branch,655 Tasks inputs and fresh10Hz tracking
on three hard scenes. [Rejected experiment patches](experiments/README.md) are
reproduction artifacts; they do not add options to the selected/current runtime.

`compare_tasks_metric.py NATIVE_RESULTS TASKS_RESULTS --images IMAGES --binary
BINARY --settings SETTINGS --onset-ratio 0.38 --output NEW_REPORT` compares the
actual Rust controller with matched single-hand Tasks world-gap estimates and
optional Tasks-derived onset permission. Native identity, confidence, eligibility,
cursor and source time remain native; multiple provider hands are rejected,
missing hands cancel, source/geometry/image hashes are retained and baseline
contacts/Stop/EOF are checked. The helper does not run a model or inject input.
Its separate `counterfactual_unions` are Boolean combinations of independently
confirmed component decisions, not a fusion controller or delivered contact IDs.
The reported intervals are source-clock intervals; a second stride over actual
10Hz observations is5Hz, not another10Hz qualification.


### Implemented pinch fusion and native dual crop

The [fusion qualification](../../docs/research/native-pinch-fusion-2026-10.md)
distinguishes actual Rust meaning from the older Boolean-only comparison.
`replay_pinch_fusion.py NATIVE_RESULTS TASKS_RESULTS --binary BINARY --settings
SETTINGS --onset-ratio 0.38 --output NEW_REPORT` attaches matched same-frame
secondary metrics, retains primary ownership/cursor/quality, and checks actual
contact IDs, independent confirmation, component union, legacy replay and Stop/EOF.
Inherited secondary quality is not Tasks confidence. This conditional single-hand
helper does not infer, capture, inject input or qualify live provider matching.

An experimental native session can select `"dual_crop":true` with
`"pinch_3d":true`; default is false. Existing `controller-sequence` and
`sequence-check` run both crops through one shared ONNX Session/weights on the
same image. Primary tracking/cursor stay authoritative, secondary evidence has
its own confidence, eligibility, world gap and optional onset cue. Diagnostics
keep secondary attempts/raw points separate from hand/contact identities.
A second crop is not a second person. Source recording metadata retains policy;
no receiving runtime/profile adoption follows. Actual10Hz adverse-view failure,
held-release conflicts, paired crossing and Pi resources remain open.


### Exact tensors and a converted secondary model

[The fixed-tensor investigation](../../docs/research/fixed-tensor-mixed-model-2026-10.md)
qualifies a default-off mixed-model source and preserves its ownership/resource
limits. `wigglecontrol crop-check IMAGE SESSION NEW_DIRECTORY [ROI.json]` exports
`input.f32` (little-endian RGBfloat32,1×224×224×3) and fixed native inference.
An explicit ROI is `[center_x_px,center_y_px,side_px,angle_rad]`; otherwise the
native cold detector supplies it. No camera or input device opens.

`compare_crop_models.py LANDMARK.tflite CROP_DIRECTORY... --expected-sha256 SHA
--output NEW_REPORT` executes pinned LiteRT2.3.0 on those exact float values.
Use `litert-requirements.lock` for this optional offline comparison, not the
receiving runtime. `wigglecontrol-ort-probe MODEL.onnx 224 RUNS INPUT.f32
NEW_REPORT.json`, with the existing `ORT_DYLIB_PATH`, records all actual Rust ORT
outputs on the same values. Omitting the final two arguments preserves its
previous bounded zero-tensor timing mode. Existing reports are never overwritten.

The [asset manifest](tasks-onnx-model.json) and
`tflite-conversion-requirements.lock` pin the observed conversion. Extract its
qualified `hand_landmarks_detector.tflite` member from the pinned Tasks bundle;
convert once with `python -m tf2onnx.convert --tflite MODEL.tflite --output
NEW_MODEL.onnx --opset 17` in that environment.
The owning report supplies the full tested command. Verify numerical equivalence
before selecting a new derivative; a file digest identifies an artifact, not
physical accuracy. Model binaries stay outside Git with upstream provenance.

An experimental3D/dual session can select `secondary_model:{path,min_presence,
source_id}` and explicit `gesture_min_confidence`; controller settings must
match. The new source executes its own initial estimate and recursive crops,
retaining raw own confidence/eligibility and the original primary owner/cursor.
It reuses ONNX Runtime, without a receiving Python/Tasks stack. Optional secondary
metric feedback is now visible in the owned offline workbench. This is not a
selected live runtime, general ownership fix or accepted human control.

### Receiving browser CPU/GPU comparison

`browser_hand_sequence.mjs` exports `runBrowserHandSequence(baseUrl, manifest)`
for an explicitly owned experiment page. The optional pinned Tasks Vision1.1.1
package/model and synthetic inputs are served read-only on receiving loopback;
the helper opens no camera and emits no input. The manifest contains
`package_version`, `model_sha256`, and `scenes:[{name,reference,frames}]`; each
frame has `image`, `source_us` and `sha256`. The server exposes `telemetry` with
`temperature_c` and serves `package/vision_bundle.mjs`, `package/wasm/`,
`hand_landmarker.task` and `SCENE/IMAGE`. Verify the npm archive and retained
asset hashes before serving. Each scene is bounded9–121 frames/60seconds, with
an80°C temperature cutoff; use a separate finite server lifetime as well.

Each delegate/scene uses a fresh VIDEO model, records fetch/hash/decode and
inference/readback costs separately, and closes it on completion/failure. Returned
world/pixel points and handedness are observations; array indices are never
stable hand identities. Optional workbench feedback shows actual producer
progress and a retained last image. Save the returned report independently of
the display. The [receiving measurements](../../docs/research/palm-correspondence-gpu-2026-10.md)
establish a hardware-WebGL speed improvement and a narrow conditional metric
replay, not an accelerated system-input backend or physical qualification.

`execution:"worker"` selects the optional classic `task_hand_worker.js` and
OffscreenCanvas. Serve the experiment page, helper, worker and assets from the
**same** loopback HTTP origin; `file://` cross-origin/module-worker attempts failed
on the actual receiver. The unchanged pinned SDK uses classic Wasm script/global
factory initialization. One ImageBitmap request is in flight, bounded30seconds;
models close with an acknowledgment before worker termination. Recorded
`inference_ms` then includes submission/transfer/readback; `worker_inference_ms`
measures the inner model call. Animation callback counts during inference observe
main-thread scheduling, not full display performance. A supplied UUID `run_id`
lets the existing projector initialize `run-state.js` with that ID/seq0 before
direct updates, preventing an old file-poll source from replacing current progress.
Persist actual terminal state afterward; do not manufacture animation for display.

`replay_browser_provider.py REPORT --images SCENE_PARENT --binary BINARY --session
SESSION --settings SETTINGS --output NEW_DIRECTORY` verifies pinned provider,
frame/image digests and source association, then executes actual `provider-replay`
without native tracking/quality/cursor inheritance. Config selects single-estimate
pinch and gesture confidence0.5 matching the configured SDK presence floor;
image-depth/visual macros remain unavailable. No model initializes in Rust here.
The optional constant `--delay-us` is an artificial age counterfactual, not measured
camera/IPC latency. Reports retain source/configuration digests, raw Rust receipts,
geometry and Stop/EOF release. See the
[worker/provider report](../../docs/research/provider-worker-2026-10.md) for actual
receiving measurements, startup failures and live integration still needed.
