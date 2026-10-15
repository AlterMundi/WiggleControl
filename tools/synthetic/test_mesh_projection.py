"""A live observer must not parse or skip an in-progress producer append."""
import json
from pathlib import Path
import tempfile
import unittest

from project_mesh_probe import read_complete_rows


class AppendedObservations(unittest.TestCase):
    def test_partial_append_is_retained_until_newline_and_never_repeated(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'raw.jsonl'
            self.assertEqual(read_complete_rows(path,0),(0,[]))
            path.write_text('{"frame":0}\n{"frame":')
            offset, rows = read_complete_rows(path,0)
            self.assertEqual(rows,[dict(frame=0)])
            self.assertEqual(read_complete_rows(path,offset),(offset,[]))
            with path.open('a') as stream:
                stream.write('1}\n{"frame":2}\n')
            end, rows = read_complete_rows(path,offset)
            self.assertEqual(rows,[dict(frame=1),dict(frame=2)])
            self.assertEqual(read_complete_rows(path,end),(end,[]))
            with path.open('a') as stream:
                stream.write('invalid completed line\n')
            with self.assertRaises(json.JSONDecodeError):
                read_complete_rows(path,end)


if __name__ == '__main__':
    unittest.main()
