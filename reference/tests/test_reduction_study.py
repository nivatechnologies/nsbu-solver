"""Complete actual-data arithmetic reports and bounded public command failure behavior."""
from collections.abc import Mapping
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from reference.reduction_audit.study import bounded, compare, preflight
from tools.json_types import Json, array_value, decode, object_value

ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'crates/nsbu-benchmarks/data/reduction-n4.bin'
MAGNITUDES=ROOT/'fixtures/reference/reduction-n4-magnitudes.bin'


def command(*args: str) -> tuple[int,Mapping[str,Json]]:
    result=subprocess.run([sys.executable,'-m','reference.verify_reductions',*args],
                          text=True,capture_output=True,check=False)
    return result.returncode,object_value(decode(result.stdout)) if result.stdout else {}


class ReductionStudyTests(unittest.TestCase):
    def test_actual_complete_six_quantity_public_report_preserves_both_precision_profiles(self) -> None:
        code,report=command('--n','4','--input',str(SOURCE),'--magnitudes',str(MAGNITUDES))
        self.assertEqual(code,0)
        self.assertEqual(report['accepted_pde_windows'],0)
        quantities=object_value(report['quantities']);self.assertEqual(len(quantities),6)
        for value in quantities.values():
            row=object_value(value)
            self.assertEqual(row['samples'],512)
            self.assertEqual(len(array_value(row['profiles'])),2)
            for key in ('direct_vs_exact_components','physical_vs_exact_components','physical_vs_rounded_magnitudes'):
                statistics=object_value(row[key]);self.assertEqual(len(statistics),4)
                for statistic in statistics.values():self.assertTrue(object_value(statistic)['precision_separated'])

    def test_preflight_runs_without_files_and_refuses_invalid_grid_and_insufficient_cap(self) -> None:
        self.assertEqual(preflight(12)['physical_points'],13824)
        for n in (0,6,16):
            with self.assertRaises(ValueError):preflight(n)
        with patch('reference.reduction_audit.study.bounded',side_effect=AssertionError('No input read allowed')):
            with self.assertRaises(ValueError):compare(Path('absent'),Path('absent'),4,1)
        code,result=command('--n','12','--dry-run');self.assertEqual(code,0)
        self.assertEqual(result['accepted_pde_windows'],0)
        self.assertEqual(command('--n','4')[0],2)
        self.assertEqual(command('--n','4','--extra')[0],2)
        self.assertEqual(command('--n','4','--dry-run','--cap-bytes','1')[0],1)
        self.assertEqual(command('--n','4','--input','absent','--magnitudes','absent')[0],1)

    def test_truncation_trailing_bytes_and_an_internally_changed_grid_are_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path=Path(temporary)/'bad.bin';path.write_bytes(b'123')
            with self.assertRaises(ValueError):bounded(path,2)
            with self.assertRaises(ValueError):bounded(path,4)
            self.assertEqual(bounded(path,3),b'123')
            # Bypass only the bounded reader to exercise the second independent grid check.
            from reference.reduction_audit.input import parse
            source=parse(SOURCE.read_bytes())
            with patch('reference.reduction_audit.study.input.parse',return_value=source):
                with patch('reference.reduction_audit.study.bounded',return_value=source.data):
                    with self.assertRaisesRegex(ValueError,'differs from the preflight'):
                        compare(SOURCE,MAGNITUDES,8)
