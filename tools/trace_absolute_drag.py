#!/usr/bin/env python3
"""Finite camera-free drag qualification. Only `run` creates/injects input.

Python orchestrates the existing Rust writer and reads its unique new virtual
mouse. It does not implement uinput, control windows, or acquire physical input.
All observations, including failed/unfinished drops, stay in the private workspace.
"""
import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import select
import stat
import struct
import subprocess
import time
import uuid

NAME = 'WiggleControl Absolute Mouse'
EVENT = struct.Struct('@llHHi')
CASES = ('release', 'stop-held', 'watchdog', 'producer-eof', 'writer-death')


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def identity(path):
    s = path.lstat()
    if not stat.S_ISDIR(s.st_mode) or s.st_uid != os.getuid():
        raise ValueError('Expected an owned directory, without a symlink')
    return [s.st_dev, s.st_ino]


def prepare(root, surface=None):
    root.mkdir(mode=0o700)  # Never reuse an existing evidence workspace.
    root = root.resolve()
    if surface is None:
        surface = root / 'surface'
        surface.mkdir(mode=0o700)
    surface = surface.resolve(strict=True)
    token = uuid.uuid4().hex[:12]
    source = surface / ('wiggle-drag-' + token)
    target = surface / ('wiggle-target-' + token)
    source.mkdir(mode=0o700)
    target.mkdir(mode=0o700)
    manifest = {'schema': 'wigglecontrol.drag-fixture/v1', 'source': str(source),
                'target': str(target), 'source_identity': identity(source),
                'target_identity': identity(target)}
    save(root / 'fixture.json', manifest)
    return manifest


def fixture(root, require_ready=False):
    if stat.S_IMODE(root.stat().st_mode) & 0o077 or root.stat().st_uid != os.getuid():
        raise ValueError('Evidence workspace must be owned and private (mode0700)')
    m = json.loads((root / 'fixture.json').read_text())
    if m['schema'] != 'wigglecontrol.drag-fixture/v1':
        raise ValueError('Unknown fixture schema')
    source, target = Path(m['source']), Path(m['target'])
    if identity(target) != m['target_identity']:
        raise ValueError('Target identity changed')
    if require_ready:
        if identity(source) != m['source_identity'] or any(source.iterdir()) or any(target.iterdir()):
            raise ValueError('Run requires the original two empty fixture directories')
    return m


def drop_receipt(m):
    source, target = Path(m['source']), Path(m['target'])
    moved = target / source.name
    target_ok = identity(target) == m['target_identity']
    same = moved.exists() and identity(moved) == m['source_identity']
    outcome = 'moved' if target_ok and same and not source.exists() and not any(moved.iterdir()) else 'not_proven'
    return {'filesystem_drop': outcome, 'source_present': source.exists(),
            'destination_present': moved.exists(), 'destination_is_original': bool(same),
            'desktop_grab_released': None,
            'limits': 'Filesystem identity proves only this move, not compositor/client drag-grab cleanup.'}


def plan(case, source, target):
    actions = []
    def frame(at, point, down=None):
        actions.append({'at_ms': at, 'type': 'frame', 'point': list(point), 'button': down})
    actions.append({'at_ms': 0, 'type': 'transition'})
    for at in range(50, 450, 50):
        frame(at, source)
    frame(450, source, True)
    for at in range(500, 751, 50):
        frame(at, source)
    for at in range(800, 1801, 50):
        f = (at - 800) / 1000
        frame(at, [a + (b-a)*f for a, b in zip(source, target)])
    if case == 'release':
        frame(1850, target, False)
        for at in range(1900, 2851, 50):
            frame(at, target)  # Keep device alive after the explicit release.
        actions.append({'at_ms': 2900, 'type': 'stop'})
    elif case == 'watchdog':
        # Open stdin, alive device, no producer packets for800ms.
        actions.append({'at_ms': 2600, 'type': 'stop'})
    else:
        kind = {'stop-held': 'stop', 'producer-eof': 'eof', 'writer-death': 'kill'}[case]
        actions.append({'at_ms': 1850, 'type': kind})
    return actions


def packet(action, sequence, now):
    kind = action['type']
    if kind == 'stop':
        return {'type': 'stop'}
    common = {'type': kind, 'sequence': sequence, 'issued_us': now, 'mode': 'pointer'}
    if kind == 'transition':
        return dict(common, enabled=True)
    if kind != 'frame':
        raise ValueError('Process kill/EOF is not a writer packet')
    mouse = [] if action['button'] is None else [{'type': 'button', 'down': action['button']}]
    return dict(common, source_us=now, absolute=dict(zip(('x', 'y'), action['point'])),
                mouse=mouse, contacts=[])


