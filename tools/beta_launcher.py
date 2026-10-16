#!/usr/bin/env python3
"""Explicit desktop entry for the receiving one-hand beta's two owned user units."""
import json
import subprocess
import sys
import time
import urllib.request

SOURCE_UNIT = 'wigglecontrol-beta1.service'
BROWSER_UNIT = 'wigglecontrol-beta1-browser.service'
BASE = 'http://127.0.0.1:8875'

def unit(action, name):
    subprocess.run(['systemctl', '--user', action, name], check=True)

def main():
    action = sys.argv[1] if len(sys.argv) == 2 else 'open'
    if action == 'stop':
        unit('stop', SOURCE_UNIT)
        unit('stop', BROWSER_UNIT)
        return
    if action != 'open':
        raise ValueError('Expected open or stop')
    unit('start', SOURCE_UNIT)
    deadline = time.monotonic() + 10
    while True:
        try:
            with urllib.request.urlopen(BASE + '/provider/config', timeout=1) as reply:
                config = json.load(reply)
            if not (config['manual_start'] and config['absolute_pointer'] and
                    config['max_hands'] == 1 and config['presence_floor'] == .5):
                raise ValueError('Receiving endpoint is not the selected manual one-hand beta')
            break
        except OSError:
            if time.monotonic() >= deadline:
                raise TimeoutError('Beta server not ready; inspect its existing unit')
            time.sleep(.1)
    # This opens an idle page. Only its human Start click prepares/acquires anything.
    unit('start', BROWSER_UNIT)
    try:
        with urllib.request.urlopen('http://127.0.0.1:9223/json/list', timeout=2) as reply:
            tabs = json.load(reply)
        for tab in tabs:
            if tab.get('type') == 'page' and tab.get('url') == BASE + '/provider/view':
                with urllib.request.urlopen('http://127.0.0.1:9223/json/activate/' + tab['id'], timeout=2):
                    pass
                break
    except OSError:
        pass  # Browser startup is asynchronous; its own unit retains failures.

if __name__ == '__main__':
    main()
