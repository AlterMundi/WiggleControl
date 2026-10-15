"""Independent distal skin-triangle distance via pinned FCL; no perception input.

This measures explicitly selected rig skin patches, not human intent, joint limits,
whole-hand self-collision, soft-tissue compression or certified anatomical tips.
"""
from importlib.metadata import version

import fcl
import numpy as np


def collision_object(vertices, triangles):
    vertices = np.asarray(vertices, dtype=np.float64)
    triangles = np.asarray(triangles, dtype=np.int32)
    if (vertices.ndim != 2 or vertices.shape[1:] != (3,) or not len(vertices)
            or not np.isfinite(vertices).all() or np.abs(vertices).max() > 1e6
            or triangles.ndim != 2 or triangles.shape[1:] != (3,)
            or not len(triangles) or triangles.min() < 0 or triangles.max() >= len(vertices)):
        raise ValueError('Expected finite world vertices and nonempty indexed triangles')
    points = vertices[triangles]
    if np.any(np.linalg.norm(np.cross(points[:, 1]-points[:, 0], points[:, 2]-points[:, 0]), axis=1) <= 1e-14):
        raise ValueError('Degenerate skin triangle; clearance is unqualified')
    model = fcl.BVHModel()
    model.beginModel(len(triangles), len(vertices))
    model.addSubModel(vertices, triangles)
    model.endModel()
    return fcl.CollisionObject(model)


def surface_distance(first, second):
    collision = fcl.CollisionResult()
    fcl.collide(first, second, fcl.CollisionRequest(), collision)
    distance = fcl.DistanceResult()
    gap = fcl.distance(first, second, fcl.DistanceRequest(enable_nearest_points=True), distance)
    if not collision.is_collision and (not np.isfinite(gap) or gap < 0):
        raise ValueError('FCL did not establish a finite separated-surface distance')
    # Mesh collision uses a sentinel negative distance; it is not penetration depth.
    return dict(surface_gap_m=0.0 if collision.is_collision else float(gap),
                surface_intersection=bool(collision.is_collision),
                nearest_world_m=None if collision.is_collision else
                    [list(map(float, point)) for point in distance.nearest_points])


def distal_skin_clearance(mesh, graph):
    evaluated = mesh.evaluated_get(graph)
    data = evaluated.data
    data.calc_loop_triangles()
    objects, patches = [], {}
    for finger in ('thumb', 'index'):
        names = (f'r_{finger}3', f'r_{finger}_null')
        groups = {g.index for g in mesh.vertex_groups if g.name in names}
        if len(groups) != 2:
            raise ValueError(f'Missing distal {finger} skinning groups')
        selected = {v.index for v in data.vertices
                    if sum(g.weight for g in v.groups if g.group in groups) >= 0.5}
        faces = [tuple(t.vertices) for t in data.loop_triangles
                 if all(i in selected for i in t.vertices)]
        indices = sorted({i for face in faces for i in face})
        lookup = {old: new for new, old in enumerate(indices)}
        vertices = [list(evaluated.matrix_world @ data.vertices[i].co) for i in indices]
        triangles = [[lookup[i] for i in face] for face in faces]
        objects.append(collision_object(vertices, triangles))
        patches[finger] = dict(groups=names, minimum_weight_sum=0.5,
                               vertices=len(vertices), triangles=len(triangles))
    return dict(**surface_distance(*objects), patches=patches, python_fcl=version('python-fcl'),
                definition='Minimum distance between selected distal thumb/index skin triangles; intersection is not signed depth')
