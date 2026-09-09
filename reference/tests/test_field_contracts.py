"""Independent-oracle scaling and arithmetic refinement orchestration contracts."""
from dataclasses import replace
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.evaluator import Evaluation
from reference.verify_fields import FieldSample, SAMPLES, compare, run, sample
from tools.json_types import array_value, object_value


def force_value(value: mpf) -> Evaluation:
    zero = mp.mpf(0)
    vector = (value,zero,zero)
    return Evaluation(vector,zero,vector,zero,(zero,),((zero,zero,zero,zero),)*3,0,((zero,zero,zero),)*3)


class FieldContractTests(unittest.TestCase):
    def test_scaled_oracle_error_and_threshold(self) -> None:
        with mp.workdps(120):
            point = ('0','0','0','0')
            for magnitude in (mp.mpf(0),mp.mpf(7)):
                delta = mp.mpf('1.234567890123456789e-70')
                value = force_value(magnitude+delta)
                oracle = (magnitude,mp.mpf(0),mp.mpf(0))
                with patch('reference.verify_fields.evaluate',return_value=value),\
                     patch('reference.verify_fields.scalar_force',return_value=oracle):
                    result = sample('scaled',point)
                expected = delta/max(1,magnitude)
                self.assertLess(abs(result.scaled_oracle_error/expected-1),mp.mpf('1e-49'))
            with patch('reference.verify_fields.evaluate',return_value=force_value(mp.mpf('1e-60'))),\
                 patch('reference.verify_fields.scalar_force',return_value=(mp.mpf(0),)*3):
                with self.assertRaisesRegex(ArithmeticError,'force disagreement'):
                    sample('boundary',point)

    def test_precision_comparison_scaling_and_exact_boundary(self) -> None:
        with mp.workdps(120):
            high = FieldSample('scaled',('0','0','0','0'),force_value(mp.mpf(7)),mp.mpf(0))
            low = replace(high,evaluated=force_value(mp.mpf(7)-mp.mpf('7e-70')))
            self.assertEqual(compare(low,high)['force_scaled_change'],'1.0e-70')
            low = replace(high,evaluated=force_value(mp.mpf(0)))
            high = replace(high,evaluated=force_value(mp.mpf('1e-60')))
            with self.assertRaisesRegex(ArithmeticError,'Precision refinement'):
                compare(low,high)

    def test_both_precisions_and_distinct_samples_are_compared(self) -> None:
        calls: list[tuple[str,int]] = []
        def sampled(label: str, coordinates: tuple[str,str,str,str]) -> FieldSample:
            calls.append((label,mp.dps))
            value = mp.mpf('1e-70') if mp.dps == 80 else mp.mpf(0)
            return FieldSample(label,coordinates,force_value(value),mp.mpf(0))
        with patch('reference.verify_fields.sample',side_effect=sampled):
            report = run()
        self.assertEqual(calls,[(name,p) for p in (80,120) for name,_ in SAMPLES])
        self.assertEqual(set(object_value(report['precisions'])),{'80','120'})
        self.assertEqual(report['precision_comparison'],[
            {'sample':name,'force_scaled_change':'1.0e-70'} for name,_ in SAMPLES])
        self.assertEqual(len(array_value(object_value(report['precisions'])['80'])),9)

    def test_derivative_requests_have_four_coordinates_and_one_sided_startup(self) -> None:
        from collections.abc import Callable
        from reference.verify_fields import scalar_force
        with mp.workdps(80):
            for time in (mp.mpf(0),mp.mpf(1)/1024):
                calls: list[tuple[tuple[int,...],int]] = []
                point = (mp.mpf(0),mp.mpf(0),mp.mpf(0),time)
                def differentiated(_function: Callable[...,mpf], coordinates: tuple[mpf,...],
                                   orders: tuple[int,...], direction: int = 0) -> mpf:
                    self.assertEqual(coordinates,point)
                    calls.append((orders,direction))
                    return mp.mpf(0)
                with patch.object(mp,'diff',side_effect=differentiated):
                    self.assertEqual(scalar_force(point),(0,0,0))
                first = [(order,0) for order in ((1,0,0,0),(0,1,0,0),(0,0,1,0))]
                first.append(((0,0,0,1),1 if time == 0 else 0))
                second = [(order,0) for order in ((2,0,0,0),(0,2,0,0),(0,0,2,0))]
                expected = [request for pressure in first[:3] for request in first+second+[pressure]]
                self.assertEqual(calls,expected)
