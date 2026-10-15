"""Reject unsafe reference substitutions before Blender can render them."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from pinch_pose import DEFAULT_ANGLES, load


class PinchPose(unittest.TestCase):
    def test_known_dorsal_pose_and_invalid_joint_values_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'pose.json'
            for invalid in (-1.57079632679, float('nan'), float('inf'), True):
                angles = dict(DEFAULT_ANGLES, r_index1=invalid)
                path.write_text(json.dumps(dict(version=1, asset_sha256='asset', joint_angles_rad=angles)))
                with self.assertRaisesRegex(ValueError, 'bounds'):
                    load(path, 'asset')

    def test_exact_asset_and_joint_set_are_required_and_loaded_bytes_are_pinned(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'pose.json'
            data = dict(version=1, asset_sha256='asset', joint_angles_rad=dict(DEFAULT_ANGLES))
            path.write_text(json.dumps(data))
            angles, digest = load(path, 'asset')
            self.assertEqual(angles, DEFAULT_ANGLES)
            self.assertEqual(digest, hashlib.sha256(path.read_bytes()).hexdigest())
            with self.assertRaisesRegex(ValueError, 'exact asset'):
                load(path, 'other asset')
            data['joint_angles_rad']['unknown_joint'] = 0
            path.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'seven'):
                load(path, 'asset')


if __name__ == '__main__':
    unittest.main()
