"""Independent controls for ownership scoring, including stable IDs on the wrong hands."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from summarize_pair_closure import associate, summarize


class OwnershipScoring(unittest.TestCase):
    def test_detector_order_and_uncertain_geometry(self):
        refs = [dict(id='a', center=[100, 100], width=50),
                dict(id='b', center=[300, 100], width=50)]
        self.assertEqual(associate([[298, 100], [102, 100]], refs), {0: 'b', 1: 'a'})
        self.assertEqual(associate([[100, 100]], refs), {0: 'a'})
        self.assertEqual(associate([[10, 10], [600, 100]], refs), {})
        self.assertEqual(associate([[100, 100]], [refs[0], refs[0]]), {})

    def test_stable_counts_and_ids_do_not_hide_a_hand_swap_across_overlap(self):
        def palm(x):
            points = [[x, 100] for _ in range(21)]
            points[5], points[17] = [x-25, 100], [x+25, 100]
            return points
        labels, predictions = [], []
        for index, (a, b) in enumerate([(100, 300), (200, 200), (300, 100)]):
            labels.append(dict(frame=index, source_us=1+index*50_000, skin_in_frame=True,
                separated_skin_depth_ranges=True, reference_hands=[dict(id=name,
                image_px=palm(x), closure_interpolation=1,
                distal_skin=dict(surface_intersection=False, surface_gap_m=0.002))
                for name, x in [('a', a), ('b', b)]]))
            # IDs stay attached to screen positions, thereby swapping physical hands.
            xs = [100, 300] if index != 1 else [200, 200]
            predictions.append(dict(frame=index, source_us=1+index*50_000, ambiguous=False,
                hands=[{}, {}], perception=[dict(id=i+1, landmarks=palm(x)) for i, x in enumerate(xs)],
                contacts=[dict(id=i, x=1-x/640, y=100/480) for i, x in enumerate(xs)]))
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory)/'config.json'
            config.write_text('{}')
            report = Path(directory)/'results.json'
            data = dict(camera_opened=False, system_input=False,
                config_sha256=hashlib.sha256(config.read_bytes()).hexdigest(),
                reference=dict(paired_closure=True, pair_layout='crossing', image_size=[640, 480], frames=labels),
                predictions=predictions)
            report.write_text(json.dumps(data))
            result = summarize(report, config)
            self.assertEqual(result['holds']['near/near']['matching_count'], 3)
            self.assertEqual(result['unscored_track_association_frames'], [1])
            self.assertEqual(result['unscored_contact_association_frames'], [1])
            self.assertEqual(len(result['track_changes']), 2)
            self.assertEqual(len(result['contact_owner_changes']), 2)
            self.assertTrue(all(c['across_gap'] for c in result['contact_owner_changes']))
            data['predictions'][0]['contacts'][1]['id'] = 0
            report.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'Duplicate contact IDs'):
                summarize(report, config)
            data['predictions'][0]['contacts'][1]['id'] = 1
            config.write_text('{"touch_workspace":{"min":{"x":0.2,"y":0.2},"max":{"x":0.8,"y":0.8}}}')
            data['config_sha256'] = hashlib.sha256(config.read_bytes()).hexdigest()
            report.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'full uncalibrated workspace'):
                summarize(report, config)


if __name__ == '__main__':
    unittest.main()
