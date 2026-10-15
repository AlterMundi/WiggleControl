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
