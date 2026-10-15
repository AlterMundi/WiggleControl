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
