#!/usr/bin/env python3
"""User-unit lifecycle with real prepared files and a device-free controller."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import manage_preview_launcher as manager
import prepare_preview_launcher as preparation


class Controller:
    def __init__(self, unit_dir):
        self.unit_dir = unit_dir
        self.calls = []
        self.foreign = {}
        self.fail_reload = False
        self.on_stop = None
        self.observations = {}

    def show(self, name):
        if name in self.observations:
            return self.observations[name]
        if name in self.foreign:
            return {'LoadState': 'loaded', 'FragmentPath': self.foreign[name]}
        path = self.unit_dir / name
        return {'LoadState': 'loaded' if path.exists() else 'not-found',
                'FragmentPath': str(path) if path.exists() else ''}

    def run(self, *args):
        self.calls.append(args)
        if args[0] == 'stop' and self.on_stop:
            self.on_stop()
        if args[0] == 'daemon-reload' and self.fail_reload:
            self.fail_reload = False
            raise subprocess.CalledProcessError(1, args)


class LifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.unit_dir = self.root / 'config/systemd/user'
        self.control = Controller(self.unit_dir)
        self.installation = manager.Installation(self.unit_dir, self.root / 'state', self.control)

    def tearDown(self):
        self.temp.cleanup()

    def bundle(self, version, name='mouse'):
        root = self.root / version
        root.mkdir()
        for filename in ('binary', 'browser', 'session.json', 'settings.json', 'warmup.jpg'):
            (root / filename).write_text('fixture-' + version)
        for filename in ('binary', 'browser'):
            (root / filename).chmod(0o700)
        (root / 'assets').mkdir()
        proof = {'preflight': 'ok', 'manual_start': True, 'camera_opened': False,
                 'system_input': False, 'acquisition_started': False,
                 'absolute_pointer': True, 'max_hands': 1, 'recording_configured': True,
                 'until_stop': True, 'trial_seconds': None}
        with patch.object(preparation.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, json.dumps(proof), '')):
            preparation.prepare(binary=root / 'binary', browser=root / 'browser',
                session=root / 'session.json', settings=root / 'settings.json',
                assets=root / 'assets', warmup=root / 'warmup.jpg', output=root / 'bundle',
                port=18876, seconds=0, name=name)
        return root / 'bundle'

    def selector(self, name, which):
        return self.root / 'state' / name / which

    def test_install_update_rollback_uninstall_preserve_versions_and_never_start_units(self):
        first, second = self.bundle('v1'), self.bundle('v2')
        before = {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                  for root in (first.parent, second.parent) for p in root.rglob('*') if p.is_file()}
        self.assertTrue(self.installation.install(first)['installed'])
        self.assertEqual(self.control.calls, [('daemon-reload',)])
        links = {name: os.readlink(self.unit_dir / name) for name in manager.units('mouse').values()}
        self.assertFalse(self.installation.install(first)['changed'])
        self.assertEqual(self.control.calls, [('daemon-reload',)], 'Idempotent install does not stop current use')
        self.installation.install(second)
        self.assertEqual(self.selector('mouse', 'current').resolve(), second)
        self.assertEqual(self.selector('mouse', 'previous').resolve(), first)
        self.assertEqual({name: os.readlink(self.unit_dir / name) for name in links}, links)
        self.installation.rollback('mouse')
        self.assertEqual(self.selector('mouse', 'current').resolve(), first)
        self.assertEqual(self.selector('mouse', 'previous').resolve(), second)
        self.installation.uninstall('mouse')
        self.assertFalse(any((self.unit_dir / name).is_symlink() for name in links))
        self.assertTrue(self.selector('mouse', 'current').is_symlink(), 'Preserve retained versions/configuration')
        for command in self.control.calls:
            self.assertIn(command[0], ('stop', 'daemon-reload'))
            if command[0] == 'stop': self.assertEqual(set(command[1:]), set(links))
        for path, digest in before.items():
            self.assertEqual(hashlib.sha256(Path(path).read_bytes()).hexdigest(), digest)
        self.installation.install(second)  # Reinstall after removal is supported.
        self.assertEqual(self.selector('mouse', 'current').resolve(), second)

    def test_foreign_unit_file_link_and_loaded_unit_are_never_replaced_or_stopped(self):
        prepared = self.bundle('v1')
        self.unit_dir.mkdir(parents=True)
        source = manager.units('mouse')['source']
        foreign = self.unit_dir / source
        foreign.write_text('preserve')
        with self.assertRaisesRegex(ValueError, 'foreign unit file'): self.installation.install(prepared)
        self.assertEqual(foreign.read_text(), 'preserve')
        foreign.unlink(); foreign.symlink_to('/fictional/foreign.service')
        with self.assertRaisesRegex(ValueError, 'foreign unit link'): self.installation.install(prepared)
        self.assertEqual(os.readlink(foreign), '/fictional/foreign.service')
        foreign.unlink(); self.control.foreign[source] = '/fictional/system/foreign.service'
        with self.assertRaisesRegex(ValueError, 'loaded foreign unit'): self.installation.install(prepared)
        self.assertEqual(self.control.calls, [])

    def test_failed_reload_restores_the_old_selection_without_starting_either_version(self):
        first, second = self.bundle('v1'), self.bundle('v2')
        self.installation.install(first)
        self.control.fail_reload = True
        with self.assertRaises(subprocess.CalledProcessError): self.installation.install(second)
        self.assertEqual(self.selector('mouse', 'current').resolve(), first)
        self.assertFalse(self.selector('mouse', 'previous').exists())
        self.assertTrue(all((self.unit_dir / name).is_file() for name in manager.units('mouse').values()))
        self.assertFalse(any(command[0] == 'start' for command in self.control.calls))

    def test_changed_bindings_refuse_update_but_do_not_prevent_removing_a_broken_installation(self):
        first, second = self.bundle('v1'), self.bundle('v2')
        self.installation.install(first)
        (second.parent / 'session.json').write_text('changed')
        self.control.calls.clear()
        with self.assertRaisesRegex(ValueError, 'changed: session'): self.installation.install(second)
        self.assertEqual(self.control.calls, [])
        (first.parent / 'binary').unlink()
        self.assertFalse(self.installation.uninstall('mouse')['installed'])

    def test_a_unit_changed_while_stopping_is_preserved_during_uninstall(self):
        self.installation.install(self.bundle('v1'))
        source = self.unit_dir / manager.units('mouse')['source']
        def replace():
            source.unlink(); source.write_text('foreign change during Stop')
        self.control.on_stop = replace
        with self.assertRaisesRegex(ValueError, 'changed during removal'): self.installation.uninstall('mouse')
        self.assertEqual(source.read_text(), 'foreign change during Stop')

    def test_unknown_names_and_tampered_bundle_inventory_cannot_touch_unrelated_units(self):
        for name in ('../other', 'other.service', 'bad/name', ''):
            with self.assertRaises(ValueError): self.installation.uninstall(name)
        with self.assertRaisesRegex(ValueError, 'No managed'): self.installation.uninstall('unknown')
        prepared = self.bundle('v1')
        manifest = json.loads((prepared / 'bundle.json').read_text())
        manifest['files']['../../foreign'] = '0' * 64
        (prepared / 'bundle.json').write_text(json.dumps(manifest))
        with self.assertRaisesRegex(ValueError, 'inventory differs'): self.installation.install(prepared)
        self.assertEqual(self.control.calls, [])

    def test_interrupted_selector_change_is_reloaded_before_the_new_version_can_open(self):
        first, second = self.bundle('v1'), self.bundle('v2')
        self.installation.install(first)
        self.installation.point(self.selector('mouse', 'previous'), first)
        self.installation.point(self.selector('mouse', 'current'), second)
        for name in manager.units('mouse').values():
            self.control.observations[name] = {'LoadState': 'loaded', 'FragmentPath': str(first / 'units' / name)}
        with self.assertRaisesRegex(ValueError, 'interrupted reload'):
            self.installation.operate('mouse', 'open')
        self.control.calls.clear()
        self.installation.install(second)
        self.assertEqual(self.control.calls, [('stop', *manager.units('mouse').values()), ('daemon-reload',)])

    def test_stable_fragment_links_still_detect_an_interrupted_update(self):
        first, second = self.bundle('v1'), self.bundle('v2')
        self.installation.install(first)
        self.installation.point(self.selector('mouse', 'previous'), first)
        self.installation.point(self.selector('mouse', 'current'), second)
        # Actual systemd may report the stable user-unit link as FragmentPath;
        # resolving it now cannot reveal which version systemd previously read.
        with self.assertRaisesRegex(ValueError, 'interrupted reload'):
            self.installation.operate('mouse', 'open')
        self.control.calls.clear()
        self.installation.install(second)
        self.assertEqual(self.control.calls, [('stop', *manager.units('mouse').values()), ('daemon-reload',)])
        self.assertEqual(self.selector('mouse', 'loaded').resolve(), second)

    def test_removal_stops_a_live_unit_even_when_its_file_has_disappeared(self):
        prepared = self.bundle('v1')
        self.installation.install(prepared)
        source = manager.units('mouse')['source']
        (prepared / 'units' / source).unlink()
        self.control.observations[source] = {'LoadState': 'not-found', 'ActiveState': 'active', 'FragmentPath': ''}
        self.control.calls.clear()
        self.installation.uninstall('mouse')
        self.assertIn(source, self.control.calls[0][1:])
        self.assertFalse((self.unit_dir / source).is_symlink())


if __name__ == '__main__':
    unittest.main()
