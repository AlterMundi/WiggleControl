#!/usr/bin/env python3
"""Orchestration controls for failed display transport, without network/models."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class ProjectionFailure(unittest.TestCase):
    def test_lost_display_preserves_model_completion_and_does_not_mask_model_failure(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            render = root/'render'
            render.mkdir()
            (render/'input.png').write_bytes(b'fixture bytes; fake model does not decode images')
            (render/'sequence.json').write_text(json.dumps(dict(version=1, frames=[
                dict(source_us=1+i*50000, image='input.png') for i in range(9)])))
            (render/'labels.json').write_text(json.dumps(dict(reference_hand_count=1,
                frames=[dict(closure_interpolation=0) for _ in range(9)])))
            config = root/'config.json'
            config.write_text('{}')
            ssh = root/'ssh'
            ssh.write_text('#!/bin/sh\necho "intentional offline display" >&2\nexit 255\n')
            ssh.chmod(0o700)
            binary = root/'fake-model'
            env = dict(os.environ, PATH=str(root)+os.pathsep+os.environ['PATH'])
            for succeeds in (True, False):
                code = ('import json\nfor i in range(9):\n print(json.dumps(dict(frame=i,image="input.png",'
                        'hands=[],perception=[],contacts=[],ambiguous=False,processing_ms=1)))\n') if succeeds else 'raise SystemExit(2)\n'
                binary.write_text('#!'+sys.executable+'\n'+code)
                binary.chmod(0o700)
                output = root/('success' if succeeds else 'failure')
                result = subprocess.run([sys.executable, str(Path(__file__).with_name('run_visible_motion.py')),
                    str(render), str(config), str(output), '--frames', '9', '--binary', str(binary),
                    '--ssh-host', 'unused-fixture', '--ssh-identity', str(root/'unused-key'),
                    '--display-dir', '/unused-fixture'], env=env, text=True, capture_output=True, timeout=15)
                if succeeds:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    report = json.loads((output/'results.json').read_text())
                    self.assertEqual(len(report['predictions']), 9)
                    self.assertTrue(report['projection_errors'])
                    self.assertFalse(report['receiving_display_verified'])
                    self.assertEqual(json.loads((output/'progress.json').read_text())['status'], 'complete')
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('Incomplete sequence inference', result.stderr)
                    self.assertFalse((output/'results.json').exists())


if __name__ == '__main__':
    unittest.main()
