"""Regression against independently checked 80/120-digit reference artifacts."""
from pathlib import Path
import unittest
from mpmath import mp
from reference.evaluator import evaluate
from reference.scalar import rational
from reference.tuples import quadruple
from reference.verify_fields import SAMPLES
from tools.json_types import array_value, decode, object_value, string_value


class PreservedFixtureTests(unittest.TestCase):
    def test_field_coordinates_and_values_against_verified_artifact(self) -> None:
        path = Path(__file__).resolve().parents[2]/'fixtures/reference/fields.json'
        fixture = object_value(decode(path.read_text()))
        precisions = object_value(fixture['precisions'])
        rows = [object_value(row) for row in array_value(precisions['80'])]
        coordinates = tuple((string_value(row['sample']),quadruple(string_value(v) for v in array_value(row['coordinates']))) for row in rows)
        self.assertEqual(SAMPLES,coordinates)
        with mp.workdps(80):
            for row,(_,point) in zip(rows,coordinates):
                actual = evaluate(*quadruple(rational(v) for v in point))
                for key,values in (('velocity',actual.velocity),('force',actual.force)):
                    expected = [mp.mpf(string_value(v)) for v in array_value(row[key])]
                    scale = max(mp.mpf(1),*(abs(v) for v in expected))
                    self.assertLess(max(abs(a-b) for a,b in zip(values,expected))/scale,mp.mpf('1e-60'))
                expected_gradients = [array_value(v) for v in array_value(row['force_gradient'])]
                for actual_row,expected_row in zip(actual.force_gradient,expected_gradients):
                    expected = [mp.mpf(string_value(v)) for v in expected_row]
                    scale = max(mp.mpf(1),*(abs(v) for v in expected))
                    self.assertLess(max(abs(a-b) for a,b in zip(actual_row,expected))/scale,mp.mpf('1e-60'))
