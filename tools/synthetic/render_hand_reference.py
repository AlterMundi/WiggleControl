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
    parser.add_argument('--yaw-steps', type=int, default=0,
                        help='Optional 3..31 equally spaced yaw views from -35 to +35 degrees')
    parser.add_argument('--hand-only', action='store_true',
                        help='Mask the rest of the body using right-hand skinning weights')
    parser.add_argument('--motion-steps', type=int, default=0,
                        help='Optional 9..121-frame rigid hand trajectory, 20 Hz source time')
    parser.add_argument('--pair-motion', action='store_true',
                        help='Two independently transformed mirrored open hands approach and cross')
    args = parser.parse_args()
    if not 0 <= args.closure <= 1:
        parser.error('--closure must be in0..1')
    if args.yaw_steps and not 3 <= args.yaw_steps <= 31:
        parser.error('--yaw-steps must be0 or3..31')
    if args.motion_steps and (not 9 <= args.motion_steps <= 121 or args.yaw_steps or not args.hand_only):
        parser.error('--motion-steps requires9..121, --hand-only and no --yaw-steps')
    if args.pair_motion and (not args.motion_steps or args.closure != 0):
        parser.error('--pair-motion requires --motion-steps and open --closure0')
    args.output.mkdir(mode=0o700, parents=False, exist_ok=False)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(args.fbx.resolve()), use_anim=False)
    scene = bpy.context.scene
    rig = next(o for o in scene.objects if o.type == 'ARMATURE')
    mesh = next(o for o in scene.objects if o.type == 'MESH')
    for obj in list(scene.objects):
        if obj.type == 'EMPTY':
            bpy.data.objects.remove(obj, do_unlink=True)
    if args.hand_only:
        # The whole body can otherwise rotate between the camera and hand.
        selected = {g.index for g in mesh.vertex_groups if
                    g.name == 'r_wrist' or any(g.name.startswith('r_' + finger)
                    for finger in ('thumb', 'index', 'middle', 'ring', 'pinky'))}
        if not selected:
            raise ValueError('Right-hand skinning groups unavailable; cannot isolate the hand')
        visible = mesh.vertex_groups.new(name='CalibrationVisibleHand')
        vertices = [v.index for v in mesh.data.vertices
                    if sum(g.weight for g in v.groups if g.group in selected) >= 0.5]
        if not vertices:
            raise ValueError('No hand vertices selected')
        visible.add(vertices, 1, 'REPLACE')
        mask = mesh.modifiers.new(name='CalibrationHandOnly', type='MASK')
        mask.vertex_group = visible.name
        mask.threshold = 0.5
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
    second_rig = None
    if args.pair_motion:
        second_rig = rig.copy()
        second_rig.data = rig.data.copy()
        second_rig.name = 'ReferenceHandB'
        scene.collection.objects.link(second_rig)
        second_mesh = mesh.copy()
        second_mesh.data = mesh.data.copy()
        second_mesh.name = 'ReferenceHandBMesh'
        scene.collection.objects.link(second_mesh)
        if mesh.parent == rig:
            second_mesh.parent = second_rig
        for modifier in second_mesh.modifiers:
            if modifier.type == 'ARMATURE':
                modifier.object = second_rig
        # Reflect in the camera's horizontal plane to create the other chirality.
        reflection = Matrix.Identity(4)
        for row in range(3):
            for col in range(3):
                reflection[row][col] -= 2 * right[row] * right[col]
    cases = [('reference', Vector(), Matrix.Identity(4))]
    for sign in [-1, 1]:
        cases += [(f'horizontal-{sign}', right * (sign * 0.12), Matrix.Identity(4)),
                  (f'vertical-{sign}', vertical * (sign * 0.07), Matrix.Identity(4)),
                  (f'depth-{sign}', forward * (sign * 0.10), Matrix.Identity(4)),
                  (f'yaw-{sign}', Vector(), Matrix.Rotation(math.radians(sign * 35), 4, vertical)),
                  (f'pitch-{sign}', Vector(), Matrix.Rotation(math.radians(sign * 25), 4, right))]
    if args.yaw_steps:
        cases = [(f'yaw-sweep-{i:02d}', Vector(),
                  Matrix.Rotation(math.radians(-35 + i * 70 / (args.yaw_steps - 1)), 4, vertical))
                 for i in range(args.yaw_steps)]
    if args.motion_steps:
        cases = [(f'motion-{i:03d}', right * (0.10 * math.sin(i * 2 * math.pi / (args.motion_steps - 1))),
                  Matrix.Rotation(math.radians(20 * math.sin(i * 2 * math.pi / (args.motion_steps - 1))), 4, vertical))
                 for i in range(args.motion_steps)]
    labels = []
    for index, (name, translation, turn) in enumerate(cases):
        if args.pair_motion:
            phase = index / (args.motion_steps - 1)
            # Distinct references cross in image space; no detector output defines identity.
            offset = -0.10 + phase * 0.20
            translation = right * offset
            turn = Matrix.Identity(4)
            second_rig.matrix_world = Matrix.Translation(center - right * offset + forward * 0.14) @ reflection @ Matrix.Translation(-center) @ base
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
        if args.motion_steps:
            labels[-1]['source_us'] = 1 + index * 50_000
        if args.pair_motion:
            inverse_camera = camera.matrix_world.inverted()
            graph = bpy.context.evaluated_depsgraph_get()
            depth_ranges = []
            for obj in (mesh, second_mesh):
                evaluated = obj.evaluated_get(graph)
                depths = [-(inverse_camera @ evaluated.matrix_world @ v.co).z
                          for v in evaluated.data.vertices]
                depth_ranges.append([min(depths), max(depths)])
            if depth_ranges[0][1] >= depth_ranges[1][0]:
                raise RuntimeError('Paired visible skin depth ranges overlap; collision-free reference not established')
            # Same marker definition in each independent rig's local coordinate frame.
            second_points = [second_rig.matrix_world @ second_rig.pose.bones[n].head for n in names]
            second_projected = [world_to_camera_view(scene, camera, p) for p in second_points]
            labels[-1]['reference_hands'] = [
                {'id': 'reference-a', 'world_m': [list(p) for p in points],
                 'image_px': labels[-1]['image_px']},
                {'id': 'reference-b', 'world_m': [list(p) for p in second_points],
                 'image_px': [[p.x * 640, (1-p.y) * 480] for p in second_projected]}]
            labels[-1]['skin_camera_depth_ranges_m'] = depth_ranges
            labels[-1]['separated_skin_depth_ranges'] = True
        if args.yaw_steps:
            labels[-1]['yaw_degrees'] = -35 + index * 70 / (args.yaw_steps - 1)
            print('WIGGLECONTROL_STAGE ' + json.dumps({'phase': 'render',
                  'frame': index, 'yaw_degrees': labels[-1]['yaw_degrees'],
                  'reference_mm': labels[-1]['rig_tip_gap_m'] * 1000,
                  'depth_cm': [min(-p.z for p in camera_points) * 100,
                               max(-p.z for p in camera_points) * 100]}), flush=True)
        scene.render.filepath = str((args.output/file).resolve())
        bpy.ops.render.render(write_still=True)
        if args.yaw_steps or args.motion_steps:
            print('WIGGLECONTROL_FRAME ' + json.dumps(labels[-1], separators=(',', ':')), flush=True)
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
              'render_geometry': 'right-hand weights >=0.5 mask' if args.hand_only else 'whole body',
              'reference_hand_count': 2 if args.pair_motion else 1,
              'applied_joint_angles_rad': {n: v * args.closure for n, v in parameters.items()},
              'pose_qualification': 'geometric candidate, anatomical limits/skin contact not validated',
              'camera_opened': False, 'rig_marker_gap_invariant_tolerance_m': 1e-6}
    (args.output/'labels.json').write_text(json.dumps(report, indent=2)+'\n')
    if args.motion_steps:
        (args.output/'sequence.json').write_text(json.dumps({'version': 1, 'frames': [
            {'source_us': f['source_us'], 'image': f['image']} for f in labels]}, indent=2)+'\n')
    print('Generated and geometrically checked', len(labels), 'controlled frames')


if __name__ == '__main__':
    main()
