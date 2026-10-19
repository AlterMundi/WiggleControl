#!/usr/bin/env python3
"""Install/update/remove selected preview user units without starting capture.

Prepared bundles and their configuration/assets stay at their selected paths.
A single current symlink selects the version; previous enables explicit rollback.
"""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROLES = ('source', 'worker', 'control')
NAME = re.compile(r'[a-z][a-z0-9-]{0,31}')


def units(name):
    if not NAME.fullmatch(name):
        raise ValueError('Expected a short lowercase installation name')
    return {role: f'wigglecontrol-preview-{name}-{role}.service' for role in ROLES}


def read_json(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 128 * 1024:
        raise ValueError('Expected a bounded regular manifest')
    return json.loads(path.read_text())


def digest(path):
    with path.open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()


def bundle(root):
    root = Path(root).resolve(strict=True)
    manifest = read_json(root / 'bundle.json')
    match = re.fullmatch(r'wigglecontrol-preview-([a-z][a-z0-9-]{0,31})-source.service',
                         manifest.get('units', {}).get('source', ''))
    if manifest.get('schema') != 'wigglecontrol.preview-launcher/v1' or not match:
        raise ValueError('Expected a prepared preview launcher')
    name = match[1]
    expected = units(name)
    if manifest['units'] != expected:
        raise ValueError('The three selected units must belong to one installation')
    required = {'open.sh', 'stop.sh', 'mouse_launcher.py',
                *('units/' + name for name in expected.values())}
    if set(manifest.get('files', {})) != required:
        raise ValueError('Prepared launcher inventory differs')
    for filename, expected_digest in manifest['files'].items():
        path = root / filename
        if path.is_symlink() or not path.is_file() or digest(path) != expected_digest:
            raise ValueError('Prepared launcher file changed: ' + filename)
    if set(manifest.get('bindings', {})) != {'binary', 'session', 'settings', 'warmup'}:
        raise ValueError('Prepared artifact bindings differ')
    for key, binding in manifest['bindings'].items():
        path = Path(binding['path'])
        if not path.is_absolute() or not path.is_file() or digest(path) != binding['sha256']:
            raise ValueError('Selected artifact/configuration changed: ' + key)
    if not all(os.access(root / filename, os.X_OK) for filename in ('open.sh', 'stop.sh')):
        raise ValueError('Prepared launch wrappers are not executable')
    return root, name


class Controller:
    def run(self, *args):
        return subprocess.run(['systemctl', '--user', *args], check=True,
                              capture_output=True, text=True, timeout=30)

    def show(self, name):
        result = subprocess.run(['systemctl', '--user', 'show', name,
                                 '--property=LoadState,FragmentPath,ActiveState'],
                                capture_output=True, text=True, timeout=10)
        fields = dict(line.split('=', 1) for line in result.stdout.splitlines() if '=' in line)
        if fields.get('LoadState') not in ('loaded', 'not-found'):
            raise ValueError('Cannot establish selected user-unit ownership: ' + name)
        return fields


class Installation:
    def __init__(self, unit_dir=None, state_dir=None, controller=None):
        home = Path.home()
        self.unit_dir = Path(unit_dir or Path(os.environ.get('XDG_CONFIG_HOME') or home / '.config') / 'systemd/user')
        self.state_dir = Path(state_dir or Path(os.environ.get('XDG_STATE_HOME') or home / '.local/state') / 'wigglecontrol/preview-installations')
        if not self.unit_dir.is_absolute() or not self.state_dir.is_absolute():
            raise ValueError('User configuration/state locations must be absolute')
        self.controller = controller or Controller()

    @contextmanager
    def selected(self, name, create=False):
        selected = units(name)
        root = self.state_dir / name
        if create:
            root.mkdir(mode=0o700, parents=True, exist_ok=True)
        elif not root.is_dir():
            raise ValueError('No managed installation with this name')
        if root.is_symlink() or root.stat().st_uid != os.geteuid():
            raise ValueError('Installation state must belong to this user')
        fd = os.open(root / '.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'r+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            owner = {'schema': 'wigglecontrol.preview-installation/v1', 'name': name,
                     'units': selected, 'unit_directory': str(self.unit_dir)}
            receipt = root / 'owner.json'
            if receipt.exists() or receipt.is_symlink():
                if read_json(receipt) != owner:
                    raise ValueError('Installation ownership/location differs')
            else:
                if not create:
                    raise ValueError('No installation ownership receipt')
                with receipt.open('x') as file:
                    json.dump(owner, file)
            yield root, selected

    @staticmethod
    def pointer(path):
        if path.is_symlink():
            target = Path(os.readlink(path))
            if not target.is_absolute():
                raise ValueError('Installation selector must be absolute')
            return target
        if path.exists():
            raise ValueError('Preserve an unexpected installation-state file')
        return None

    @staticmethod
    def point(path, target):
        Installation.pointer(path)  # Preserve unexpected regular files.
        if target is None:
            path.unlink(missing_ok=True)
            return
        temp = path.with_name(path.name + '.new')
        # The per-installation lock serializes our writers. Unknown leftovers
        # are preserved rather than deleted on behalf of another operation.
        temp.symlink_to(target)
        try:
            temp.replace(path)
        finally:
            temp.unlink(missing_ok=True)

    def inspect(self, root, selected):
        loaded = []
        links = []
        # FragmentPath may retain the stable link rather than the loaded
        # version's path. A completed-reload receipt also detects a crash
        # between switching current and asking systemd to reload its definition.
        stale = self.pointer(root / 'loaded') != self.pointer(root / 'current')
        previous = self.pointer(root / 'previous')
        for name in selected.values():
            link = self.unit_dir / name
            target = root / 'current/units' / name
            if link.is_symlink():
                if os.readlink(link) != str(target):
                    raise ValueError('Preserve a foreign unit link: ' + name)
                links.append(link)
            elif link.exists():
                raise ValueError('Preserve a foreign unit file: ' + name)
            observed = self.controller.show(name)
            fragment = observed.get('FragmentPath', '')
            if fragment and Path(fragment).resolve() != target.resolve():
                if previous is None or Path(fragment).resolve() != (previous / 'units' / name).resolve():
                    raise ValueError('Preserve a loaded foreign unit: ' + name)
                stale = True  # Interrupted update: selector changed before reload.
            if observed['LoadState'] == 'loaded':
                if not fragment:
                    raise ValueError('Preserve a loaded foreign unit: ' + name)
                loaded.append(name)
            elif observed.get('ActiveState', 'inactive') not in ('inactive', 'failed'):
                loaded.append(name)  # A removed unit file can still have a live process.
        return links, loaded, stale

    def activate(self, root, selected, target):
        old = self.pointer(root / 'current')
        previous = self.pointer(root / 'previous')
        loaded_version = self.pointer(root / 'loaded')
        links, loaded, stale = self.inspect(root, selected)
        if old == target and len(links) == 3 and len(loaded) == 3 and not stale:
            return {'installed': True, 'changed': False, 'started': False}
        # Updating stops only this installation; it never starts the new source,
        # a detector, capture, another application or a coordinated global Stop.
        if (old != target or stale) and loaded:
            self.controller.run('stop', *loaded)
        self.unit_dir.mkdir(parents=True, exist_ok=True)
        created = []
        try:
            if old != target and old is not None:
                self.point(root / 'previous', old)
            self.point(root / 'current', target)
            for name in selected.values():
                link = self.unit_dir / name
                if link.is_symlink():
                    if os.readlink(link) != str(root / 'current/units' / name):
                        raise ValueError('Unit link changed during installation: ' + name)
                else:
                    link.symlink_to(root / 'current/units' / name)
                    created.append(link)
            self.controller.run('daemon-reload')
            self.point(root / 'loaded', target)
        except Exception:
            for link in created:
                if link.is_symlink() and os.readlink(link) == str(root / 'current/units' / link.name):
                    link.unlink()
            self.point(root / 'current', old)
            self.point(root / 'previous', previous)
            self.controller.run('daemon-reload')
            self.point(root / 'loaded', loaded_version)
            raise
        return {'installed': True, 'changed': True, 'started': False}

    def install(self, path):
        target, name = bundle(path)
        with self.selected(name, create=True) as (root, selected):
            return self.activate(root, selected, target) | {'name': name}

    def rollback(self, name):
        with self.selected(name) as (root, selected):
            previous = self.pointer(root / 'previous')
            if previous is None:
                raise ValueError('No previous prepared bundle is retained')
            target, found = bundle(previous)
            if found != name:
                raise ValueError('Previous bundle belongs to another installation')
            return self.activate(root, selected, target) | {'name': name}

    def uninstall(self, name):
        with self.selected(name) as (root, selected):
            links, loaded, _ = self.inspect(root, selected)
            if loaded:
                self.controller.run('stop', *loaded)
            for link in links:
                if not link.is_symlink() or os.readlink(link) != str(root / 'current/units' / link.name):
                    raise ValueError('Unit link changed during removal: ' + link.name)
                link.unlink()
            self.controller.run('daemon-reload')
            return {'name': name, 'installed': False, 'artifacts_preserved': True}

    def operate(self, name, action):
        with self.selected(name) as (root, selected):
            links, _, stale = self.inspect(root, selected)
            if action == 'stop':
                self.controller.run('stop', *selected.values())
            else:
                if stale:
                    raise ValueError('Reinstall the selected bundle to finish its interrupted reload')
                if len(links) != 3:
                    raise ValueError('Install this launcher before opening it')
                target, found = bundle(root / 'current')
                if found != name:
                    raise ValueError('Current bundle belongs to another installation')
                subprocess.run([str(target / 'open.sh')], check=True)
            return {'name': name, 'action': action}


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('install', 'rollback', 'uninstall', 'open', 'stop'))
    parser.add_argument('selection', help='Prepared bundle directory for install; installation name otherwise')
    args = parser.parse_args()
    manager = Installation()
    if args.action in ('open', 'stop'):
        result = manager.operate(args.selection, args.action)
    else:
        result = getattr(manager, args.action)(args.selection)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
