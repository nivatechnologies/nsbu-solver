"""Complete-band differences, metadata refusals and comparison-command behavior."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from mpmath import mp
from reference.cyclic.comparison import MAXIMUM_INPUT_BYTES, RESERVATION_BYTES, bounded_bytes, compare, difference
from reference.cyclic.results import decimal, reality_defect, reference_state, rust_state
from reference.tests.cyclic_artifacts import reference_report, rust_report, write_inputs
from tools.json_types import Json, array_value, decode, object_value


class CyclicComparisonTests(unittest.TestCase):
    def test_full_high_mode_and_asymmetric_stored_plane_are_never_repaired(self) -> None:
        data = rust_report()
        rows = [dict(object_value(row)) for row in array_value(data['state'])]
        data['state'] = rows
        rows[16]['bits'] = [[963 << 52,0],[0,0],[0,0]]  # (1,1,1), exactly 2^-60
        rows[3]['bits'] = [[0,963 << 52],[0,0],[0,0]]  # (0,1,0), leave partner unchanged
        with mp.workdps(120):
            state = rust_state(data,4,'CM')
            zero = reference_state(reference_report(120,None),4,'CM',120,None)
            amplitude = mp.mpf(2)**-60
            self.assertEqual(state[(1,1,1)][0],amplitude)
            self.assertEqual(state[(-1,-1,-1)][0],amplitude)
            self.assertEqual(state[(0,-1,0)][0],0)
            self.assertEqual(reality_defect(state),amplitude)
            measured = difference(state,zero)
            differences = array_value(measured['coefficient_differences'])
            self.assertEqual(len(differences),27)
            high = object_value(differences[-1])
            self.assertEqual(high['mode'],[1,1,1])
            pair = array_value(array_value(high['value'])[0])
            self.assertLess(abs(mp.mpf(str(pair[0]))/amplitude-1),mp.mpf('1e-38'))
            self.assertEqual(pair[1],'0.0')
            self.assertLess(abs(mp.mpf(str(measured['l2']))/(mp.sqrt(3)*amplitude)-1),mp.mpf('1e-38'))
            expected = amplitude*mp.sqrt(3+28*mp.pi**2)
            self.assertLess(abs(mp.mpf(str(measured['h1']))/expected-1),mp.mpf('1e-38'))

    def test_reference_schedule_identity_and_numerical_shape_are_mandatory(self) -> None:
        changes: list[tuple[str,Json]] = [('grid',8),('method','HO'),('precision',80),
            ('rhs_calls',95),('reference_assignments',False),('fixed_force_sha256','0'*64),
            ('fine_steps_committed',8),('accepted_pde_windows',1),('state',[])]
        for key, value in changes:
            data = reference_report(120,None)
            data[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                reference_state(data,4,'CM',120,None)
        data = reference_report(120,None)
        del data['fixed_force_sha256']
        with self.assertRaises(ValueError):
            reference_state(data,4,'CM',120,None)
        for key, value in (('mode',[0,0,0]),('value',[['0'],['0','0'],['0','0']]),
                           ('value',[['nan','0'],['0','0'],['0','0']])):
            data = reference_report(120,None)
            rows = [dict(object_value(row)) for row in array_value(data['state'])]
            data['state'] = rows
            rows[0][key] = value
            with self.assertRaises(ValueError):
                reference_state(data,4,'CM',120,None)
        for value in ('x'*257,'inf',1):
            with self.assertRaises(ValueError):
                decimal(value)

    def test_rust_layout_and_nyquist_refusals_and_complete_band_comparison(self) -> None:
        data = rust_report()
        data['state'] = []
        with self.assertRaises(ValueError):
            rust_state(data,4,'CM')
        for index, key, value in ((0,'mode',[1,0,0]),(2,'bits',[[1023 << 52,0],[0,0],[0,0]])):
            data = rust_report()
            rows = [dict(object_value(row)) for row in array_value(data['state'])]
            data['state'] = rows
            rows[index][key] = value
            with self.assertRaises(ValueError):
                rust_state(data,4,'CM')
        with self.assertRaises(ValueError):
            difference({}, {})

    def test_comparison_remains_diagnostic_and_checks_caps_before_opening_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            paths = write_inputs(Path(directory),'HO')
            report = compare(paths,4,'HO',RESERVATION_BYTES)
            self.assertEqual(report['accepted_pde_windows'],0)
            self.assertEqual(report['artifact_origin'],'externally supplied diagnostic numbers')
            comparisons = object_value(report['comparisons'])
            self.assertEqual(object_value(comparisons['rust_vs_fixed_120'])['h1'],'0.0')
            paths.rust.unlink()
            for method, cap in (('bad',RESERVATION_BYTES),('HO',1)):
                with self.assertRaises(ValueError):
                    compare(paths,4,method,cap)
            with self.assertRaises(OSError):
                compare(paths,4,'HO',RESERVATION_BYTES)
            with paths.rust.open('wb') as target:
                target.seek(MAXIMUM_INPUT_BYTES)
                target.write(b'0')
            with self.assertRaises(ValueError):
                bounded_bytes(paths.rust)

    def test_public_comparison_command_checks_fixed_hash_and_exit_codes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = write_inputs(root)
            args = [sys.executable,'-m','reference.compare_cyclic','--n','4','--method','CM']
            for name in ('rust','force','independent80','independent120','fixed80','fixed120'):
                args.extend(['--'+name,str(root/(name+'.json'))])
            success = subprocess.run(args,capture_output=True,text=True,check=False,timeout=20)
            self.assertEqual(success.returncode,0,success.stderr)
            self.assertEqual(object_value(decode(success.stdout))['accepted_pde_windows'],0)
            data = reference_report(120,'f'*64)
            paths.fixed120.write_text(json.dumps(data))
            refused = subprocess.run(args,capture_output=True,text=True,check=False,timeout=20)
            self.assertEqual(refused.returncode,1)
            self.assertEqual(object_value(decode(refused.stdout))['status'],'diagnostic-refused')
            syntax = subprocess.run(args+['--n','3'],capture_output=True,text=True,check=False,timeout=20)
            self.assertEqual(syntax.returncode,2)
