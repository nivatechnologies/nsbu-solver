"""Smooth trajectory contracts; report doubles do not establish numerical order."""
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import runpy
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.steps import RightHandSide, cm_step
from reference.tuples import triple
from reference.verify_trajectory import Method, errors, prescribed_force, run, spatial_profile, trajectory
from tools.json_types import JsonObject


def manufactured_result(_method: Method, divisor: int, precision: int) -> tuple[Spectrum,JsonObject]:
    with mp.workdps(precision):
        state = {k:triple(mp.sin(mp.mpf(13)/32)*v for v in values)
                 for k,values in spatial_profile().items()}
        mean = state[(0,0,0)]
        state[(0,0,0)] = (mean[0]+mp.mpf(divisor)**-4,mean[1],mean[2])
        return state,{'divisor':divisor,'precision':precision}


def euler_probe(state: Spectrum, time: mpf, dt: mpf, rhs: RightHandSide, _nu: mpf) -> Spectrum:
    if any(value != 0 for vector in state.values() for value in vector):
        raise AssertionError('Expected rest at the first independent step')
    return {k:triple(dt*v for v in vector) for k,vector in rhs(state,time).items()}


class TrajectoryTests(unittest.TestCase):
    def test_profile_and_exact_error(self) -> None:
        with mp.workdps(80):
            profile = spatial_profile()
            self.assertEqual(profile[(0,0,0)],(1,-2,3))
            for mode,vector in profile.items():
                self.assertEqual(profile[triple(-m for m in mode)],triple(mp.conj(v) for v in vector))
            time = mp.mpf(1)/32
            state = {k:triple(mp.sin(13*time)*v for v in vector) for k,vector in profile.items()}
            self.assertEqual(errors(state,profile,time),(0,0))
            zero = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in profile}
            self.assertEqual(prescribed_force(profile,zero,mp.mpf(0)),
                             {k:triple(13*v for v in vector) for k,vector in profile.items()})

    def test_from_rest_and_invalid_divisors(self) -> None:
        for divisor in (0,-32,31,33):
            with self.assertRaisesRegex(ValueError,'multiple of 32'):
                trajectory(cm_step,divisor,80)
        state,report = trajectory(euler_probe,32,80)
        self.assertEqual(state[(0,0,0)],(mp.mpf(13)/32,-mp.mpf(26)/32,mp.mpf(39)/32))
        self.assertEqual(report['steps'],1)
        self.assertEqual(report['reference_assignments'],0)
        self.assertGreater(float(str(report['h1_error'])),0)

    def test_report_and_failures(self) -> None:
        with patch('reference.verify_trajectory.trajectory',side_effect=manufactured_result):
            report = run()
            self.assertEqual(report['status'],'passed')
            self.assertEqual(report['reference_assignments'],0)
            self.assertEqual(report['case'],'smooth-cyclic-mms-v1')
            with patch('reference.verify_trajectory.errors',return_value=(mp.mpf(1),mp.mpf(1))):
                with self.assertRaisesRegex(ArithmeticError,'temporal order'):
                    run()
        def inaccurate(method: Method, divisor: int, precision: int) -> tuple[Spectrum,JsonObject]:
            state,report = manufactured_result(method,divisor,precision)
            if precision == 120:
                state[(0,0,0)] = (mp.mpc(0),mp.mpc(0),mp.mpc(0))
            return state,report
        with patch('reference.verify_trajectory.trajectory',side_effect=inaccurate):
            with self.assertRaisesRegex(ArithmeticError,'arithmetic refinement'):
                run()

    def test_entry_point_rejects_unconverged_double(self) -> None:
        output = StringIO()
        with patch('reference.steps.cm_step',side_effect=euler_probe),redirect_stdout(output):
            with self.assertRaisesRegex(AssertionError,'Expected rest'):
                runpy.run_path(str(Path(__file__).resolve().parents[1]/'verify_trajectory.py'),run_name='__main__')
        self.assertEqual(output.getvalue(),'')