def virtual_nodes():
    # Read names/sysfs only; never open a physical input stream.
    matches = {}
    for path in Path('/sys/class/input').glob('event*'):
        try:
            device = (path / 'device').resolve(strict=True)
            if '/devices/virtual/input/' in str(device) and (device / 'name').read_text().strip() == NAME:
                matches[path.name] = device
        except FileNotFoundError:
            pass
    return matches


class Monitor:
    def __init__(self, fd, log):
        self.fd, self.log, self.buffer = fd, log, b''
        self.buttons, self.pending = [], []
        self.events = 0
        self.dropped = self.removed = False

    def decode(self, data, read_us):
        self.buffer += data
        while len(self.buffer) >= EVENT.size:
            sec, usec, kind, code, value = EVENT.unpack(self.buffer[:EVENT.size])
            self.buffer = self.buffer[EVENT.size:]
            row = {'read_monotonic_us': read_us, 'kernel_sec': sec, 'kernel_usec': usec,
                   'kernel_timestamp_clock': 'unverified_kernel_default',
                   'kind': kind, 'code': code, 'value': value}
            self.log.write(json.dumps(row) + '\n')
            self.events += 1
            if kind == 1 and code == 272:
                self.pending.append({'value': value, 'read_us': read_us})
            if kind == 0 and code == 3:  # SYN_DROPPED: do not invent recovery state.
                self.dropped = True
            if kind == 0 and code == 0:
                self.buttons.extend(self.pending)
                self.pending.clear()
        self.log.flush()

    def until(self, deadline):
        while time.monotonic() < deadline:
            timeout = min(0.05, deadline-time.monotonic())
            if self.removed:
                time.sleep(max(0, timeout))
                continue
            if select.select([self.fd], [], [], max(0, timeout))[0]:
                try:
                    data = os.read(self.fd, EVENT.size*128)
                    if not data:
                        self.removed = True
                    else:
                        self.decode(data, time.monotonic_ns()//1000)
                except OSError as e:
                    if e.errno == errno.ENODEV:
                        self.removed = True
                    elif e.errno != errno.EAGAIN:
                        raise


def run(root, binary, actions, case, grant_observer_read=False):
    m = fixture(root, require_ready=True)
    if EVENT.size != 24 or struct.calcsize('P') != 8:
        raise ValueError('This observer requires the64-bit Linux input_event ABI')
    if virtual_nodes():
        raise ValueError('A named WiggleControl mouse already exists; do not overlap writers')
    binary = binary.resolve(strict=True)
    # Exclusive creation prevents overwriting a previous trial's evidence.
    trial = root / 'trial'
    trial.mkdir(mode=0o700)
    save(trial / 'plan.json', {'case': case, 'actions': actions})
    report = {'schema': 'wigglecontrol.absolute-drag-trace/v1', 'case': case,
              'camera_opened': False, 'source_kind': 'scripted_device_packets',
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'system_input_requested': True, 'packets_sent': 0, 'trace_completed': False,
              'kernel_events_observed': None,
              'node_binding': 'unique new virtual named node; no concurrent writer permitted',
              'desktop_grab_released': None}
    child = monitor = fd = device = None
    try:
        with (trial/'writer.stderr').open('x') as stderr, (trial/'kernel.jsonl').open('x') as log, (trial/'packets.jsonl').open('x') as packets:
            child = subprocess.Popen([str(binary), 'controller-absolute-stream', '12', '2000', '300'],
                                     stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=stderr)
            deadline = time.monotonic()+2
            while time.monotonic() < deadline:
                if child.poll() is not None:
                    raise RuntimeError('Writer exited before observation could attach')
                nodes = virtual_nodes()
                if len(nodes) > 1:
                    raise RuntimeError('Concurrent named devices; binding is ambiguous')
                if nodes:
                    name, device = next(iter(nodes.items()))
                    path = Path('/dev/input') / name
                    try:
                        fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
                    except PermissionError:
                        if not grant_observer_read:
                            raise
                        # Reuse the installed named-operator ACL mechanism, only
                        # on this new virtual node. Device removal retires the ACL.
                        report['observer_acl_requested'] = True
                        subprocess.run(['sudo', '-n', 'setfacl', '-m', f'u:{os.getuid()}:r',
                                        '--', str(path)], check=True, capture_output=True, timeout=3)
                        fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
                        report['observer_acl_read_verified'] = True
                    except FileNotFoundError:
                        time.sleep(0.02)
                        continue
                    if not stat.S_ISCHR(os.fstat(fd).st_mode):
                        raise RuntimeError('Expected a character device')
                    report['device_node'] = str(path)
                    report['device_sysfs'] = str(device)
                    break
                time.sleep(0.02)
            if fd is None:
                raise TimeoutError('No uniquely named virtual mouse appeared')
            monitor = Monitor(fd, log)
            monitor.until(time.monotonic()+0.5)  # Allow compositor device discovery.
            started = time.monotonic()
            expected_down = False
            for sequence, action in enumerate(actions, 1):
                due = started+action['at_ms']/1000
                monitor.until(due)
                if time.monotonic()-due > 0.1:
                    raise TimeoutError('Schedule exceeded100ms lateness; refusing catch-up burst')
                if monitor.dropped or monitor.removed:
                    raise RuntimeError('Incomplete/lost event stream before planned termination')
                if action['type'] == 'frame' and action['at_ms'] >= 800 and expected_down:
                    if not monitor.buttons or monitor.buttons[-1]['value'] != 1:
                        raise RuntimeError('Kernel button is not held; refusing drag motion')
                if action['type'] in ('kill', 'eof'):
                    packets.write(json.dumps({'sent_monotonic_us': time.monotonic_ns()//1000,
                                              'action': 'kill_owned_writer' if action['type']=='kill' else 'close_producer_stdin',
                                              'pid': child.pid})+'\n')
                    if action['type'] == 'kill':
                        child.kill()
                    else:
                        child.stdin.close()
                else:
                    now = time.monotonic_ns()//1000
                    message = packet(action, sequence, now)
                    child.stdin.write((json.dumps(message)+'\n').encode())
                    child.stdin.flush()
                    report['packets_sent'] += 1
                    if action['type'] == 'frame' and action['button'] is not None:
                        expected_down = action['button']
                    packets.write(json.dumps({'sent_monotonic_us': now, 'packet': message,
                                              'planned_offset_ms': action['at_ms'],
                                              'send_lateness_us': max(0, round((time.monotonic()-due)*1e6))})+'\n')
                packets.flush()
            monitor.until(time.monotonic()+0.5)
            child.wait(timeout=2)
            if monitor.dropped or monitor.buffer or monitor.pending:
                raise RuntimeError('Final kernel trace is incomplete or contains unsynchronized buttons')
            report['trace_completed'] = True
    except Exception as e:
        report['error'] = str(e)
        report['trace_limits'] = 'Error ended tracing before cleanup; cleanup events are not qualified.'
    finally:
        if child is not None:
            if child.poll() is None:
                try:
                    if child.stdin and not child.stdin.closed:
                        child.stdin.write(b'{"type":"stop"}\n')
                        child.stdin.flush()
                    child.wait(timeout=2)
                except (OSError, subprocess.TimeoutExpired):
                    try:
                        child.kill()
                        child.wait(timeout=2)
                    except (OSError, subprocess.TimeoutExpired) as e:
                        report['cleanup_error'] = str(e)
            if child.stdin:
                try:
                    child.stdin.close()
                except OSError:
                    pass
            report['writer_exit_code'] = child.returncode
            report['writer_reaped'] = child.returncode is not None
        if monitor is not None:
            report.update(kernel_buttons=monitor.buttons, syn_dropped=monitor.dropped,
                          kernel_events_observed=monitor.events,
                          partial_event_bytes=len(monitor.buffer),
                          unsynchronized_button_events=monitor.pending)
        if fd is not None:
            os.close(fd)
        report['device_sysfs_removed'] = None if device is None else not device.exists()
        try:
            report.update(drop_receipt(m))
        except (OSError, ValueError) as e:
            report['filesystem_error'] = str(e)
        save(trial/'result.json', report)
    return report


def point(values):
    p = tuple(float(v) for v in values)
    if len(p) != 2 or not all(0 <= v <= 1 for v in p):
        raise ValueError('Points require two finite normalized screen coordinates0..1')
    return p


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    prep = sub.add_parser('prepare')
    prep.add_argument('workspace', type=Path)
    prep.add_argument('--surface-directory', type=Path)
    inspect = sub.add_parser('inspect')
    inspect.add_argument('workspace', type=Path)
    for name in ('plan', 'run'):
        p = sub.add_parser(name)
        p.add_argument('workspace', type=Path)
        p.add_argument('--case', choices=CASES, required=True)
        p.add_argument('--source', nargs=2, required=True)
        p.add_argument('--target', nargs=2, required=True)
        if name == 'run':
            p.add_argument('--binary', type=Path, required=True)
            p.add_argument('--grant-observer-read', action='store_true',
                           help='If needed, sudo -n setfacl grants only this user read access to the new virtual node')
    args = parser.parse_args()
    if args.command == 'prepare':
        result = prepare(args.workspace, args.surface_directory)
    elif args.command == 'inspect':
        result = drop_receipt(fixture(args.workspace))
    else:
        actions = plan(args.case, point(args.source), point(args.target))
        if args.command == 'plan':
            fixture(args.workspace, require_ready=True)
            result = {'case': args.case, 'actions': actions, 'camera_opened': False,
                      'system_input': False, 'limits': 'Plan only; no process or device opened.'}
        else:
            result = run(args.workspace, args.binary, actions, args.case, args.grant_observer_read)
    print(json.dumps(result, indent=2))
    if result.get('error'):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
