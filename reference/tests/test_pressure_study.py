"""Complete finite pressure study, frozen identity and public command failure contracts."""
from fractions import Fraction
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from mpmath import mp
from reference.pressure_gauge import MeanPlan, PressureMean
from reference.pressure_study import changes, preflight, study
from tools.json_types import Json, object_value, decode, string_value


def command(*args: str) -> tuple[int,dict[str,Json]]:
    result=subprocess.run([sys.executable,'-m','reference.verify_pressure_mean',*args],
                          text=True,capture_output=True,check=False)
    return result.returncode,dict(object_value(decode(result.stdout))) if result.stdout else {}


class PressureStudyTests(unittest.TestCase):
    def test_complete_profiles_separate_precision_and_each_quadrature_direction(self) -> None:
        admission=preflight(Fraction(1,256),2)
        self.assertEqual(admission['sequential_profiles'],10)
        self.assertEqual(admission['pressure_evaluations'],410)
        result=study(Fraction(1,256),2)
        self.assertEqual(result['status'],'pressure-mean-diagnostic-complete')
        self.assertEqual(result['accepted_pde_windows'],0)
        measured=object_value(result['changes'])
        self.assertLess(mp.mpf(string_value(measured['finest_mean'])),0)
        self.assertLess(mp.mpf(string_value(measured['maximum_precision_to_finest_quadrature_ratio'])),mp.mpf('1e-60'))
        self.assertGreater(mp.mpf(string_value(measured['relative_finest_quadrature_change'])),0)
        rest=object_value(study(Fraction(0),2)['changes'])
        self.assertEqual(rest['maximum_precision_to_finest_quadrature_ratio'],None)
        self.assertEqual(rest['relative_finest_quadrature_change'],None)
        self.assertEqual(rest['finest_mean'],'0.0')

    def test_named_changes_do_not_conflate_quadrature_with_arithmetic(self) -> None:
        plan=MeanPlan(Fraction(1,256),2,2)
        values=(10,9,8,7,6,10,8,4,5,6)
        result=changes(tuple(PressureMean(plan,mp.mpf(v)) for v in values))
        for key,value in (('coarse_to_middle','2.0'),('middle_to_fine','4.0'),
                          ('axial_only_to_fine','1.0'),('radial_only_to_fine','2.0'),
                          ('maximum_precision_to_finest_quadrature_ratio','1.0'),
                          ('relative_finest_quadrature_change','1.0')):
            self.assertEqual(result[key],value)

    def test_frozen_case_and_all_profiles_are_admitted_before_evaluation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path=Path(temporary)/'changed.json'
            path.write_text('{}')
            with patch('reference.pressure_study.CASE',path):
                with self.assertRaisesRegex(ValueError,'hash mismatch'):preflight(Fraction(0),2)
            path.unlink()
            with patch('reference.pressure_study.CASE',path):
                with self.assertRaises(OSError):preflight(Fraction(0),2)
        with patch('reference.pressure_study.evaluate',side_effect=AssertionError('No evaluation allowed')):
            with self.assertRaises(ValueError):study(Fraction(0),258)
            with self.assertRaises(ValueError):study(Fraction(0),2,1)
            self.assertEqual(preflight(Fraction(0),2)['accepted_pde_windows'],0)

    def test_public_command_preflight_complete_refusal_and_bad_syntax(self) -> None:
        code,report=command('--dry-run','--panels','2')
        self.assertEqual((code,report['status']),(0,'pressure-mean-preflight'))
        code,report=command('--time','0','--panels','2')
        self.assertEqual((code,report['status']),(0,'pressure-mean-diagnostic-complete'))
        code,report=command('--dry-run','--cap-bytes','1')
        self.assertEqual((code,report['status']),(1,'pressure-mean-refused'))
        for time in ('nope','1/0'):
            self.assertEqual(command('--time',time)[0],2)
        self.assertEqual(command('--bad-argument')[0],2)
