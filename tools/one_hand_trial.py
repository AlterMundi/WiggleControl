#!/usr/bin/env python3
"""Explicit manual-Start finite one-hand trial with independent virtual-input evidence.

Use only after receiving preparation; physical Start needs fresh human readiness.
All inference, capture, recording and input remain in the selected Rust provider.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import uuid
import urllib.request


FIELDS={'schema','binary','binary_sha256','session','session_sha256','settings',
        'settings_sha256','assets','warmup','port','seconds','grant_observer_read'}


def standby_seconds(spec):
    # Manual idle waiting is separate from the acquisition/attached-trace clocks.
    seconds=spec.get('standby_seconds',3600)
    if type(seconds) is not int or not 30<=seconds<=3600:
        raise ValueError('Manual standby must be30..3600 seconds')
    return seconds


def load(path):
    if path.stat().st_size>16384:
        raise ValueError('Trial specification exceeds16KiB')
    spec=json.loads(path.read_text())
    if set(spec)-{'standby_seconds'}!=FIELDS or spec['schema']!='wigglecontrol.one-hand-trial/v1':
        raise ValueError('Unexpected trial specification')
    if not isinstance(spec['seconds'],int) or not 1<=spec['seconds']<=90:
        raise ValueError('Finite trial must be1..90s after manual Start')
    if not isinstance(spec['port'],int) or not 1024<=spec['port']<=65535:
        raise ValueError('Expected an unprivileged receiving port')
    if not isinstance(spec['grant_observer_read'],bool):
        raise ValueError('Observer ACL choice must be explicit')
    standby_seconds(spec)
    for field in ('binary','session','settings'):
        selected=Path(spec[field])
        if not selected.is_absolute() or not selected.is_file():
            raise ValueError('Expected existing absolute selected artifact/config paths')
        if hashlib.sha256(selected.read_bytes()).hexdigest()!=spec[field+'_sha256']:
            raise ValueError('Selected '+field+' differs from the prepared trial')
    for field in ('assets','warmup'):
        if not Path(spec[field]).is_absolute() or not Path(spec[field]).exists():
            raise ValueError('Existing receiving assets and synthetic warmup required')
    return spec


def command(spec, preflight=False):
    return [spec['binary'],'provider-camera-manual'+('-preflight' if preflight else ''),
            spec['session'],spec['settings'],spec['assets'],spec['warmup'],str(spec['seconds']),
            str(spec['port']),'1','--inject']


def preflight(spec):
    run=subprocess.run(command(spec,True),text=True,capture_output=True,timeout=20)
    if run.returncode:
        raise RuntimeError('Manual finite provider preflight failed: '+run.stderr)
    proof=json.loads(run.stdout)
    if not (proof.get('preflight')=='ok' and proof.get('manual_start') is True and
            proof.get('trial_seconds')==spec['seconds'] and proof.get('until_stop') is False and
            proof.get('camera_opened') is False and proof.get('system_input') is False and
            proof.get('acquisition_started') is False and proof.get('recording_configured') is True and
            proof.get('absolute_pointer') is True and proof.get('max_hands')==1):
        raise ValueError('Selected provider did not confirm the manual finite no-acquisition contract')
    return proof


def save(root,value):
    temporary=root/'run.json.tmp'
    temporary.write_text(json.dumps(value,indent=2)+'\n');temporary.replace(root/'run.json')


def run(spec, logs):
    from trace_absolute_drag import virtual_nodes
    if virtual_nodes():
        raise ValueError('A named virtual mouse is already present; no trial started')
    proof=preflight(spec)
    logs.mkdir(mode=0o700,parents=True,exist_ok=True)
    if logs.stat().st_uid!=os.getuid() or logs.stat().st_mode&0o077:
        raise ValueError('Private owned log directory required')
    root=logs/(datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:10])
    root.mkdir(mode=0o700)
    (root/'specification.json').write_text(json.dumps(spec,indent=2)+'\n')
    (root/'preflight.json').write_text(json.dumps(proof,indent=2)+'\n')
    state={'schema':'wigglecontrol.one-hand-trial-run/v1','phase':'starting',
           'binary_sha256':spec['binary_sha256'],'seconds_after_start':spec['seconds'],
           'root':str(root),'startup_camera_opened':False,'startup_input_opened':False,'standby_seconds':standby_seconds(spec)}
    provider=observer=None;stopped=False;sent_stop=None
    def stop(*_):
        nonlocal stopped
        stopped=True
    previous={sig:signal.signal(sig,stop) for sig in (signal.SIGTERM,signal.SIGINT)}
    try:
        with (root/'provider.jsonl').open('w') as output,(root/'provider.stderr').open('w') as error, \
             (root/'observer.stdout').open('w') as observer_output,(root/'observer.stderr').open('w') as observer_error:
            provider=subprocess.Popen(command(spec),stdout=output,stderr=error)
            state['provider_pid']=provider.pid
            args=[sys.executable,'-B',str(Path(__file__).resolve().with_name('observe_absolute_input.py')),
                  '--producer-pid',str(provider.pid),'--binary',spec['binary'],'--sha256',spec['binary_sha256'],
                  '--output',str(root/'kernel'),'--wait-seconds',str(standby_seconds(spec)),'--seconds',str(spec['seconds']+15)]
            if spec['grant_observer_read']:
                args.append('--grant-observer-read')
            observer=subprocess.Popen(args,stdout=observer_output,stderr=observer_error)
            state['observer_pid']=observer.pid;save(root,state)
            until=time.monotonic()+10
            while True:
                if stopped or provider.poll() is not None or observer.poll() is not None:
                    raise RuntimeError('Selected trial or observer ended before standby was confirmed')
                try:
                    with urllib.request.urlopen(f'http://127.0.0.1:{spec["port"]}/provider/config',timeout=.5) as reply:
                        config=json.load(reply)
                    armed=json.loads((root/'kernel/state.json').read_text())
                    if (config.get('run_id','').startswith(str(provider.pid)+'-') and
                        config.get('manual_start') is True and config.get('trial_seconds')==spec['seconds'] and
                        config.get('camera_opened') is False and config.get('system_input') is False and
                        armed.get('phase')=='armed_waiting_writer'):
                        state.update(phase='standby',provider_run_id=config['run_id'],observer_armed=True)
                        save(root,state);break
                except (OSError,ValueError):
                    pass
                if time.monotonic()>until:
                    raise TimeoutError('Owned source/observer standby not confirmed')
                time.sleep(.05)
            while provider.poll() is None:
                if state['phase']=='standby':
                    try:
                        current=json.loads((root/'kernel/state.json').read_text())
                        if current.get('phase')=='observing':
                            state.update(phase='input_observer_attached',observer_attached_monotonic_us=current['attached_monotonic_us'])
                            save(root,state)
                    except (OSError,ValueError):
                        pass
                if observer.poll() is not None and not state.get('observer_end_seen'):
                    state['observer_end_seen']=True
                    try:
                        observed=json.loads((root/'kernel/state.json').read_text())
                    except (OSError,ValueError):
                        observed={}
                    normal=(observer.returncode==0 and observed.get('device_removed') is True
                            and observed.get('end_reason') in ('device_removed','writer_ended'))
                    state['observer_ended_after_device_removal']=normal
                    if not normal:
                        state['observer_ended_before_provider']=True
                        stopped=True
                    save(root,state)
                if stopped and sent_stop is None:
                    provider.terminate();sent_stop=time.monotonic()
                if sent_stop is not None and time.monotonic()-sent_stop>6:
                    provider.kill();state['provider_forced_kill']=True
                time.sleep(.05)
            state['provider_exit_code']=provider.returncode
    except Exception as error:
        state['error']=str(error)
    finally:
        if provider is not None and provider.poll() is None:
            provider.terminate()
            try:provider.wait(timeout=6)
            except subprocess.TimeoutExpired:provider.kill();provider.wait(timeout=2)
        if provider is not None:
            state.update(provider_exit_code=provider.returncode,provider_reaped=provider.poll() is not None)
        if observer is not None:
            try:observer.wait(timeout=2)
            except subprocess.TimeoutExpired:
                observer.terminate()
                try:observer.wait(timeout=2)
                except subprocess.TimeoutExpired:observer.kill();observer.wait(timeout=2)
            state.update(observer_exit_code=observer.returncode,observer_reaped=observer.poll() is not None)
        for sig,handler in previous.items():signal.signal(sig,handler)
        state.update(phase='ended',ended_monotonic_us=time.monotonic_ns()//1000)
        save(root,state)
    print(json.dumps(state))
    return int(bool(state.get('error')) or state.get('provider_exit_code')!=0 or state.get('observer_exit_code')!=0)


def main():
    os.umask(0o077)
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--config',type=Path,required=True)
    p.add_argument('--logs',type=Path)
    p.add_argument('--preflight',action='store_true')
    args=p.parse_args();spec=load(args.config)
    if args.preflight:
        print(json.dumps(preflight(spec)));return 0
    if args.logs is None:p.error('--logs is required for a live manual-Start session')
    return run(spec,args.logs)


if __name__=='__main__':
    try:raise SystemExit(main())
    except (OSError,ValueError,RuntimeError,subprocess.SubprocessError) as error:
        print(f'{type(error).__name__}: {error}',file=sys.stderr)
        raise SystemExit(1)
