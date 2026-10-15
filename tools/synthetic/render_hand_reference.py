#!/usr/bin/env python3
"""Offline MHR geometry controls via Blender's Python API; no sensor/input access.

uv run --no-project --python 3.11 --with-requirements tools/synthetic/requirements.lock python \
  tools/synthetic/render_hand_reference.py assets/lod1.fbx NEW_DIRECTORY

The rig joint markers are reproducible geometric labels, not certified skin
contact points or measured human anatomy. Keep those distinctions in evaluations.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

import bpy
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Matrix, Quaternion, Vector


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('fbx', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--closure', type=float, default=0.0,
                        help='0..1 interpolation toward a research-only near-finger pose')
    args = parser.parse_args()
    if not 0 <= args.closure <= 1:
        parser.error('--closure must be in0..1')
    args.output.mkdir(mode=0o700, parents=False, exist_ok=False)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(args.fbx.resolve()), use_anim=False)
    scene = bpy.context.scene
    rig = next(o for o in scene.objects if o.type == 'ARMATURE')
    mesh = next(o for o in scene.objects if o.type == 'MESH')
    for obj in list(scene.objects):
        if obj.type == 'EMPTY':
            bpy.data.objects.remove(obj, do_unlink=True)
    bpy.context.view_layer.update()
    names = ['r_wrist', 'r_thumb1', 'r_thumb2', 'r_thumb3', 'r_thumb_null']
    names += [f'r_{finger}{joint}' for finger in ['index', 'middle', 'ring', 'pinky']
              for joint in ['1', '2', '3', '_null']]
    if any(name not in rig.pose.bones for name in names):
        raise ValueError('The asset does not have the expected MHR right-hand rig')
    point = lambda name: rig.matrix_world @ rig.pose.bones[name].head
    wrist, middle = point('r_wrist'), point('r_middle1')
    up = (middle - wrist).normalized()
    side = (point('r_index1') - point('r_pinky1')).normalized()
    normal = side.cross(up).normalized()
    center = wrist + (middle - wrist) * 1.05
    # A frozen geometric-solver candidate, not a biomechanically qualified pose
    # or a physical skin-contact label. Retain exact applied angles for review.
    parameters = {
        'r_index1': -1.4403919155978504, 'r_index2': -1.7, 'r_index3': -1.7,
        'r_thumb0': 0.39951653963086003, 'r_thumb1': -0.1086158339027759,
        'r_thumb2': -1.0, 'r_thumb3': -1.0,
    }
    children = {'r_index1': 'r_index2', 'r_index2': 'r_index3',
                'r_index3': 'r_index_null', 'r_thumb2': 'r_thumb3',
                'r_thumb3': 'r_thumb_null'}
    axes = {}
    for name in parameters:
        axis = ((point(children[name])-point(name)).cross(normal).normalized()
                if name in children else normal)
        axes[name] = (rig.data.bones[name].matrix_local.to_3x3().inverted() @
                      (rig.matrix_world.to_3x3().inverted() @ axis)).normalized()
    for name, angle in parameters.items():
        bone = rig.pose.bones[name]
        bone.rotation_mode = 'QUATERNION'
        bone.rotation_quaternion = Quaternion(axes[name], angle * args.closure)
    bpy.context.view_layer.update()
    camera_data = bpy.data.cameras.new('CalibrationCamera')
    camera = bpy.data.objects.new('CalibrationCamera', camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    camera.location = center + normal * 0.65
    camera.rotation_euler = (center - camera.location).to_track_quat('-Z', 'Y').to_euler()
    rotation = camera.rotation_euler.to_matrix()
    right = rotation @ Vector((1, 0, 0))
    camera_up = rotation @ Vector((0, 1, 0))
    roll = math.atan2(up.dot(right), up.dot(camera_up))
    camera.rotation_euler.rotate_axis('Z', -roll)
    camera_data.lens = 42
    camera_data.sensor_width = 36
    camera_data.clip_start = 0.01
    material = bpy.data.materials.new('SyntheticSkin')
    material.use_nodes = True
    bsdf = material.node_tree.nodes.get('Principled BSDF')
    bsdf.inputs['Base Color'].default_value = (0.52, 0.26, 0.15, 1)
    bsdf.inputs['Roughness'].default_value = 0.53
    mesh.data.materials.clear()
    mesh.data.materials.append(material)
    for poly in mesh.data.polygons:
        poly.use_smooth = True
    world = bpy.data.worlds.new('CalibrationWorld')
    world.use_nodes = True
    world.node_tree.nodes['Background'].inputs[0].default_value = (0.12, 0.13, 0.16, 1)
    world.node_tree.nodes['Background'].inputs[1].default_value = 0.5
    scene.world = world
    light_data = bpy.data.lights.new('Key', 'AREA')
    light_data.energy = 5
    light_data.shape = 'DISK'
    light_data.size = 0.5
    light = bpy.data.objects.new('Key', light_data)
    scene.collection.objects.link(light)
    light.location = center + normal * 0.4 + up * 0.3 + side * 0.2
    light.rotation_euler = (center - light.location).to_track_quat('-Z', 'Y').to_euler()
    scene.render.engine = 'CYCLES'
    scene.cycles.device = 'CPU'
    scene.cycles.samples = 12
    scene.cycles.seed = 0
    scene.cycles.use_denoising = False
    scene.render.threads_mode = 'FIXED'
    scene.render.threads = 2
    scene.render.resolution_x, scene.render.resolution_y = 640, 480
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    scene.view_settings.view_transform = 'Standard'
    scene.unit_settings.system = 'METRIC'
    scene.unit_settings.scale_length = 1
    bpy.context.view_layer.update()
    projection = camera.calc_matrix_camera(bpy.context.evaluated_depsgraph_get(), x=640, y=480)
    camera_rotation = camera.rotation_euler.to_matrix()
    right = camera_rotation @ Vector((1, 0, 0))
    vertical = camera_rotation @ Vector((0, 1, 0))
    forward = camera_rotation @ Vector((0, 0, -1))
    base = rig.matrix_world.copy()
    cases = [('reference', Vector(), Matrix.Identity(4))]
    for sign in [-1, 1]:
        cases += [(f'horizontal-{sign}', right * (sign * 0.12), Matrix.Identity(4)),
                  (f'vertical-{sign}', vertical * (sign * 0.07), Matrix.Identity(4)),
                  (f'depth-{sign}', forward * (sign * 0.10), Matrix.Identity(4)),
                  (f'yaw-{sign}', Vector(), Matrix.Rotation(math.radians(sign * 35), 4, vertical)),
                  (f'pitch-{sign}', Vector(), Matrix.Rotation(math.radians(sign * 25), 4, right))]
    labels = []
    for index, (name, translation, turn) in enumerate(cases):
        rig.matrix_world = Matrix.Translation(center + translation) @ turn @ Matrix.Translation(-center) @ base
        bpy.context.view_layer.update()
        points = [point(n) for n in names]
        camera_points = [camera.matrix_world.inverted() @ p for p in points]
        projected = [world_to_camera_view(scene, camera, p) for p in points]
        file = f'{index:03d}-{name}.png'
        labels.append({'frame': index, 'case': name, 'image': file,
                       'world_m': [list(p) for p in points],
                       'camera_m': [list(p) for p in camera_points],
                       'image_px': [[p.x * 640, (1-p.y) * 480] for p in projected],
                       'markers_in_frame': all(0 <= p.x <= 1 and 0 <= p.y <= 1 and p.z > 0 for p in projected),
                       'rig_tip_gap_m': (points[4]-points[8]).length,
                       'rig_world_matrix': list(map(list, rig.matrix_world))})
        scene.render.filepath = str((args.output/file).resolve())
        bpy.ops.render.render(write_still=True)
    gaps = [entry['rig_tip_gap_m'] for entry in labels]
    if max(gaps)-min(gaps) > 1e-6:
        raise RuntimeError('Rigid camera/pose control changed the reference marker distance')
    rig.matrix_world = base
    bpy.context.view_layer.update()
    scene.render.filepath = str((args.output/'source-preview.png').resolve())
    bpy.ops.wm.save_as_mainfile(filepath=str((args.output/'source.blend').resolve()))
    report = {'schema': 'wigglecontrol.synthetic-rig/v1', 'blender': bpy.app.version_string,
              'asset_sha256': hashlib.sha256(args.fbx.read_bytes()).hexdigest(),
              'units': 'meters, imported FBX world coordinates', 'landmark_names': names,
              'landmark_kind': 'rig joint markers; skin contact/anatomical mapping not qualified',
              'camera_world_matrix': list(map(list, camera.matrix_world)),
              'camera_projection_matrix': list(map(list, projection)),
              'image_size': [640, 480], 'render_seed': 0, 'frames': labels,
              'closure_interpolation': args.closure,
              'applied_joint_angles_rad': {n: v * args.closure for n, v in parameters.items()},
              'pose_qualification': 'geometric candidate, anatomical limits/skin contact not validated',
              'camera_opened': False, 'rig_marker_gap_invariant_tolerance_m': 1e-6}
    (args.output/'labels.json').write_text(json.dumps(report, indent=2)+'\n')
    print('Generated and geometrically checked', len(labels), 'controlled frames')


if __name__ == '__main__':
    main()
