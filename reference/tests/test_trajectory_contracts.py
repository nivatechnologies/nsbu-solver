"""Trajectory orchestration budgets and final-refinement policy, not PDE evidence."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.steps import RightHandSide
from reference.tests.test_trajectory import manufactured_result
from reference.verify_trajectory import run, trajectory
from tools.json_types import array_value, object_value


class TrajectoryContractTests(unittest.TestCase):
    def test_complete_tick_schedule_and_viscosity(self) -> None:
        calls: list[tuple[mpf,mpf,mpf,int]] = []
        def advance(state: Spectrum, time: mpf, dt: mpf, _rhs: RightHandSide, nu: mpf) -> Spectrum:
            calls.append((time,dt,nu,mp.dps))
            return dict(state)
        _,report = trajectory(advance,1024,80)
        with mp.workdps(80):
            self.assertEqual(calls,[(mp.mpf(tick)/1024,mp.mpf(1)/1024,mp.mpf(1),80) for tick in range(32)])
        self.assertEqual(report['steps'],32)
        self.assertEqual(report['precision'],80)
        reservation = object_value(report['preflight'])
        self.assertEqual(reservation['cap_bytes'],1024**3)

    def test_only_final_h1_refinement_controls_order_gate(self) -> None:
        with mp.workdps(80):
            for final_order in ('3.5','4'):
                norms = [mp.mpf(64),mp.mpf(16),mp.mpf(4),4/mp.mpf(2)**mp.mpf(final_order)]
                measurements = [(mp.mpf(1),v) for v in norms]*2
                with patch('reference.verify_trajectory.trajectory',side_effect=manufactured_result),\
                     patch('reference.verify_trajectory.errors',side_effect=measurements):
                    report = run()
                self.assertEqual(report['grid'],4)
                for value in array_value(report['reports']):
                    row = object_value(value)
                    self.assertEqual(row['h1_observed_orders'],['2.0','2.0',str(mp.mpf(final_order))])
            measurements = [(mp.mpf(1),mp.mpf(v)) for v in (4096,256,16,4)]
            with patch('reference.verify_trajectory.trajectory',side_effect=manufactured_result),\
                 patch('reference.verify_trajectory.errors',side_effect=measurements):
                with self.assertRaisesRegex(ArithmeticError,'temporal order'):
                    run()
