#!/usr/bin/env python3
"""No-device checks for fixture receipts, event framing and the real writer protocol."""
import argparse
from contextlib import redirect_stdout
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, call, patch

import trace_absolute_drag as drag

BINARY = None


class FixtureTests(unittest.TestCase):
    def test_only_original_empty_directory_move_proves_drop(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)/'private'
            m = drag.prepare(root)
            drag.fixture(root, require_ready=True)
            source, target = Path(m['source']), Path(m['target'])
            copied = target/source.name
            copied.mkdir()
            self.assertEqual(drag.drop_receipt(m)['filesystem_drop'], 'not_proven')
            with self.assertRaises(ValueError):
                drag.fixture(root, require_ready=True)
            copied.rmdir()
            source.rename(copied)
            receipt = drag.drop_receipt(m)
            self.assertEqual(receipt['filesystem_drop'], 'moved')
            self.assertIsNone(receipt['desktop_grab_released'])
            with self.assertRaises(FileExistsError):
                drag.prepare(root)

    def test_plan_opens_no_process_or_input_and_nonempty_fixture_refuses_run(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)/'private'
            m = drag.prepare(root)
            with patch.object(drag.subprocess, 'Popen', side_effect=AssertionError('No process')), \
                 patch.object(drag.os, 'open', side_effect=AssertionError('No input descriptor')):
                output = io.StringIO()
                with patch.object(sys, 'argv', [str(Path(drag.__file__)), 'plan', str(root),
                                               '--case', 'release', '--source', '0.2', '0.3',
                                               '--target', '0.7', '0.8']), redirect_stdout(output):
                    drag.main()
                self.assertFalse(json.loads(output.getvalue())['system_input'])
                (Path(m['source'])/'preserve-me').write_text('human data')
                with self.assertRaises(ValueError):
                    drag.run(root, Path('/unavailable'), [], 'release')
                self.assertEqual((Path(m['source'])/'preserve-me').read_text(), 'human data')

    def test_preexisting_named_device_refuses_overlapping_writer(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)/'private'
            drag.prepare(root)
            with patch.object(drag, 'virtual_nodes', return_value={'event999': Path('/fictional')}), \
                 patch.object(drag.subprocess, 'Popen', side_effect=AssertionError('No process')):
                with self.assertRaises(ValueError):
                    drag.run(root, Path('/unavailable'), [], 'release')

    def test_observer_permission_failure_sends_only_cleanup_without_injection(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)/'private'
            drag.prepare(root)
            child = Mock()
            child.stdin.closed = False
            child.returncode = None
            child.poll.side_effect = lambda: child.returncode
            def finish(timeout):
                child.returncode = 0
            child.wait.side_effect = finish
            with patch.object(drag, 'virtual_nodes', side_effect=[{}, {'event999': root/'fictional-sysfs'}]), \
                 patch.object(drag.subprocess, 'Popen', return_value=child), \
                 patch.object(drag.os, 'open', side_effect=PermissionError('denied')):
                result = drag.run(root, Path(__file__), drag.plan('release', (0.2,0.3), (0.7,0.8)), 'release')
            self.assertIn('denied', result['error'])
            self.assertFalse(result['trace_completed'])
            self.assertIn('before cleanup', result['trace_limits'])
            self.assertEqual(result['packets_sent'], 0)
            self.assertEqual(child.stdin.write.call_args_list, [call(b'{"type":"stop"}\n')])


class EventTests(unittest.TestCase):
    def test_fragmented_input_requires_syn_and_reports_dropped_evidence(self):
        output = io.StringIO()
        m = drag.Monitor(-1, output)
        press = drag.EVENT.pack(123, 456, 1, 272, 1)
        sync = drag.EVENT.pack(123, 457, 0, 0, 0)
        m.decode(press[:7], 100)
        self.assertEqual(m.buttons, [])
        m.decode(press[7:], 101)
        self.assertEqual(m.buttons, [])
        m.decode(sync, 102)
        self.assertEqual(m.buttons, [{'value': 1, 'read_us': 101}])
        m.decode(drag.EVENT.pack(123, 458, 0, 3, 0), 103)
        self.assertTrue(m.dropped)
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(rows[0]['kernel_timestamp_clock'], 'unverified_kernel_default')
        for value in [('nan', '0.5'), ('1.1', '0'), ('0', '-0.1')]:
            with self.assertRaises(ValueError):
                drag.point(value)


class ContractTests(unittest.TestCase):
    def test_plans_use_actual_rust_delivery_and_keep_watchdog_independent(self):
        if BINARY is None:
            self.skipTest('Pass --binary to verify the actual Rust delivery consumer')
        for case in drag.CASES:
            with self.subTest(case=case):
                actions = drag.plan(case, (0.2, 0.3), (0.7, 0.8))
                commands, times = [], []
                previous = 1
                for sequence, action in enumerate(actions, 1):
                    now = 1+action['at_ms']*1000
                    for tick in range(previous+20_000, now, 20_000):
                        commands.append({'type': 'tick', 'now_us': tick})
                        times.append(tick)
                    if action['type'] in ('kill', 'eof'):
                        break  # EOF release is not a simulated SIGKILL proof.
                    commands.append({'type': 'packet', 'now_us': now,
                                     'packet': drag.packet(action, sequence, now)})
                    times.append(now)
                    previous = now
                process = subprocess.run([str(BINARY), 'delivery-replay', '2000', '300'],
                                         input=''.join(json.dumps(c)+'\n' for c in commands),
                                         text=True, capture_output=True, check=True)
                rows = [json.loads(line) for line in process.stdout.splitlines()]
                self.assertEqual(len(rows), len(commands)+1)
                self.assertTrue(all(not r['camera_opened'] and not r['system_input'] for r in rows))
                buttons = [(t, e['value']) for t, r in zip(times+[None], rows)
                           for e in r['mouse'] if e['kind'] == 1 and e['code'] == 272]
                self.assertEqual([v for _, v in buttons], [1, 0])
                self.assertEqual(buttons[0][0], 450_001)
                if case == 'watchdog':
                    self.assertEqual(buttons[1][0], 2_100_001)
                elif case not in ('writer-death', 'producer-eof'):
                    self.assertEqual(buttons[1][0], 1_850_001)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path)
    args, remaining = parser.parse_known_args()
    BINARY = args.binary.resolve(strict=True) if args.binary else None
    unittest.main(argv=[__file__]+remaining)
