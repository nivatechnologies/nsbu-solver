"""Importing study modules must not evaluate fields or advance a trajectory."""
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import runpy
import unittest
from unittest.mock import patch


class DriverImportTests(unittest.TestCase):
    def test_report_imports_do_not_perform_numerical_work(self) -> None:
        output = StringIO()
        forbidden = AssertionError('Import attempted numerical work')
        with patch('reference.evaluator.evaluate',side_effect=forbidden),\
             patch('reference.steps.cm_step',side_effect=forbidden),\
             patch('reference.steps.ho_step',side_effect=forbidden),redirect_stdout(output):
            for name in ('verify_fields','verify_steps','verify_sampling','verify_trajectory'):
                path = Path(__file__).resolve().parents[1]/(name+'.py')
                runpy.run_path(str(path),run_name='reference.'+name)
        self.assertEqual(output.getvalue(),'')
