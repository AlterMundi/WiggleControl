"""Known-crop boundaries are checked without importing the optional model stack."""
import json
from pathlib import Path
import tempfile
import unittest

from wilor_probe import load_selection, write_progress


class ReconstructionSelection(unittest.TestCase):
    def test_progress_replacement_keeps_an_open_reader_on_a_complete_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            previous, current = dict(status='running', frame=0), dict(status='complete', predictions=143)
            write_progress(root, previous)
            with (root/'progress.json').open() as reader:
                write_progress(root, current)
                self.assertEqual(json.load(reader), previous)
            self.assertEqual(json.loads((root/'progress.json').read_text()), current)

    def test_unknown_side_invalid_box_and_escaping_input_are_not_model_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'input.png').write_bytes(b'fixture; selection validation does not decode images')
            path = root/'selection.json'
            original = dict(name='fixture', image='input.png', bbox=[0, 0, 40, 40], right=True)
            path.write_text(json.dumps(dict(version=1, cases=[original])))
            self.assertEqual(len(load_selection(path)['cases']), 1)
            for changes in [dict(right=False), dict(bbox=[0, 0, float('nan'), 40]),
                            dict(bbox=[40, 0, 0, 40]), dict(image='../outside.png')]:
                path.write_text(json.dumps(dict(version=1, cases=[dict(original, **changes)])))
                with self.assertRaises(ValueError):
                    load_selection(path)

    def test_symlink_escape_is_rejected_before_decode_or_model_load(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inner = root/'selection'
            inner.mkdir()
            (root/'outside.png').write_bytes(b'outside')
            (inner/'linked.png').symlink_to(root/'outside.png')
            path = inner/'selection.json'
            path.write_text(json.dumps(dict(version=1, cases=[dict(
                image='linked.png', bbox=[0, 0, 40, 40], right=True)])))
            with self.assertRaisesRegex(ValueError, 'escapes'):
                load_selection(path)


if __name__ == '__main__':
    unittest.main()
