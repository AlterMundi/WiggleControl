#!/usr/bin/env python3
"""Offline unit preparation: no services, browsers, inference or devices start."""
import json
import io
from pathlib import Path
import shlex
import subprocess
import tempfile
import sys
import unittest
from unittest.mock import patch

import prepare_preview_launcher as launcher
import mouse_launcher


class LauncherTests(unittest.TestCase):
    def fixture(self, root, seconds=90):
        for name in ('native', 'browser', 'session.json', 'settings.json', 'warmup.jpg'):
            (root / name).write_text('fixture')
        for name in ('native', 'browser'):
            (root / name).chmod(0o700)
        (root / 'assets').mkdir()
        kwargs = {key: root / value for key, value in {
            'binary': 'native', 'browser': 'browser', 'session': 'session.json',
            'settings': 'settings.json', 'assets': 'assets', 'warmup': 'warmup.jpg',
            'output': 'prepared $50% bundle'}.items()}
        kwargs.update(seconds=seconds, port=18876)
        proof = {'preflight': 'ok', 'manual_start': True, 'camera_opened': False,
                 'system_input': False, 'acquisition_started': False, 'absolute_pointer': True,
                 'max_hands': 1, 'recording_configured': True,
                 'until_stop': seconds == 0, 'trial_seconds': seconds or None}
        return kwargs, proof

    def test_finite_and_until_stop_prepare_only_the_selected_preflight(self):
        for seconds in (90, 0):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); kwargs, proof = self.fixture(root, seconds)
                result = subprocess.CompletedProcess([], 0, json.dumps(proof), '')
                with patch.object(launcher.subprocess, 'run', return_value=result) as call:
                    manifest = launcher.prepare(**kwargs)
                self.assertEqual(call.call_count, 1)
                cmd = call.call_args.args[0]
                self.assertEqual(cmd[1], 'provider-camera-' + ('manual' if seconds else 'until-stop') + '-preflight')
                output = kwargs['output']
                self.assertFalse(manifest['installed']); self.assertFalse(manifest['started'])
                self.assertEqual(output.stat().st_mode & 0o777, 0o700)
                for role, name in manifest['units'].items():
                    unit = (output / 'units' / name).read_text()
                    self.assertNotIn('[Install]', unit)
                    self.assertIn('Restart=no\n', unit)
                    self.assertIn('KillMode=control-group\n', unit)
                    self.assertNotIn('daimon', unit)
                    self.assertNotIn('--remote-debugging', unit)
                    self.assertNotIn('--no-sandbox', unit)
                    if role == 'worker':
                        self.assertIn('BindsTo=' + manifest['units']['source'], unit)
                    if role == 'control':
                        self.assertNotIn('BindsTo=', unit, 'Controls retain the terminal result after source exit')
                        self.assertIn('After=' + manifest['units']['source'], unit)
                    if role == 'worker':
                        self.assertIn('/provider/worker', unit)
                        self.assertNotIn('/provider/feedback', unit)
                        self.assertIn('$$50%%', unit)
                for action in ('open', 'stop'):
                    script = (output / (action + '.sh')).read_text()
                    args = shlex.split(script.split('exec ', 1)[1])
                    self.assertEqual(args[1], str(output / 'mouse_launcher.py'))
                    self.assertEqual(args[2], action)
                    self.assertIn(manifest['units']['source'], args)
                    self.assertIn('--require-direct-source', args)
                self.assertEqual(json.loads((output / 'bundle.json').read_text()), manifest)

    def test_existing_bundle_and_invalid_input_are_preserved_without_preflight(self):
        with tempfile.TemporaryDirectory() as tmp:
            kwargs, _ = self.fixture(Path(tmp))
            with patch.object(launcher.subprocess, 'run', side_effect=AssertionError('No process')):
                for change in ({'seconds': True}, {'seconds': 91}, {'port': 80}, {'name': 'bad/name'}):
                    with self.assertRaises(ValueError):
                        launcher.prepare(**(kwargs | change))
                kwargs['output'].mkdir(); marker = kwargs['output'] / 'existing'
                marker.write_text('preserve')
                with self.assertRaisesRegex(ValueError, 'Preserve'):
                    launcher.prepare(**kwargs)
                self.assertEqual(marker.read_text(), 'preserve')

    def test_nonmanual_or_active_preflight_cannot_publish_a_bundle(self):
        with tempfile.TemporaryDirectory() as tmp:
            kwargs, proof = self.fixture(Path(tmp))
            for key in ('manual_start', 'camera_opened', 'system_input', 'acquisition_started', 'trial_seconds'):
                invalid = proof | {key: 0 if key == 'trial_seconds' else not proof[key]}
                with patch.object(launcher.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, json.dumps(invalid), '')):
                    with self.assertRaisesRegex(ValueError, 'manual-only'):
                        launcher.prepare(**kwargs)
                self.assertFalse(kwargs['output'].exists())

    def test_unit_arguments_escape_systemd_and_reject_control_characters(self):
        self.assertEqual(launcher.unit_arg('/a path/100%/$HOME/"x"'), '"/a path/100%%/$$HOME/\\"x\\""')
        for value in ('/a\nb', '/a\tb', '/a\x00b'):
            with self.assertRaises(ValueError): launcher.unit_arg(value)

    def test_source_process_binding_rejects_another_provider_on_the_same_port(self):
        config = {'manual_start': True, 'absolute_pointer': True, 'max_hands': 1,
                  'prepared': False, 'run_id': '123-456'}
        mouse_launcher.validate_source(config, 123)
        for pid in (0, 12, 124):
            with self.assertRaisesRegex(ValueError, 'service process'):
                mouse_launcher.validate_source(config, pid)
        # Existing wrapper services may own a native child rather than MainPID;
        # their established launch route is unchanged unless explicitly opted in.
        mouse_launcher.validate_source(config)
        with self.assertRaisesRegex(ValueError, 'manual mouse'):
            mouse_launcher.validate_source(config | {'manual_start': False}, 123)

    def test_open_does_not_launch_worker_or_controls_for_a_different_service_pid(self):
        config = {'manual_start': True, 'absolute_pointer': True, 'max_hands': 1,
                  'prepared': False, 'run_id': '123-456'}
        for pid in (123, 124):
            calls = []
            def run(args, **kwargs):
                calls.append(args)
                return subprocess.CompletedProcess(args, 0, str(pid) + '\n', '')
            with patch.object(sys, 'argv', ['mouse_launcher.py', 'open', '--require-direct-source']), \
                    patch.object(mouse_launcher.subprocess, 'run', side_effect=run), \
                    patch.object(mouse_launcher.urllib.request, 'urlopen', return_value=io.StringIO(json.dumps(config))):
                if pid == 123:
                    mouse_launcher.main()
                else:
                    with self.assertRaisesRegex(ValueError, 'service process'):
                        mouse_launcher.main()
            self.assertEqual(len(calls), 4 if pid == 123 else 2)
            self.assertEqual(calls[0][:3], ['systemctl', '--user', 'start'])
            self.assertIn('--property=MainPID', calls[1])


if __name__ == '__main__':
    unittest.main()
