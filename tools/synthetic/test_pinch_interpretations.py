"""Prevent geometry substitution and incomplete scoring from hiding false contacts."""
import copy
import math
import unittest

from compare_pinch_interpretations import encoded_hands, matched_geometry, score


class PinchInterpretationEvidence(unittest.TestCase):
    def reference(self):
        return dict(asset_sha256='fixture', pose_recipe='fixture',
                    target_joint_angles_rad={}, macro_abduction_target_rad={},
                    camera_world_matrix=[], frames=[dict(frame=0, source_us=1,
                        closure_interpolation=1, world_m=[[0, 0, 0]],
                        distal_skin=dict(surface_gap_m=.08, surface_intersection=False))])

    def test_replacement_requires_same_geometry_and_source_time(self):
        original = self.reference()
        for key, value in [('world_m', [[0, 0, .001]]), ('source_us', 2),
                           ('world_m', [[0, 0, float('nan')]])]:
            altered = copy.deepcopy(original)
            altered['frames'][0][key] = value
            with self.assertRaises(ValueError):
                matched_geometry(original, altered)
        self.assertEqual(matched_geometry(original, copy.deepcopy(original))[1], 0)

    def test_far_skin_contact_is_counted_even_without_a_close_hold(self):
        reference = self.reference()
        row = dict(frame=0, source_us=1, hands=[{}])
        result = score(reference, [(row, dict(contacts=[dict(id=1)]))])
        self.assertEqual(result['close_hold_frames'], 0)
        self.assertEqual(result['separated_50mm_contacts'], 1)
        self.assertEqual(result['contact_onsets'][0]['skin_gap_m'], .08)
        reference['frames'][0]['distal_skin']['surface_intersection'] = True
        with self.assertRaisesRegex(ValueError, 'unqualified'):
            score(reference, [(row, dict(contacts=[]))])

    def test_probe_encoding_preserves_originals_and_rejects_degenerate_scale(self):
        points = [[0, 0, 0] for _ in range(21)]
        points[9], points[4] = [0, .1, 0], [.02, 0, 0]
        row = dict(hands=[dict(id=7, pinch_distance_m=.02)],
                   perception=[dict(id=7, world_landmarks=points)])
        before = copy.deepcopy(row)
        self.assertTrue(math.isclose(encoded_hands(row, True)[0]['pinch_distance_m'], .02))
        self.assertEqual(row, before)
        points[9] = [0, 0, 0]
        with self.assertRaisesRegex(ValueError, 'Unqualified'):
            encoded_hands(row, True)

    def test_pixel_ratio_is_distinct_from_world_ratio_and_keeps_native_authority(self):
        world = [[0, 0, 0] for _ in range(21)]
        world[9], world[4] = [0, .1, 0], [.02, 0, 0]
        pixels = [[0, 0] for _ in range(21)]
        pixels[9], pixels[4] = [0, 100], [40, 0]
        row = dict(hands=[dict(id=7, confidence=.91, pinch_eligible=True, pinch_distance_m=.02)],
                   perception=[dict(id=7, world_landmarks=world, landmarks=pixels)])
        before = copy.deepcopy(row)
        screen = encoded_hands(row, 'screen')
        self.assertTrue(math.isclose(screen[0]['pinch_distance_m'], .04))
        self.assertTrue(math.isclose(encoded_hands(row, True)[0]['pinch_distance_m'], .02))
        self.assertEqual(row, before)
        expected = copy.deepcopy(row['hands'])
        expected[0]['pinch_distance_m'] = screen[0]['pinch_distance_m']
        self.assertEqual(screen, expected)
        pixels[9] = [0,0]
        with self.assertRaisesRegex(ValueError, 'Unqualified'):
            encoded_hands(row, 'screen')


if __name__ == '__main__':
    unittest.main()
