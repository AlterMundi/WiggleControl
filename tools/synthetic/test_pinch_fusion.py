import copy
import unittest
from replay_pinch_fusion import attach_secondary
import test_tasks_metric as fixtures


class FusionAttachment(unittest.TestCase):
    def test_secondary_has_native_owner_quality_but_provider_metric_and_exact_frame(self):
        native,tasks=fixtures.TasksMetric().fixture()
        native['hands'][0].update(engaged=True,pinch_eligible=True)
        # The fixed native projected cue differs from the provider cue.
        native['perception']=copy.deepcopy(tasks['hands'])
        native['perception'][0]['id']=42
        before=copy.deepcopy(native)
        fused=attach_secondary([native],[tasks],.38)[0]['hands'][0]
        self.assertEqual(fused['id'],42)
        self.assertEqual(fused['pinch_distance_m'],.08)
        self.assertEqual(fused['secondary_pinch'],dict(source_id=2,source_us=1,
            confidence=.91,eligible=True,distance_m=.02,onset_allowed=True))
        self.assertEqual(native,before)
        tasks['hands']=[]
        self.assertNotIn('secondary_pinch',attach_secondary([native],[tasks],.38)[0]['hands'][0])

    def test_already_fused_or_mismatched_source_is_not_silently_relabelled(self):
        native,tasks=fixtures.TasksMetric().fixture()
        native['hands'][0]['secondary_pinch']={}
        with self.assertRaisesRegex(ValueError,'single-hypothesis'):
            attach_secondary([native],[tasks],.38)
        native,tasks=fixtures.TasksMetric().fixture();tasks['source_us']=2
        with self.assertRaisesRegex(ValueError,'differs'):
            attach_secondary([native],[tasks],.38)


if __name__=='__main__':unittest.main()
