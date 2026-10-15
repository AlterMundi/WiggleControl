# Offline articulated hand references

This optional research tool renders actual3D geometry using Blender's Python API.
Python is used for the renderer's supported scripting interface; the input device,
perception and gesture runtime stay Rust. It is not installed on the integrator,
opens no sensors and does not change the production models or camera calibration.

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
The default keeps an open pose. Optional `--closure VALUE` in the range0..1 interpolates toward a frozen geometric-solver candidate;
`--closure 1` was also rendered at all eleven views. Applied joint angles and
actual marker coordinates are retained. Joint limits, skin penetration and
physical contact are unqualified, so do not turn this candidate into a pinch
label. This tool does not qualify a live correction or change45/65 mm thresholds.

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
