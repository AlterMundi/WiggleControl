"""Analytic geometry checks for the optional independent contact reference."""
import math
import unittest
import numpy as np
from skin_clearance import collision_object, surface_distance


class SurfaceDistance(unittest.TestCase):
    def test_known_separation_edge_distance_and_rigid_transform(self):
        square = np.array([[0, 0, 0], [0.01, 0, 0], [0.01, 0.01, 0], [0, 0.01, 0]])
        triangles = [[0, 1, 2], [0, 2, 3]]
        for offset, expected in [([0, 0, 0.02], 0.02), ([0.03, 0, 0.02], math.sqrt(0.02**2*2))]:
            second = square+offset
            rotation = np.array([[0, -1, 0], [1, 0, 0], [0, 0, 1]])
            for transformed in (False, True):
                a, b = square, second
                if transformed:
                    a, b = a@rotation.T+[1, 2, 3], b@rotation.T+[1, 2, 3]
                result = surface_distance(collision_object(a, triangles), collision_object(b, triangles))
                self.assertFalse(result['surface_intersection'])
                self.assertAlmostEqual(result['surface_gap_m'], expected, places=10)
                witness = np.asarray(result['nearest_world_m'])
                self.assertAlmostEqual(np.linalg.norm(witness[0]-witness[1]), expected, places=10)

    def test_crossing_triangles_are_not_reported_as_positive_clearance(self):
        a = collision_object([[0, 0, 0], [1, 0, 0], [0, 1, 0]], [[0, 1, 2]])
        b = collision_object([[0.2, 0.2, -1], [0.2, 0.2, 1], [0.5, 0.2, 0]], [[0, 1, 2]])
        result = surface_distance(a, b)
        self.assertTrue(result['surface_intersection'])
        self.assertEqual(result['surface_gap_m'], 0)
        self.assertIsNone(result['nearest_world_m'])

    def test_signed_flexion_guard_rejects_backward_joint_motion(self):
        from render_hand_reference import forward_flexion
        from mathutils import Vector
        rest = {'pip': Vector((0, 0, 0)), 'tip': Vector((0, 1, 0))}
        normal = Vector((0, 0, 1))
        backward = {n: p+Vector((0, 0, -0.01)) for n, p in rest.items()}
        with self.assertRaisesRegex(ValueError, 'behind the rest palm plane'):
            forward_flexion(rest, backward, normal, 1.0)
        forward = {n: p+Vector((0, 0, 0.01)) for n, p in rest.items()}
        self.assertTrue(all(d > 0 for d in forward_flexion(rest, forward, normal, 1.0).values()))

    def test_invalid_or_degenerate_surfaces_are_unqualified(self):
        for vertices, faces in [([], []), ([[0, 0, float('nan')]], [[0, 0, 0]]),
                                ([[0, 0, 0]], [[0, 0, 0]]), ([[0, 0, 0]], [[0, 1, 2]])]:
            with self.assertRaises(ValueError):
                collision_object(vertices, faces)


if __name__ == '__main__':
    unittest.main()
