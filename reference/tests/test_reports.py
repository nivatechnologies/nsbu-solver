"""Report contracts and failure attribution, with bounded orchestration doubles."""
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import runpy
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.evaluator import Evaluation
from reference.steps import RightHandSide
from reference.verify_fields import FieldSample, compare as compare_fields, run as run_fields, sample
from reference.verify_steps import Study, compare as compare_steps, force, right_hand_side, run as run_steps
from tools.json_types import JsonObject, decode, object_value


def zero_evaluation(_x: mpf, _y: mpf, _z: mpf, _t: mpf) -> Evaluation:
    zero = mp.mpf(0)
    triple = (zero,zero,zero)
    return Evaluation(triple,zero,triple,zero,(zero,),((zero,zero,zero,zero),)*3,0,(triple,)*3)


def zero_fields(_x: mpf, _y: mpf, _z: mpf, _t: mpf) -> tuple[tuple[mpf,mpf,mpf],mpf]:
    return (mp.mpf(0),mp.mpf(0),mp.mpf(0)),mp.mpf(0)


def fixed_step(state: Spectrum, _time: mpf, _dt: mpf, _rhs: RightHandSide, _nu: mpf) -> Spectrum:
    return dict(state)


class ReportTests(unittest.TestCase):
    def setUp(self) -> None:
        context = mp.workdps(80)
        context.__enter__()
        self.addCleanup(context.__exit__,None,None,None)

    def test_field_failure_attribution(self) -> None:
        point = ('1/32','1/64','1/10','1/1024')
        with patch('reference.verify_fields.scalar_force',return_value=(mp.mpf(0),)*3):
            with self.assertRaisesRegex(ArithmeticError,'force disagreement'):
                sample('bad-oracle',point)
        good = zero_evaluation(*(mp.mpf(0),)*4)
        bad = Evaluation(good.velocity,good.pressure_raw,(mp.mpf(1),mp.mpf(0),mp.mpf(0)),
                         good.divergence,good.root_residual_coefficients,good.momentum_terms,
                         good.root_iterations,good.force_gradient)
        with self.assertRaisesRegex(ArithmeticError,'Precision refinement'):
            compare_fields(FieldSample('same',point,good,mp.mpf(0)),FieldSample('same',point,bad,mp.mpf(0)))

    def test_step_failure_attribution(self) -> None:
        zero: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
        nonzero: Spectrum = {(0,0,0):(mp.mpc(1),mp.mpc(0),mp.mpc(0))}
        low = Study(80,'CM',{},zero,zero,mp.mpf(0))
        high = Study(120,'CM',{},nonzero,nonzero,mp.mpf(0))
        with self.assertRaisesRegex(ArithmeticError,'DFT step arithmetic'):
            compare_steps(low,high)

    def test_actual_force_and_rhs(self) -> None:
        time = mp.mpf(1)/100
        prescribed = force(4,time)
        self.assertEqual(prescribed[(0,0,0)],(1,-2,3))
        self.assertLess(abs(prescribed[(0,1,0)][0]-(1+mp.sin(7*time))/(2*mp.j)),mp.mpf('1e-75'))
        zero: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in prescribed}
        computed = right_hand_side(4)(zero,time)
        self.assertEqual(computed,prescribed)

    def assert_report(self, report: JsonObject, scope: str) -> None:
        self.assertEqual(report['status'],'passed')
        self.assertIn(scope,str(report['scope']))
        self.assertTrue(report['limitations'])

    def test_field_report_and_entry_point(self) -> None:
        with patch('reference.verify_fields.evaluate',side_effect=zero_evaluation),patch('reference.verify_fields.fields',side_effect=zero_fields):
            self.assert_report(run_fields(),'pointwise')
        output = StringIO()
        with patch('reference.evaluator.evaluate',side_effect=zero_evaluation),patch('reference.scalar.fields',side_effect=zero_fields),redirect_stdout(output):
            runpy.run_path(str(Path(__file__).resolve().parents[1]/'verify_fields.py'),run_name='__main__')
        report = object_value(decode(output.getvalue()))
        self.assertEqual(report['status'],'passed')
        self.assertEqual(report['scaled_error_tolerance'],'1e-60')

    def test_step_report_and_entry_point(self) -> None:
        with patch('reference.verify_steps.cm_step',side_effect=fixed_step),patch('reference.verify_steps.ho_step',side_effect=fixed_step):
            self.assert_report(run_steps(),'N=4')
        output = StringIO()
        with patch('reference.steps.cm_step',side_effect=fixed_step),patch('reference.steps.ho_step',side_effect=fixed_step),redirect_stdout(output):
            runpy.run_path(str(Path(__file__).resolve().parents[1]/'verify_steps.py'),run_name='__main__')
        report = object_value(decode(output.getvalue()))
        self.assertEqual(report['reference_assignments'],0)
        self.assertEqual(report['dt'],'1/100')
