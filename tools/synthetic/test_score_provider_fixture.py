import copy
import unittest

from score_provider_fixture import stale_releases, validate, wall_time


def journal():
    common = dict(camera_opened=False, system_input=False)
    return [
        dict(common, event='capture', sequence=1, capture_sequence=0, source_us=100),
        dict(common, event='capture', sequence=2, capture_sequence=1, source_us=200),
        dict(common, event='frame', sequence=1, capture_sequence=0, source_us=100,
             issued_us=110, received_us=120, now_us=130, meaning_us=140,
             controller=dict(contacts=[dict(id=0)], button_down=False)),
        dict(common, event='stop', error=None, captured=2, issued=1, expired=0, skipped=1,
             worker_closed=True, controller=dict(contacts=[], button_down=False)),
    ]


class JournalEvidenceTests(unittest.TestCase):
    def test_accepts_ordered_capture_bound_result(self):
        rows, terminal = validate(journal())
        self.assertEqual((len(rows), terminal['captured']), (1, 2))

    def test_rejects_unacknowledged_close_or_held_terminal(self):
        for change in [dict(worker_closed=False), dict(error='timeout'),
                       dict(controller=dict(contacts=[dict(id=0)], button_down=False))]:
            events = journal()
            events[-1].update(change)
            with self.assertRaises(ValueError):
                validate(events)

    def test_rejects_substituted_capture_or_non_host_clock(self):
        for change in [dict(capture_sequence=1), dict(source_us=99), dict(issued_us=121),
                       dict(meaning_us=119)]:
            events = journal()
            events[2].update(change)
            with self.assertRaises(ValueError):
                validate(events)

    def test_rejects_duplicate_or_hidden_dropped_result(self):
        events = journal()
        events.insert(3, copy.deepcopy(events[2]))
        with self.assertRaises(ValueError):
            validate(events)
        events = journal()
        events[-1]['skipped'] = 0
        with self.assertRaises(ValueError):
            validate(events)
        events = journal()
        events[1]['source_us'] = 99
        with self.assertRaises(ValueError):
            validate(events)

    def test_skipped_images_and_latency_remain_in_wall_time_denominator(self):
        events = journal()
        near = dict(closure_interpolation=1., distal_skin=dict(surface_gap_m=.002))
        result = wall_time(events, dict(frames=[near, near]))
        self.assertEqual(result['near_hold_ms'], .1)
        self.assertEqual(result['matching_near_ms'], .06)

    def test_release_requires_latest_accepted_input_held(self):
        events = journal()
        release = dict(event='intent_transition', now_us=400, last_source_us=100,
                       controller=dict(contacts=[], button_down=False, reason='stale_input'))
        events.insert(-1, release)
        self.assertEqual(stale_releases(events)[0]['source_age_ms'], .3)
        events[2]['controller']['contacts'] = []
        self.assertEqual(stale_releases(events), [])


if __name__ == '__main__':
    unittest.main()
