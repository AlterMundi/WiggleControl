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



def forward_flexion(rest, posed, normal, unit_scale):
    """Reject the dorsal solver branch; this alone does not certify anatomy."""
    displacement = {name: (posed[name]-point).dot(normal)*unit_scale
                    for name, point in rest.items()}
    if any(not math.isfinite(d) or d < -1e-6 for d in displacement.values()):
        raise ValueError('Index flexion moved behind the rest palm plane; invalid reference pose')
    return displacement


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
    parser.add_argument('--closing-steps', type=int, default=0,
                        help='9..121-frame opening/closing sequence toward --closure; requires --hand-only')
    parser.add_argument('--closing-view', default='reference', choices=[
        'reference', 'horizontal--1', 'horizontal-1', 'vertical--1', 'vertical-1',
        'depth--1', 'depth-1', 'yaw--1', 'yaw-1', 'pitch--1', 'pitch-1'],
        help='Reuse a controlled rigid view for the entire articulated sequence')
    parser.add_argument('--skin-check', action='store_true',
                        help='Measure selected distal skin patches using optional pinned python-fcl')
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
    if args.closing_steps and (not 9 <= args.closing_steps <= 121 or not args.hand_only
                               or args.motion_steps or args.yaw_steps or args.pair_motion or args.closure <= 0):
        parser.error('--closing-steps requires9..121, --hand-only, positive --closure and no other sequence')
    if args.skin_check and args.pair_motion:
        parser.error('--skin-check currently supports a single reference hand')
    if args.closing_view != 'reference' and not args.closing_steps:
        parser.error('--closing-view requires --closing-steps')
    if args.skin_check:
        from skin_clearance import distal_skin_clearance
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
    # Forward-flex recipe v2. The previous unconstrained endpoint solver chose
    # dorsal index bending, rejected by Nicolás's actual TV review. Positive
    # flexion and a signed rest-plane check reject that branch. Distal skin
    # clearance is measured separately; these bounds do not certify anatomy.
    parameters = {
        'r_index1': 0.5780913844963557, 'r_index2': 0.9080790178234996,
        'r_index3': 0.6839674325177723, 'r_thumb0': 0.4828462042455118,
        'r_thumb1': -0.06196422713524768, 'r_thumb2': 0.21760893089706515,
        'r_thumb3': 0.0,
    }
    limits = {'r_index1': [0, 1.0], 'r_index2': [0, 1.2], 'r_index3': [0, 1.0],
              'r_thumb0': [-1.0, 1.0], 'r_thumb1': [-0.5, 0.5],
              'r_thumb2': [0, 1.0], 'r_thumb3': [0, 1.0]}
    if any(not limits[n][0] <= value <= limits[n][1] for n, value in parameters.items()):
        raise ValueError('Forward pose exceeds the retained recipe bounds')
    index_rest = {n: rig.pose.bones[n].head.copy()
                  for n in ('r_index2', 'r_index3', 'r_index_null')}
    normal_local = (rig.matrix_world.to_3x3().inverted() @ normal).normalized()
    unit_scale = rig.matrix_world.to_scale().x
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
    forward_flexion(index_rest, {n: rig.pose.bones[n].head for n in index_rest}, normal_local, unit_scale)
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
    if args.closing_steps:
        _, translation, turn = next(case for case in cases if case[0] == args.closing_view)
        cases = [(f'motion-closing-{i:03d}', translation, turn)
                 for i in range(args.closing_steps)]
    labels = []
    for index, (name, translation, turn) in enumerate(cases):
        closure = args.closure
        if args.closing_steps:
            phase = index / (args.closing_steps - 1)
            # Positive open/near holds surround the transition; geometry, not
            # model output, defines the independently retained articulation.
            closure *= max(0.0, min(1.0, (phase - 0.2) / 0.2, (0.8 - phase) / 0.2))
            for bone, angle in parameters.items():
                rig.pose.bones[bone].rotation_quaternion = Quaternion(axes[bone], angle * closure)

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
        if args.motion_steps or args.closing_steps:
            labels[-1]['source_us'] = 1 + index * 50_000
        if args.closing_steps:
            labels[-1]['closure_interpolation'] = closure
            labels[-1]['view_case'] = args.closing_view
        labels[-1]['index_forward_displacement_m'] = forward_flexion(
            index_rest, {n: rig.pose.bones[n].head for n in index_rest}, normal_local, unit_scale)
        if args.skin_check:
            labels[-1]['distal_skin'] = distal_skin_clearance(mesh, bpy.context.evaluated_depsgraph_get())
        if args.hand_only and not args.pair_motion:
            evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
            skin_points = [evaluated.matrix_world @ v.co for v in evaluated.data.vertices]
            inverse_camera = camera.matrix_world.inverted()
            depths = [-(inverse_camera @ p).z for p in skin_points]
            skin_projected = [world_to_camera_view(scene, camera, p) for p in skin_points]
            labels[-1]['skin_camera_depth_range_m'] = [min(depths), max(depths)]
            labels[-1]['skin_image_bounds_px'] = [
                min(p.x for p in skin_projected)*640, (1-max(p.y for p in skin_projected))*480,
                max(p.x for p in skin_projected)*640, (1-min(p.y for p in skin_projected))*480]
            labels[-1]['skin_in_frame'] = all(0 <= p.x <= 1 and 0 <= p.y <= 1 and p.z > 0
                                              for p in skin_projected)
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
        if args.yaw_steps or args.motion_steps or args.closing_steps:
            print('WIGGLECONTROL_FRAME ' + json.dumps(labels[-1], separators=(',', ':')), flush=True)
    gaps = [entry['rig_tip_gap_m'] for entry in labels]
    if not args.closing_steps and max(gaps)-min(gaps) > 1e-6:
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
              'closure_interpolation': None if args.closing_steps else args.closure,
              'articulation_sequence': bool(args.closing_steps),
              'closing_view': args.closing_view if args.closing_steps else None,
              'skin_clearance_measured': args.skin_check,
              'render_geometry': 'right-hand weights >=0.5 mask' if args.hand_only else 'whole body',
              'reference_hand_count': 2 if args.pair_motion else 1,
              'applied_joint_angles_rad': {n: v * closure for n, v in parameters.items()},
              'target_joint_angles_rad': {n: v * args.closure for n, v in parameters.items()},
              'pose_recipe': 'forward-flex-v2', 'joint_recipe_bounds_rad': limits,
              'pose_qualification': 'Forward branch checked; selected distal skin clearance is separate from joint-limit, whole-hand collision and human acceptance',
              'camera_opened': False, 'rig_marker_gap_invariant_tolerance_m': None if args.closing_steps else 1e-6}
    (args.output/'labels.json').write_text(json.dumps(report, indent=2)+'\n')
    if args.motion_steps or args.closing_steps:
        (args.output/'sequence.json').write_text(json.dumps({'version': 1, 'frames': [
            {'source_us': f['source_us'], 'image': f['image']} for f in labels]}, indent=2)+'\n')
    print('Generated and geometrically checked', len(labels), 'controlled frames')


if __name__ == '__main__':
    main()
