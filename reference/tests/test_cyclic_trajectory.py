"""Actual independent from-rest trajectories and finite reference-resource admission."""
import unittest
from mpmath import mp
from reference.cyclic.bits import FixedForcing
from reference.cyclic.profile import profile, projected_force
from reference.cyclic.study import StudyPlan, StudyRhs, maximum_difference, trajectory
from reference.dft import Spectrum, preflight, retained
from reference.tests.test_cyclic_reference import encode, fixture
from tools.json_types import array_value, object_value


class CyclicTrajectoryTests(unittest.TestCase):
    def test_both_independent_methods_evolve_from_rest_without_reference_assignment(self) -> None:
        with mp.workdps(80):
            reference = profile(4,mp.mpf(1)/512)[0]
            for method in ('CM','HO'):
                state, report = trajectory(StudyPlan(4,80,method,4*1024**3))
                error = maximum_difference(state,reference)
                self.assertLess(error,mp.mpf('1e-11'))
                self.assertGreater(error,mp.mpf('1e-30'))
                self.assertEqual(report['rhs_calls'],96 if method == 'CM' else 120)
                self.assertEqual(report['reference_assignments'],0)
                self.assertEqual(report['accepted_pde_windows'],0)
                self.assertEqual(len(array_value(report['full_step_two_half_max_differences'])),8)
                self.assertIsNone(report['fixed_force_sha256'])
                self.assertEqual(report['case'],'CyclicSine')
                self.assertEqual(report['fine_steps_committed'],16)

    def test_exact_fixed_stage_inputs_are_labeled_separately_and_cannot_change_the_grid(self) -> None:
        raw = encode(fixture())
        force = FixedForcing(raw,4,len(raw))
        state, report = trajectory(StudyPlan(4,80,'CM',4*1024**3,force))
        self.assertTrue(all(v == 0 for vector in state.values() for v in vector))
        self.assertEqual(report['fixed_force_sha256'],force.sha256)
        self.assertEqual(report['case'],'fixed-stage-arithmetic-fixture')
        self.assertEqual(report['nominal_comparison_case'],'CyclicSine')
        reservation = object_value(report['preflight'])
        self.assertEqual(reservation['force_fixture_reservation'],768*1024**2)
        with self.assertRaises(ValueError):
            StudyPlan(8,80,'CM',4*1024**3,force).reservation()
        base = int(str(preflight(4,80,4*1024**3)['reserved_bytes']))
        with self.assertRaises(ValueError):
            StudyPlan(4,80,'CM',base+1,force).reservation()

    def test_stage_window_and_force_band_failures_do_not_refund_work(self) -> None:
        state: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(4)}
        rhs = StudyRhs(4,4,lambda time: projected_force(4,time))
        for time in (mp.inf,mp.mpf(-1),mp.mpf(129)/65536):
            with self.assertRaises(ArithmeticError):
                rhs(state,time)
        self.assertEqual(rhs.calls,3)
        missing = StudyRhs(4,1,lambda _time: {})
        with self.assertRaises(ArithmeticError):
            missing(state,mp.mpf(0))
        self.assertEqual(missing.calls,1)
