#!/usr/bin/env python3
"""No-camera checks for selected-process binding, partial evidence and trial admission."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import observe_absolute_input as observer
import one_hand_trial as trial


class TrialTests(unittest.TestCase):
    def test_real_process_cookie_and_executable_validation(self):
        child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(10)'])
        try:
            selected=observer.process(child.pid)
            self.assertTrue(observer.same_process(selected))
            binary=Path(sys.executable).resolve();digest=hashlib.sha256(binary.read_bytes()).hexdigest()
            observer.validate_process(selected,binary,digest,{'-c'})
            with self.assertRaisesRegex(ValueError,'supported native'):
                observer.validate_process(selected,binary,digest,observer.WRITERS)
            with self.assertRaisesRegex(ValueError,'artifact'):
                observer.validate_process(selected,binary,'0'*64,{'-c'})
            stale=dict(selected,start_ticks=str(int(selected['start_ticks'])+1))
            self.assertFalse(observer.same_process(stale))
        finally:
            child.terminate();child.wait()
        self.assertIsNone(observer.process(child.pid))

    def test_specification_tamper_and_finite_duration_refuse_before_preflight(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);blob=root/'selected';blob.write_bytes(b'prepared artifact')
            digest=hashlib.sha256(blob.read_bytes()).hexdigest()
            spec={'schema':'wigglecontrol.one-hand-trial/v1','binary':str(blob),'binary_sha256':digest,
                  'session':str(blob),'session_sha256':digest,'settings':str(blob),'settings_sha256':digest,
                  'assets':str(root),'warmup':str(blob),'port':8876,'seconds':45,'grant_observer_read':False}
            path=root/'spec.json';path.write_text(json.dumps(spec))
            self.assertEqual(trial.load(path)['seconds'],45)
            spec['seconds']=0;path.write_text(json.dumps(spec))
            with self.assertRaisesRegex(ValueError,'Finite trial'):
                trial.load(path)
            spec['seconds']=45;path.write_text(json.dumps(spec));blob.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError,'differs'):
                trial.load(path)

    def test_automatic_or_unbounded_native_preflight_is_not_accepted_as_manual_finite(self):
        proof={'preflight':'ok','manual_start':False,'trial_seconds':45,'until_stop':False,
               'camera_opened':False,'system_input':False,'acquisition_started':False,
               'recording_configured':True,'absolute_pointer':True,'max_hands':1}
        spec={'binary':'/selected','session':'/session','settings':'/settings','assets':'/assets',
              'warmup':'/warmup','seconds':45,'port':8876}
        result=subprocess.CompletedProcess([],0,json.dumps(proof),'')
        with patch.object(trial.subprocess,'run',return_value=result) as call:
            with self.assertRaisesRegex(ValueError,'manual finite'):
                trial.preflight(spec)
            self.assertEqual(call.call_args.args[0][1],'provider-camera-manual-preflight')
        proof.update(manual_start=True,until_stop=True,trial_seconds=None)
        with patch.object(trial.subprocess,'run',return_value=subprocess.CompletedProcess([],0,json.dumps(proof),'')):
            with self.assertRaisesRegex(ValueError,'manual finite'):
                trial.preflight(spec)


class ObserverTests(unittest.TestCase):
    def selected(self,root,command):
        binary=root/'native';binary.write_bytes(b'selected native bytes')
        digest=hashlib.sha256(binary.read_bytes()).hexdigest()
        return binary,digest,{'pid':1234,'start_ticks':'123','exe':binary,'args':[str(binary),command]}

    def test_preexisting_device_refuses_producer_scope_without_opening(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);binary,digest,info=self.selected(root,'provider-camera-manual')
            with patch.object(observer,'process',return_value=info), \
                 patch.object(observer,'virtual_nodes',return_value={'event9':root/'other'}), \
                 patch.object(observer.os,'open',side_effect=AssertionError('No event may open')):
                with self.assertRaisesRegex(ValueError,'already present'):
                    observer.observe(root/'trace',binary,digest,producer_pid=1234,wait_seconds=1,seconds=1)

    def test_ended_producer_has_explicit_unobserved_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);binary,digest,info=self.selected(root,'provider-camera-manual')
            with patch.object(observer,'process',return_value=info), \
                 patch.object(observer,'same_process',return_value=False), \
                 patch.object(observer,'virtual_nodes',return_value={}), \
                 patch.object(observer.os,'open',side_effect=AssertionError('No event may open')):
                result=observer.observe(root/'trace',binary,digest,producer_pid=1234,wait_seconds=1,seconds=1)
            self.assertEqual(result['end_reason'],'selected_process_ended_before_attachment')
            self.assertFalse(result['attached_trace_complete'])
            self.assertEqual(result['initial_button_state'],'unknown')

    def test_permission_failure_does_not_read_or_invent_events(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);binary,digest,info=self.selected(root,'controller-absolute-stream')
            with patch.object(observer,'process',return_value=info), \
                 patch.object(observer,'same_process',return_value=True), \
                 patch.object(observer,'virtual_nodes',return_value={'event9':root/'new-virtual'}), \
                 patch.object(observer.os,'open',side_effect=PermissionError('denied')), \
                 patch.object(observer,'Monitor',side_effect=AssertionError('No read')):
                result=observer.observe(root/'trace',binary,digest,writer_pid=1234,wait_seconds=1,seconds=1)
            self.assertEqual(result['end_reason'],'observation_error')
            self.assertIn('denied',result['error'])
            self.assertFalse(result['attached_trace_complete'])
            self.assertNotIn('kernel_buttons',result)

    def test_node_replacement_after_open_closes_fd_without_reading(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);binary,digest,info=self.selected(root,'controller-absolute-stream')
            node={'event9':root/'new-virtual'}
            fd=os.open('/dev/null',os.O_RDONLY)
            with patch.object(observer,'process',return_value=info), \
                 patch.object(observer,'same_process',return_value=True), \
                 patch.object(observer,'virtual_nodes',side_effect=[node,node,{}]), \
                 patch.object(observer.os,'open',return_value=fd), \
                 patch.object(observer,'Monitor',side_effect=AssertionError('No replaced-node read')):
                result=observer.observe(root/'trace',binary,digest,writer_pid=1234,wait_seconds=1,seconds=1)
            self.assertIn('changed during attachment',result['error'])
            self.assertFalse(result['attached_trace_complete'])
            with self.assertRaises(OSError):os.fstat(fd)


if __name__=='__main__':unittest.main()
