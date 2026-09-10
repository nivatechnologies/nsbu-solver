"""Complete independent N=4 comparison, explicit precision separation and public CLI refusals."""
import gzip
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from mpmath import mp
from reference.cyclic.comparison import Inputs
from reference.cyclic.schema import decode_fixture
from reference.derived.comparison import CAP_BYTES, QUANTITIES, compare, precision_ratios
from tools.json_types import JsonObject, object_value, string_value

ROOT=Path(__file__).resolve().parents[2]


def inputs(folder: Path) -> Inputs:
    """Extract public byte-preserved trajectory artifacts and the current exact-bit derived fixture."""
    paths=[ROOT/'fixtures/reference/derived-n4-cm.json.gz',ROOT/'evidence/p09/arithmetic/inputs/force-n4.json.gz']
    paths += [ROOT/f'evidence/p09/arithmetic/trajectories/n4-cm-{precision}{fixed}.json.gz'
              for fixed in ('','-fixed') for precision in (80,120)]
    for i,p in enumerate(paths):(folder/f'{i}.json').write_bytes(gzip.decompress(p.read_bytes()))
    return Inputs(*(folder/f'{i}.json' for i in range(6)))


class DerivedComparisonTests(unittest.TestCase):
    def test_complete_current_state_and_independent_trajectories_have_subordinate_precision_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            supplied=inputs(Path(temp))
            with self.assertRaises(ValueError):compare(supplied,4,'invalid',CAP_BYTES)
            with self.assertRaises(ValueError):compare(supplied,4,'CM',1)
            result=compare(supplied,4,'CM')
        self.assertEqual(result['status'],'diagnostic-completed')
        self.assertEqual(result['accepted_pde_windows'],0)
        findings=object_value(object_value(result['comparisons'])['rust_vs_fixed_120'])
        for key,(components,_floor) in QUANTITIES.items():
            value=object_value(findings[key])
            self.assertEqual(value['ordered_components'],components)
            self.assertEqual(value['samples'],512)
            error=mp.mpf(string_value(value['rms_error']))
            self.assertGreater(error,0)
            self.assertLess(error,mp.mpf('1e-12'))
        for group in object_value(result['precision_ratios']).values():
            for norms in object_value(group).values():
                for value in object_value(norms).values():
                    self.assertLess(mp.mpf(string_value(value)),mp.mpf('1e-40'))

    def test_zero_denominators_are_undefined_and_cli_preflight_is_independent_of_inputs(self) -> None:
        empty:JsonObject={key:{q:{'rms_error':'0','sampled_peak_error':'0'} for q in QUANTITIES}
                          for key in ('diagnostic_80_120','rust_vs_same_state_120','fixed_80_120',
                                      'rust_vs_fixed_120','independent_80_120','rust_vs_independent_120')}
        for group in precision_ratios(empty).values():
            for norms in object_value(group).values():self.assertTrue(all(v is None for v in object_value(norms).values()))
        command=[sys.executable,'-m','reference.compare_derived','--n','4','--method','CM']
        for flags,code in ((['--dry-run'],0),(['--dry-run','--cap-bytes','1'],1),([],2)):
            result=subprocess.run(command+flags,cwd=ROOT,text=True,capture_output=True,check=False)
            self.assertEqual(result.returncode,code)
            if code!=2: self.assertIsInstance(object_value(decode_fixture(result.stdout.encode())),dict)
        missing=[item for key in ('rust','force','independent80','independent120','fixed80','fixed120')
                 for item in ('--'+key,'/nonexistent/nsbu-derived-fixture')]
        result=subprocess.run(command+missing,cwd=ROOT,text=True,capture_output=True,check=False)
        self.assertEqual(result.returncode,1)
