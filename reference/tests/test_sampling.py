"""Sampling normalization and full fine-band diagnostic contracts."""
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import runpy
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.evaluator import Evaluation
from reference.verify_sampling import difference, run, sample_force
from tools.json_types import JsonObject, decode, object_value


def smooth_force(x: mpf, _y: mpf, _z: mpf, _time: mpf) -> Evaluation:
    zero = mp.mpf(0)
    vector = (zero,mp.cos(2*mp.pi*x),zero)
    return Evaluation((zero,zero,zero),zero,vector,zero,(zero,),
                      ((zero,zero,zero,zero),(zero,vector[1],zero,zero),(zero,zero,zero,zero)),
                      2,((zero,zero,zero),)*3)


def fixed_sampling(n: int, _time: mpf, precision: int, _cap: int) -> tuple[Spectrum,JsonObject]:
    return {(0,0,0):(mp.mpc(1),mp.mpc(2),mp.mpc(3))},{'grid':n,'precision':precision}


class SamplingTests(unittest.TestCase):
    def test_normalization_and_origin(self) -> None:
        with mp.workdps(80),patch('reference.verify_sampling.evaluate',side_effect=smooth_force):
            coefficients,report = sample_force(4,mp.mpf(1)/256,80,4*1024**3)
            for mode,vector in coefficients.items():
                expected = mp.mpf(1)/2 if mode in ((1,0,0),(-1,0,0)) else mp.mpf(0)
                self.assertLess(abs(vector[1]-expected),mp.mpf('1e-75'))
                self.assertEqual(vector[0],0)
                self.assertEqual(vector[2],0)
            self.assertEqual(report['field_evaluations'],64)
            self.assertEqual(report['maximum_field_evaluations'],64)
            self.assertEqual(report['scalar_root_iterations'],128)
            self.assertEqual(mp.mpf(str(report['max_assembly_conditioning'])),1)
        for time in (mp.mpf(-1),mp.mpf(1)/128,mp.mpf('nan')):
            with self.assertRaisesRegex(ValueError,'sampling requires'):
                sample_force(4,time,80,4*1024**3)

    def test_full_fine_band_high_mode(self) -> None:
        with mp.workdps(80):
            coarse: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            fine: Spectrum = dict(coarse)
            fine[(3,0,0)] = (mp.mpc(0),mp.mpc(2),mp.mpc(0))
            result = difference(coarse,fine)
            self.assertEqual(mp.mpf(str(result['full_fine_band_l2'])),2)
            self.assertLess(abs(mp.mpf(str(result['full_fine_band_h1']))-2*mp.sqrt(1+36*mp.pi**2)),mp.mpf('1e-17'))
            self.assertEqual(mp.mpf(str(result['max_component_change'])),2)

    def test_orchestration_and_arithmetic_refusal(self) -> None:
        with patch('reference.verify_sampling.sample_force',side_effect=fixed_sampling):
            result = run()
            self.assertEqual(result['status'],'diagnostic-completed')
            self.assertEqual(result['case'],'similarity-mms-v2')
            self.assertIn('unresolved',str(result['spatial_qualification']))
            with patch('reference.verify_sampling.difference',return_value={'max_component_change':'1e-60'}):
                with self.assertRaisesRegex(ArithmeticError,'arithmetic refinement'):
                    run()

    def test_entry_point(self) -> None:
        output = StringIO()
        with patch('reference.evaluator.evaluate',side_effect=smooth_force),redirect_stdout(output):
            runpy.run_path(str(Path(__file__).resolve().parents[1]/'verify_sampling.py'),run_name='__main__')
        result = object_value(decode(output.getvalue()))
        self.assertEqual(result['status'],'diagnostic-completed')
