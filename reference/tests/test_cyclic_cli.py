"""Public arithmetic-command admission, refusal codes and lossless numerical reports."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from mpmath import mp
from reference.cyclic.schema import decode_fixture
from reference.cyclic.study import StudyPlan
from reference.tests.test_cyclic_reference import encode, fixture
from reference.verify_cyclic import MAXIMUM_INPUT_BYTES, execute, fixed_input, state_rows
from tools.json_types import array_value, decode, object_value


class CyclicCliTests(unittest.TestCase):
    def invoke(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([sys.executable,'-m','reference.verify_cyclic',*args],
                              capture_output=True,text=True,check=False,timeout=20)

    def test_dry_run_validates_fixed_bytes_and_reports_their_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'force.json'
            raw = encode(fixture())
            path.write_bytes(raw)
            result = self.invoke('--force-bits',str(path),'--dry-run')
            self.assertEqual(result.returncode,0,result.stderr)
            report = object_value(decode(result.stdout))
            self.assertEqual(report['status'],'preflight')
            self.assertEqual(report['fixed_force_sha256'],fixed_input(path,4).sha256)
            self.assertEqual(report['accepted_pde_windows'],0)
            self.assertNotIn('state',report)
            path.write_text('{}')
            refused = self.invoke('--force-bits',str(path),'--dry-run')
            self.assertEqual(refused.returncode,1)
            self.assertEqual(object_value(decode(refused.stdout))['status'],'diagnostic-refused')
            path.unlink()
            self.assertEqual(self.invoke('--force-bits',str(path),'--dry-run').returncode,1)
            with self.assertRaisesRegex(ValueError,'memory reservation'):
                execute(StudyPlan(4,80,'CM',1),path,True)

    def test_cli_syntax_resource_and_plain_preflight_contracts(self) -> None:
        for args in (('--n','3'),('--precision','53'),('--method','RK4')):
            self.assertEqual(self.invoke(*args).returncode,2)
        result = self.invoke('--cap-bytes','1')
        self.assertEqual(result.returncode,1)
        self.assertEqual(object_value(decode(result.stdout))['accepted_pde_windows'],0)
        result = self.invoke('--n','12','--precision','120','--method','HO','--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        report = object_value(decode(result.stdout))
        self.assertIsNone(report['fixed_force_sha256'])
        self.assertEqual(object_value(report['preflight'])['maximum_rhs_evaluations'],120)

    def test_json_ambiguity_depth_and_byte_limits_are_refused(self) -> None:
        for raw in (b'{"grid":4,"grid":8}',b'NaN',b'Infinity',b'-Infinity',
                    b'['*2000+b'0'+b']'*2000,b'\xff'):
            with self.assertRaises(ValueError):
                decode_fixture(raw)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'oversize.json'
            with path.open('wb') as target:
                target.seek(MAXIMUM_INPUT_BYTES)
                target.write(b'0')
            with self.assertRaisesRegex(ValueError,'admission'):
                fixed_input(path,4)
        self.assertEqual(decode_fixture(b'{"[\\\"\\\\": [1]}'),{'["\\':[1]})

    def test_serialization_keeps_small_complex_components_at_declared_precision(self) -> None:
        with mp.workdps(120):
            value = mp.mpc(1+mp.mpf(2)**-200,mp.mpf(2)**-300)
            rows = state_rows({(-1,0,1):(value,mp.mpc(0),-value)},120)
            row = object_value(rows[0])
            self.assertEqual(row['mode'],[-1,0,1])
            components = array_value(row['value'])
            pair = array_value(components[0])
            self.assertLess(abs(mp.mpf(str(pair[0]))-value.real),mp.mpf('1e-118'))
            self.assertLess(abs(mp.mpf(str(pair[1]))/value.imag-1),mp.mpf('1e-119'))
