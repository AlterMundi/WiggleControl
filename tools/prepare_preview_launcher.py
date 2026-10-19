#!/usr/bin/env python3
"""Prepare independent manual-only user units; do not install or start them.

Reuse the existing mouse launcher and Rust manual-Start provider. This is a
receiving preparation component, not a release installer or calibration wizard.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import tempfile


def unit_arg(value):
    # systemd ExecStart is an argv parser, not a shell. Escape its own specifier
    # and environment expansion as well as quoting; never interpolate a shell.
    text = str(value)
    if any(ord(c) < 32 or ord(c) == 127 for c in text):
        raise ValueError('Control characters are not supported in unit arguments')
    return '"' + text.replace('\\', '\\\\').replace('"', '\\"').replace('%', '%%').replace('$', '$$') + '"'


def selected(path, directory=False, executable=False):
    path = Path(path)
    if not path.is_absolute() or not (path.is_dir() if directory else path.is_file()):
        raise ValueError('Select existing absolute artifact/configuration paths')
    unit_arg(path)
    if executable and not os.access(path, os.X_OK):
        raise ValueError('Selected program is not executable')
    return path


def command(binary, session, settings, assets, warmup, seconds, port, preflight=False):
    mode = 'provider-camera-until-stop' if seconds == 0 else 'provider-camera-manual'
    return [str(binary), mode + ('-preflight' if preflight else ''), str(session),
            str(settings), str(assets), str(warmup),
            *([] if seconds == 0 else [str(seconds)]), str(port), '1', '--inject']


def prepare(*, binary, browser, session, settings, assets, warmup, output, port, seconds,
            name='mouse', python=Path('/usr/bin/python3')):
    if type(seconds) is not int or not 0 <= seconds <= 90:
        raise ValueError('Select explicit seconds: 1..90 finite, or 0 until Stop')
    if type(port) is not int or not 1024 <= port <= 65535:
        raise ValueError('Select an unprivileged loopback port')
    if not re.fullmatch(r'[a-z][a-z0-9-]{0,31}', name):
        raise ValueError('Expected a short lowercase launcher name')
    binary = selected(binary, executable=True)
    browser = selected(browser, executable=True)
    python = selected(python, executable=True)
    session, settings = selected(session), selected(settings)
    assets, warmup = selected(assets, directory=True), selected(warmup)
    output = Path(output)
    if not output.is_absolute():
        raise ValueError('Choose an absolute final bundle location')
    unit_arg(output)
    if output.exists() or output.is_symlink():
        raise ValueError('Preserve the existing bundle; choose a new output')
    # The real selected provider validates configuration/assets without opening
    # a camera, model runtime, listener or input device. It cannot request Start.
    result = subprocess.run(command(binary, session, settings, assets, warmup,
                                    seconds, port, True), capture_output=True,
                            text=True, timeout=30, check=True)
    proof = json.loads(result.stdout)
    expected = {'preflight': 'ok', 'manual_start': True, 'camera_opened': False,
                'system_input': False, 'acquisition_started': False,
                'absolute_pointer': True, 'max_hands': 1, 'recording_configured': True,
                'until_stop': seconds == 0, 'trial_seconds': seconds or None}
    if any(type(proof.get(k)) is not type(v) or proof.get(k) != v for k, v in expected.items()):
        raise ValueError('Selected provider did not confirm manual-only no-acquisition preflight')
    output.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=output.name + '.new-', dir=output.parent))
    try:
        prefix = 'wigglecontrol-preview-' + name
        units = {key: prefix + '-' + key + '.service' for key in ('source', 'worker', 'control')}
        base = f'http://127.0.0.1:{port}'
        common = 'Restart=no\nKillMode=control-group\nTimeoutStopSec=6\nUMask=0077\n'
        argv = command(binary, session, settings, assets, warmup, seconds, port)
        source = ('[Unit]\nDescription=WiggleControl manual mouse source\n'
                  '[Service]\nType=simple\nExecStart=' + ' '.join(map(unit_arg, argv)) + '\n' + common)
        flags = ['--no-first-run', '--no-default-browser-check']
        worker = [str(browser), '--headless=new', '--enable-gpu',
                  '--user-data-dir=' + str(output / 'worker-profile'), *flags,
                  '--disable-background-timer-throttling', '--disable-renderer-backgrounding',
                  '--disable-backgrounding-occluded-windows', base + '/provider/worker']
        control = [str(browser), '--user-data-dir=' + str(output / 'control-profile'),
                   *flags, '--start-maximized', '--app=' + base + '/provider/control']
        (stage / 'units').mkdir()
        (stage / 'units' / units['source']).write_text(source)
        for role, args in [('worker', worker), ('control', control)]:
            text = ('[Unit]\nDescription=WiggleControl ' + role + '\nBindsTo=' + units['source'] +
                    '\nAfter=' + units['source'] + '\n[Service]\nType=simple\nExecStart=' +
                    ' '.join(map(unit_arg, args)) + '\n' + common)
            (stage / 'units' / units[role]).write_text(text)
        # The browser inherits the user's graphical environment through their
        # existing user manager. Do not pin a receiving host's Wayland socket,
        # disable browser sandboxing, or open a remote-debugging port.
        shutil.copyfile(Path(__file__).with_name('mouse_launcher.py'), stage / 'mouse_launcher.py')
        for action in ('open', 'stop'):
            args = [str(python), str(output / 'mouse_launcher.py'), action, '--port', str(port),
                    '--source-unit', units['source'], '--worker-unit', units['worker'],
                    '--control-unit', units['control'], '--require-direct-source']
            (stage / (action + '.sh')).write_text('#!/bin/sh\nset -eu\nexec ' + shlex.join(args) + '\n')
            (stage / (action + '.sh')).chmod(0o700)
        files = {str(p.relative_to(stage)): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in stage.rglob('*') if p.is_file()}
        bindings = {key: {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
                    for key, path in [('binary', binary), ('session', session), ('settings', settings),
                                      ('warmup', warmup)]}
        manifest = {'schema': 'wigglecontrol.preview-launcher/v1', 'units': units,
                    'seconds_after_start': seconds, 'port': port, 'files': files,
                    'bindings': bindings, 'assets': str(assets), 'browser': str(browser),
                    'preflight': proof, 'installed': False, 'started': False}
        (stage / 'bundle.json').write_text(json.dumps(manifest, indent=2) + '\n')
        # Publish only a complete new bundle. No systemd, browser or sensor is
        # invoked; a user must install these units and explicitly open controls.
        if output.exists() or output.is_symlink():
            raise ValueError('Output appeared during preparation; preserved')
        stage.rename(output)
        return manifest
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('binary', 'browser', 'session', 'settings', 'assets', 'warmup', 'output'):
        parser.add_argument('--' + key, type=Path, required=True)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--seconds', type=int, required=True,
                        help='1..90 for a finite manual trial; explicit 0 for use until Stop')
    parser.add_argument('--name', default='mouse')
    args = parser.parse_args()
    manifest = prepare(**vars(args))
    print(json.dumps({'prepared': True, 'installed': False, 'started': False,
                      'units': manifest['units']}))


if __name__ == '__main__':
    main()
