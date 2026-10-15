import copy
import math
import unittest

from compare_tasks_metric import substitute


class TasksMetric(unittest.TestCase):
    def fixture(self):
        world = [[0,0,0] for _ in range(21)]
        world[8] = [0.02,0,0]
        pixels = [[0,0] for _ in range(21)]
        pixels[9],pixels[8] = [0,100],[20,0]
        native = dict(frame=0,source_us=1,image='input.png',hands=[dict(id=42,
            confidence=.91,engaged=True,pinch_eligible=True,cursor=dict(x=.2,y=.3),
            pinch_distance_m=.08)])
        provider = dict(frame=0,source_us=1,image='input.png',hands=[dict(array_index=0,
            stable_id_available=False,world_landmarks=world,landmarks=pixels,world_pinch_distance_m=.02)])
        return native,provider

    def test_no_provider_identity_or_confidence_is_invented_and_missing_means_loss(self):
        native,provider=self.fixture()
        before=copy.deepcopy(native)
        result=substitute([native],[provider],.38)[0]
        expected=copy.deepcopy(native)
        expected['hands'][0].update(pinch_distance_m=.02,pinch_onset_allowed=True)
        self.assertEqual(result,expected)
        self.assertEqual(native,before)
        provider['hands']=[]
        self.assertEqual(substitute([native],[provider])[0]['hands'],[])

    def test_array_order_and_mismatched_source_cannot_become_native_identity(self):
        native,provider=self.fixture()
        provider['hands']*=2
        with self.assertRaisesRegex(ValueError,'no array identity'):
            substitute([native],[provider])
        for key,value in [('frame',1),('source_us',2),('image','other.png')]:
            native,provider=self.fixture();provider[key]=value
            with self.assertRaisesRegex(ValueError,'differs from native'):
                substitute([native],[provider])

    def test_invalid_metric_or_projected_scale_is_rejected(self):
        native,provider=self.fixture();provider['hands'][0]['world_pinch_distance_m']=.01
        with self.assertRaisesRegex(ValueError,'Declared gap'):
            substitute([native],[provider])
        native,provider=self.fixture();provider['hands'][0]['landmarks'][9]=[0,0]
        with self.assertRaisesRegex(ValueError,'Degenerate'):
            substitute([native],[provider],.38)
        native,provider=self.fixture();provider['hands'][0]['world_landmarks'][8][0]=math.nan
        with self.assertRaisesRegex(ValueError,'Invalid provider'):
            substitute([native],[provider])


if __name__=='__main__':
    unittest.main()
