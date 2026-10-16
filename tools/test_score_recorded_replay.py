#!/usr/bin/env python3
"""Continuity scoring must include skips and processing delay without inventing labels."""
import copy
import unittest
from score_recorded_replay import analyze


def fixture():
    captures = [dict(event='capture', sequence=i+1, capture_sequence=i,
                     source_us=1_000_000+i*100_000,
                     recorded_frame=dict(source_us=100_000+i*100_000,
                                         sequence=i, segment=0, offset=i*100, length=100),
                     camera_opened=False, system_input=False) for i in range(3)]
    frames = []
    for i in (0, 2):
        row = copy.deepcopy(captures[i])
        row.update(event='frame', issued_us=row['source_us']+10_000,
                   received_us=row['source_us']+149_000, now_us=row['source_us']+149_500,
                   meaning_us=row['source_us']+150_000,
                   controller=dict(contacts=[dict(id=0)], button_down=False))
        frames.append(row)
    source = dict(event='listening', source_kind='recorded_frame_republication',
                  camera_opened=False, system_input=False)
    end = dict(event='stop', error=None, worker_closed=True, terminal_visible=True,
               captured=3, issued=2, skipped=1, expired=0, camera_opened=False,
               system_input=False, controller=dict(contacts=[], button_down=False))
    labels = dict(schema='wigglecontrol.recorded-appearance/v1',
                  annotation_kind='assistant_visual_review', limits='Fictional appearance only',
                  frames=[dict(index=i, source_us=100_000+i*100_000,
                               appearance='apparent_tip_contact') for i in range(3)])
    return [source, *captures, *frames, end], labels


class RecordedScore(unittest.TestCase):
    def test_coalesced_source_and_actual_meaning_delay_are_included(self):
        events, labels = fixture()
        events.insert(-1, dict(event='intent_transition', now_us=1_149_500,
                               last_source_us=1_000_000, camera_opened=False,
                               system_input=False, controller=events[4]['controller']))
        result = analyze(events, labels)['appearance']
        self.assertEqual(result['wall_time_ms']['apparent_contact_ms'], 200)
        self.assertEqual(result['wall_time_ms']['contact_during_apparent_contact_ms'], 50)
        self.assertEqual(result['longest_reviewed_contact_segment_ms'], 50)
        self.assertEqual(result['samples'][1]['status'], 'coalesced')

    def test_sparse_labels_and_original_dropout_cannot_become_continuous_holds(self):
        events, labels = fixture()
        labels['frames'].pop(1)
        self.assertEqual(analyze(events, labels)['appearance']['scored_adjacent_pairs'], 0)
        events, labels = fixture()
        self.assertEqual(analyze(events, labels, 50)['appearance']['scored_adjacent_pairs'], 0)

    def test_wrong_source_time_synthetic_source_or_live_input_are_rejected(self):
        events, labels = fixture()
        labels['frames'][1]['source_us'] += 1
        with self.assertRaisesRegex(ValueError, 'different original'):
            analyze(events, labels)
        events, labels = fixture()
        events[0]['source_kind'] = 'synthetic_encoded_frame_publish'
        with self.assertRaisesRegex(ValueError, 'recorded source'):
            analyze(events, labels)
        events, labels = fixture()
        events[1]['system_input'] = True
        with self.assertRaisesRegex(ValueError, 'system input'):
            analyze(events, labels)

    def test_final_view_and_original_provenance_are_required(self):
        events, labels = fixture()
        events[-1]['terminal_visible'] = False
        with self.assertRaisesRegex(ValueError, 'termination'):
            analyze(events, labels)
        events, labels = fixture()
        events[4]['recorded_frame']['offset'] += 1
        with self.assertRaisesRegex(ValueError, 'provenance mismatch'):
            analyze(events, labels)


if __name__ == '__main__':
    unittest.main()
