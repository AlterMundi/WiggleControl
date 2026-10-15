"""A model replacement cannot silently change source, confidence or ownership."""
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest

from compare_mesh_metric import substitute


class MatchedMetric(unittest.TestCase):
    def test_only_the_matched_metric_changes_and_incomplete_or_wrong_sources_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            image = root/'input.png'
            image.write_bytes(b'identical retained image')
            native = [dict(frame=0, source_us=1, image='input.png', hands=[dict(
                id=7, confidence=.91, pinch_eligible=True, pinch_distance_m=.04)])]
            before = copy.deepcopy(native)
            selection = dict(cases=[dict(reference_case='fixture', source_frame=0, source_us=1)])
            rows = [dict(frame=0, reference_source_frame=0, focal='provider_default',
                         image_sha256=hashlib.sha256(image.read_bytes()).hexdigest(), gap_m=.1)]
            changed = substitute(native, selection, rows, 'fixture', root, 'provider_default')
            self.assertEqual(native, before)
            expected = copy.deepcopy(native)
            expected[0]['hands'][0]['pinch_distance_m'] = .1
            self.assertEqual(changed, expected)
            for bad in ([], rows+rows, [dict(rows[0], image_sha256='wrong')],
                        [dict(rows[0], gap_m=float('nan'))]):
                with self.assertRaises(ValueError):
                    substitute(native, selection, bad, 'fixture', root, 'provider_default')
            selection['cases'][0]['source_us'] = 2
            with self.assertRaises(ValueError):
                substitute(native, selection, rows, 'fixture', root, 'provider_default')


if __name__ == '__main__':
    unittest.main()
