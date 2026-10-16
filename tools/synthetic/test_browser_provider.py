#!/usr/bin/env python3
"""Wire provenance/handedness checks; actual tracking/meaning lives in Rust tests."""
import hashlib
import tempfile
from pathlib import Path
import unittest
from replay_browser_provider import commands, right_probability


class ProviderWire(unittest.TestCase):
    def test_category_is_only_handedness_and_wire_inherits_no_native_fields(self):
        category = [dict(categoryName='Left', score=.9)]
        self.assertAlmostEqual(right_probability(category), .1)
        with tempfile.TemporaryDirectory() as root:
            image = Path(root)/'frame.png'
            image.write_bytes(b'independent hash fixture; no image decoding in wire adapter')
            row = dict(frame=0, source_us=1, image=image.name,
                image_sha256=hashlib.sha256(image.read_bytes()).hexdigest(),
                camera_opened=False, system_input=False,
                hands=[dict(landmarks=[[1,2]]*21, world_landmarks=[[0,0,0]]*21,
                            handedness=category, array_index=0)])
            case = dict(reference=dict(image_size=[640,480], frames=[dict(frame=0,source_us=1)]),
                        predictions=[row], camera_opened=False, system_input=False)
            frame = commands(case, Path(root), 130000)[1]['observation']
            self.assertEqual(frame['now_us'], 130001)
            self.assertEqual(set(frame['hands'][0]),
                             {'landmarks','world_landmarks','right_probability'})
            depths = [-i*2.0 for i in range(21)]
            row['hands'][0]['image_depths'] = depths
            complete = commands(case, Path(root), 0)[1]['observation']['hands'][0]
            self.assertEqual(complete['image_depths'], depths)
            self.assertEqual(complete['world_landmarks'], [[0,0,0]]*21)
            # Bound includes the complete XYZ pair, not just legacy XY packets.
            row['hands'] *= 2
            self.assertEqual(len(commands(case, Path(root), 0)[1]['observation']['hands']), 2)
            row['image_sha256'] = '0'*64
            with self.assertRaisesRegex(ValueError,'digest'):
                commands(case, Path(root), 0)
            row['image'] = '../frame.png'
            with self.assertRaisesRegex(ValueError,'escapes'):
                commands(case, Path(root), 0)

    def test_unknown_nonfinite_or_multiple_categories_are_not_invented_scores(self):
        for categories in ([], [dict(categoryName='Unknown',score=.9)],
                           [dict(categoryName='Right',score=float('nan'))],
                           [dict(categoryName='Left',score=.8)]*2):
            with self.assertRaises(ValueError):
                right_probability(categories)


if __name__ == '__main__':
    unittest.main()
