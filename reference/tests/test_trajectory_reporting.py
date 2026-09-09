"""Noninteger order/error formatting and exact arithmetic rejection policy."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.verify_trajectory import Method, run
from tools.json_types import JsonObject, array_value, object_value


class TrajectoryReportingTests(unittest.TestCase):
    def run_policy(self, high_value: str) -> JsonObject:
        counter = 0
        original = mp.nstr
        def simulated(_method: Method, _divisor: int, precision: int) -> tuple[Spectrum,JsonObject]:
            with mp.workdps(precision):
                value = mp.mpf(high_value) if precision == 120 else mp.mpf(0)
                return {(0,0,0):(mp.mpc(value),mp.mpc(0),mp.mpc(0))},{}
        def measured(_state: Spectrum, _profile: Spectrum, _time: mpf) -> tuple[mpf,mpf]:
            nonlocal counter
            self.assertEqual(mp.dps,80)
            value = (mp.mpf(1),mp.mpf(1)/7,mp.mpf(1)/49,mp.mpf(1)/1000)[counter%4]
            counter += 1
            return mp.mpf(1),value
        def rendered(value: mpf, digits: int) -> str:
            self.assertEqual(mp.dps,120)
            return original(value,digits)
        with patch('reference.verify_trajectory.trajectory',side_effect=simulated),\
             patch('reference.verify_trajectory.errors',side_effect=measured),\
             patch.object(mp,'nstr',side_effect=rendered):
            return run()

    def test_noninteger_orders_and_nonzero_precision_changes(self) -> None:
        result = self.run_policy('1.2345678901234567891234e-70')
        for value in array_value(result['reports']):
            row = object_value(value)
            self.assertEqual(row['h1_observed_orders'],[
                '2.8073549220576041074','2.8073549220576041074','4.3510744405468788287'])
            self.assertEqual(row['max_full_band_arithmetic_change'],'1.2345678901234567891e-70')

    def test_exact_arithmetic_boundary_is_refused(self) -> None:
        with self.assertRaisesRegex(ArithmeticError,'arithmetic refinement'):
            self.run_policy('1e-60')
