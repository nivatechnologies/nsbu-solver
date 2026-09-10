"""Independent scalar differentiation and fixture failure semantics."""
import unittest
from pathlib import Path
import subprocess
import sys
from mpmath import mp, mpf
from reference.scalar import fields, rational
from reference.tuples import quadruple
from reference.verify_derivatives import FIELDS, evaluate, precision_change
from tools.json_types import array_value, decode, object_value, string_value


class DerivativeFixtures(unittest.TestCase):
    def test_public_generator_reproduces_preserved_json_and_rust_values(self) -> None:
        root = Path(__file__).resolve().parents[2]
        result = subprocess.run([sys.executable, '-m', 'reference.verify_derivatives'],
                                cwd=root, capture_output=True, text=True, check=True)
        self.assertEqual(result.stdout, (root/'fixtures/reference/derivatives.json').read_text())
        data = object_value(decode(result.stdout))
        rows = [object_value(row) for row in array_value(data['samples'])]
        lines = ['\t'.join([string_value(row['sample']),
                            *(string_value(v) for v in array_value(row['coordinates'])),
                            *(string_value(v) for v in array_value(row['values_120']))]) for row in rows]
        path = root/'crates/nsbu-benchmarks/tests/fixtures/derivatives.tsv'
        self.assertEqual('\n'.join(lines)+'\n', path.read_text())

    def test_tensor_entries_agree_with_independent_scalar_differentiation(self) -> None:
        coordinates = ('1/32', '-1/64', '1/10', '1/1024')
        with mp.workdps(80):
            point = quadruple(rational(value) for value in coordinates)
            values = dict(zip(FIELDS, evaluate(coordinates), strict=True))

            def velocity(*p: mpf) -> mpf:
                return fields(*quadruple(p))[0][2]

            def pressure(*p: mpf) -> mpf:
                return fields(*quadruple(p))[1]

            for key, expected in (
                ('gradient_2_0', mp.diff(velocity, point, (1, 0, 0, 0))),
                ('hessian_2_0_2', mp.diff(velocity, point, (1, 0, 1, 0))),
                ('hessian_2_2_2', mp.diff(velocity, point, (0, 0, 2, 0))),
                ('pressure_gradient_1', mp.diff(pressure, point, (0, 1, 0, 0))),
            ):
                self.assertLess(abs(values[key]-expected)/max(mp.mpf(1), abs(expected)), mp.mpf('1e-60'))

    def test_rest_and_outer_flat_region_have_all_zero_derivatives(self) -> None:
        with mp.workdps(80):
            for point in [('1/32', '1/64', '1/10', '0'), ('1/2', '0', '0', '1/256')]:
                values = evaluate(point)
                self.assertEqual(len(values), 46)
                self.assertTrue(all(value == 0 for value in values))

    def test_precision_failure_and_shape_mismatch_cannot_be_hidden(self) -> None:
        self.assertEqual(precision_change((mp.mpf(0),), (mp.mpf(0),)), '0.0')
        with self.assertRaises(ArithmeticError):
            precision_change((mp.mpf(1),), (mp.mpf(2),))
        with self.assertRaises(ValueError):
            precision_change((mp.mpf(0),), ())
        for value in ('nan', 'inf'):
            with self.assertRaises(ArithmeticError):
                precision_change((mp.mpf(0), mp.mpf(value)), (mp.mpf(0), mp.mpf(0)))
