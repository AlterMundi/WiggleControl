#!/usr/bin/env python3
"""Open explicit mouse controls with an independent, idle detector unit.

Open starts idle controls. Start is an explicit headless acquisition request;
it is intended for a human-clicked desktop/panel action, never autostart.
"""
import argparse
import json
import subprocess
import time
import urllib.request


def validate_source(config, expected_pid=None):
    if not (config['manual_start'] and config['absolute_pointer']
            and config['max_hands'] == 1 and 'prepared' in config):
        raise ValueError('Selected source does not support separate manual mouse controls')
    if expected_pid is not None and (expected_pid <= 0 or
            not str(config.get('run_id', '')).startswith(str(expected_pid) + '-')):
        raise ValueError('Loopback source does not match the selected service process')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('open', 'start', 'stop'))
    parser.add_argument('--port', type=int, default=8876)
    parser.add_argument('--source-unit', default='wigglecontrol-onehand-smooth.service')
    parser.add_argument('--worker-unit', default='wigglecontrol-onehand-mouse-worker.service')
    parser.add_argument('--control-unit', default='wigglecontrol-onehand-trial-browser.service')
    parser.add_argument('--require-direct-source', action='store_true',
                        help='Require the native run ID to match the source unit MainPID')
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error('Expected an unprivileged loopback port')
    selected = [args.source_unit, args.worker_unit, args.control_unit]
    if any(not name.startswith('wigglecontrol-') or not name.endswith('.service')
           or '/' in name for name in selected):
        parser.error('Expected selected WiggleControl user services')
    if args.action == 'stop':
        subprocess.run(['systemctl', '--user', 'stop', *selected], check=True)
        return
    subprocess.run(['systemctl', '--user', 'start', args.source_unit], check=True)
    deadline = time.monotonic() + 10
    while True:
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{args.port}/provider/config', timeout=1) as reply:
                config = json.load(reply)
            expected_pid = None
            if args.require_direct_source:
                process = subprocess.run(['systemctl', '--user', 'show', '--property=MainPID',
                                          '--value', args.source_unit], check=True,
                                         capture_output=True, text=True)
                expected_pid = int(process.stdout.strip())
            validate_source(config, expected_pid)
            break
        except OSError:
            if time.monotonic() >= deadline:
                raise TimeoutError('Selected mouse source is unavailable') from None
            time.sleep(.1)
    subprocess.run(['systemctl', '--user', 'start', args.worker_unit], check=True)
    if args.action == 'start':
        if not config.get('prepared'):
            base = f'http://127.0.0.1:{args.port}'
            request = urllib.request.Request(base + '/provider/prepare',
                data=json.dumps({'run_id': config['run_id']}).encode(),
                headers={'Content-Type': 'application/json', 'Origin': base})
            with urllib.request.urlopen(request, timeout=3) as reply:
                json.load(reply)
        # Repeated ON is idempotent and never opens a control/feedback window.
        return
    # A retained control page is bound to its prior run ID. Reopen it for the
    # current source without touching the independent detector or held input.
    subprocess.run(['systemctl', '--user', 'restart', args.control_unit], check=True)


if __name__ == '__main__':
    main()
