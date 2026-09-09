"""Pilot modules must never launch numerical work while being imported."""
import subprocess
import sys
import unittest


class PilotImportTests(unittest.TestCase):
    def test_import_is_inert(self) -> None:
        code = '''from unittest.mock import patch
with patch("reference.steps.cm_step",side_effect=RuntimeError("numerical work during import")), patch("reference.steps.ho_step",side_effect=RuntimeError("numerical work during import")):
 import reference.pilot
 import runpy
 runpy.run_path("reference/pilot.py",run_name="__import_probe__")
'''
        result = subprocess.run([sys.executable,'-c',code],capture_output=True,text=True,timeout=10,check=False)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(result.stdout,'')
