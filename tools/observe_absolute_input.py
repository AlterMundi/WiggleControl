#!/usr/bin/env python3
"""Finite read-only trace of one explicitly selected native virtual mouse writer.

This observer never grabs a device, opens physical streams, starts capture/models
or injects input. Startup before attachment is always reported as unobserved.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import time

from trace_absolute_drag import Monitor, virtual_nodes


WRITERS = {'controller-absolute-stream', 'controller-absolute-stream-until-stop'}
PROVIDERS = {'provider-camera', 'provider-camera-manual', 'provider-camera-until-stop'}


def process(pid):
    try:
        root = Path('/proc')/str(pid)
        if root.stat().st_uid != os.getuid():
            raise ValueError('Selected process is not owned by this operator')
        fields = (root/'stat').read_text().rsplit(')',1)[1].split()
        if fields[0] == 'Z':
            return None
        return {'pid':pid,'start_ticks':fields[19], 'exe':(root/'exe').resolve(strict=True),
                'args':(root/'cmdline').read_bytes().rstrip(b'\0').decode().split('\0')}
    except (FileNotFoundError, ProcessLookupError):
        return None


def validate_process(info, binary, digest, commands):
    if not info or len(info['args']) < 2 or info['args'][1] not in commands:
        raise ValueError('Selected process is not a supported native producer/writer')
    if info['exe'] != binary or hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
        raise ValueError('Selected native executable differs from the expected artifact')
    return info


def same_process(before):
    current = process(before['pid'])
    return current is not None and current['start_ticks'] == before['start_ticks'] and current['exe']==before['exe']


def writer_child(parent, binary, digest):
    try:
        children = (Path('/proc')/str(parent['pid'])/'task'/str(parent['pid'])/'children').read_text().split()
    except FileNotFoundError:
        return None
    found=[]
    for pid in children:
        info=process(int(pid))
        if info and len(info['args'])>1 and info['args'][1] in WRITERS:
            found.append(validate_process(info,binary,digest,WRITERS))
    if len(found)>1:
        raise ValueError('More than one eligible child writer; no event node selected')
    return found[0] if found else None


def save(root, name, value):
    temporary=root/(name+'.tmp')
    temporary.write_text(json.dumps(value,indent=2,default=str)+'\n')
    temporary.replace(root/name)


def observe(root, binary, digest, producer_pid=None, writer_pid=None,
            wait_seconds=600, seconds=120, grant=False):
    if (producer_pid is None)==(writer_pid is None):
        raise ValueError('Select exactly one producer PID or writer PID')
    if not 1<=wait_seconds<=3600 or not 1<=seconds<=600:
        raise ValueError('Wait must be1..3600s and trace1..600s')
    binary=binary.resolve(strict=True)
    selected=validate_process(process(producer_pid or writer_pid),binary,digest,
                              PROVIDERS if producer_pid else WRITERS)
    root.mkdir(mode=0o700)  # Never overwrite or append to another run.
    if root.stat().st_uid!=os.getuid() or stat.S_IMODE(root.stat().st_mode)&0o077:
        raise ValueError('Private owned output directory required')
    initial=virtual_nodes()
    if producer_pid and initial:
        raise ValueError('Named device already present before this producer observer was armed')
    state={'schema':'wigglecontrol.virtual-input-observation/v1',
           'selected_pid':selected['pid'],'selected_start_ticks':selected['start_ticks'],
           'binary_sha256':digest,'armed_monotonic_us':time.monotonic_ns()//1000,
           'phase':'armed_waiting_writer','camera_opened':False,'input_injected':False,
           'initial_button_state':'unknown','startup_coverage':'unobserved until event-node attachment',
           'kernel_clock':'unverified_kernel_default','physical_event_streams_opened':0,
           'node_binding':'unique named virtual node with selected native PID/artifact; no concurrent same-name writer permitted'}
    save(root,'state.json',state)
    stopped=False
    def stop(*_):
        nonlocal stopped
        stopped=True
    previous={sig:signal.signal(sig,stop) for sig in (signal.SIGTERM,signal.SIGINT)}
    fd=None;monitor=None;writer=None;device=None
    try:
        until=time.monotonic()+wait_seconds
        while not stopped and time.monotonic()<until:
            if not same_process(selected):
                state['end_reason']='selected_process_ended_before_attachment';break
            writer=writer_child(selected,binary,digest) if producer_pid else selected
            nodes=virtual_nodes()
            if len(nodes)>1:
                raise ValueError('Multiple named virtual mice; binding ambiguous')
            if writer and len(nodes)==1:
                name,device=next(iter(nodes.items()))
                path=Path('/dev/input')/name
                try:
                    fd=os.open(path,os.O_RDONLY|os.O_NONBLOCK)
                except PermissionError:
                    if not grant:
                        raise
                    state['observer_acl_requested']=True
                    subprocess.run(['sudo','-n','setfacl','-m',f'u:{os.getuid()}:r','--',str(path)],
                                   check=True,capture_output=True,timeout=3)
                    fd=os.open(path,os.O_RDONLY|os.O_NONBLOCK)
                    state['observer_acl_read_verified']=True
                except FileNotFoundError:
                    time.sleep(.02);continue
                if not stat.S_ISCHR(os.fstat(fd).st_mode) or virtual_nodes()!=nodes or not same_process(writer):
                    raise ValueError('Selected new virtual node or writer changed during attachment')
                state.update(phase='observing',writer_pid=writer['pid'],writer_start_ticks=writer['start_ticks'],
                             node=str(path),sysfs=str(device),attached_monotonic_us=time.monotonic_ns()//1000)
                save(root,'state.json',state);break
            time.sleep(.02)
        if fd is None:
            state.setdefault('end_reason','observer_cancelled_before_attachment' if stopped else 'wait_limit')
        else:
            with (root/'events.jsonl').open('w',buffering=1) as log:
                monitor=Monitor(fd,log)
                until=time.monotonic()+seconds
                published=time.monotonic()
                while not stopped and time.monotonic()<until:
                    monitor.until(min(until,time.monotonic()+.1))
                    if time.monotonic()-published>=.25:
                        state.update(kernel_events=monitor.events,kernel_buttons=monitor.buttons,
                                     observation_update_monotonic_us=time.monotonic_ns()//1000)
                        save(root,'state.json',state);published=time.monotonic()
                    if monitor.removed or not device.exists():
                        state['end_reason']='device_removed';break
                    nodes=virtual_nodes()
                    if len(nodes)!=1 or next(iter(nodes.values()))!=device:
                        raise ValueError('Virtual mouse binding changed while observing')
                    if not same_process(writer):
                        state['end_reason']='writer_ended';break
                state.setdefault('end_reason','observer_cancelled' if stopped else 'trace_limit')
                # Drain final release/removal after producer/writer termination or owned cancellation.
                monitor.until(time.monotonic()+.5)
                log.flush();os.fsync(log.fileno())
    except Exception as error:
        state.update(error=str(error),end_reason='observation_error')
    finally:
        if fd is not None:
            os.close(fd)
        for sig,handler in previous.items():
            signal.signal(sig,handler)
        state.update(phase='ended',ended_monotonic_us=time.monotonic_ns()//1000,
                     device_removed=None if device is None else not device.exists())
        if monitor is not None:
            state.update(kernel_events=monitor.events,kernel_buttons=monitor.buttons,
                         syn_dropped=monitor.dropped,partial_event_bytes=len(monitor.buffer),
                         unsynchronized_buttons=monitor.pending)
        state['attached_trace_complete']=bool(monitor is not None and not state.get('error')
            and state.get('device_removed') and not monitor.dropped and not monitor.buffer and not monitor.pending)
        save(root,'state.json',state)
    return state


def main():
    os.umask(0o077)
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--sha256',required=True)
    group=p.add_mutually_exclusive_group(required=True)
    group.add_argument('--producer-pid',type=int)
    group.add_argument('--writer-pid',type=int)
    p.add_argument('--wait-seconds',type=int,default=600)
    p.add_argument('--seconds',type=int,default=120)
    p.add_argument('--grant-observer-read',action='store_true')
    args=p.parse_args()
    result=observe(args.output,args.binary,args.sha256,args.producer_pid,args.writer_pid,
                   args.wait_seconds,args.seconds,args.grant_observer_read)
    print(json.dumps(result,default=str))
    return int(bool(result.get('error')))


if __name__=='__main__':
    try:
        raise SystemExit(main())
    except (OSError,ValueError) as error:
        print(f'{type(error).__name__}: {error}',file=__import__('sys').stderr)
        raise SystemExit(1)
