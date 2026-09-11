"""Fast contracts for the bounded current-grid arithmetic pilot."""
from contextlib import redirect_stdout
from dataclasses import replace
import io
import json
from mpmath import mp, mpf
from pathlib import Path
import unittest
from unittest.mock import patch
from reference.reference_arithmetic_full import bits
from reference.reference_arithmetic_pilot import (
    CLOCKS, POINTS, RustRow, category, centered_numerator, full_grid_counts,
    evaluate_one, parse_row, rational_point, run, tracking_values, validate_rows,
)


def pilot_row(label: str, index: tuple[int, int, int], elapsed: int) -> RustRow:
    """Build one correctly bound synthetic pilot row."""
    argument = tuple(bits(value/12.0) for value in index)
    centered = tuple(bits(value/12.0-1.0 if value >= 6 else value/12.0) for value in index)
    time = bits(float(elapsed)*2.0**-20)
    text = "\t".join(("ARITH_PILOT", label, ",".join(map(str, index)), str(elapsed),
                       category(index, elapsed), ",".join(argument), ",".join(centered), time,
                       ",".join(["0000000000000000"]*42)))
    return parse_row(text)


def fake_evaluate(row: RustRow, _binding: str, precision: int) -> tuple[tuple[mpf, ...], int, bool]:
    """Return bounded vectors for orchestration without repeating numerical work."""
    return (mp.mpf(0),)*42, 0, row.category == "active" or precision == 120


class ReferenceArithmeticPilot(unittest.TestCase):
    """Keep exact input classes and corrected full-grid counts stable."""

    def test_centering_and_representative_classes_are_exact(self) -> None:
        self.assertEqual(tuple(centered_numerator(value) for value in range(12)),
                         (0, 1, 2, 3, 4, 5, -6, -5, -4, -3, -2, -1))
        self.assertEqual(category((0, 0, 1), 64), "active")
        self.assertEqual(category((5, 0, 0), 64), "active")
        self.assertEqual(category((1, 1, 1), 64), "active")
        self.assertEqual(category((6, 0, 0), 64), "exterior")
        self.assertEqual(category((6, 0, 0), 0), "startup")

    def test_full_grid_union_counts_overlap_once(self) -> None:
        self.assertEqual(full_grid_counts(), {
            "rows": 5184, "exterior_spatial": 1213, "exterior_rows": 3639,
            "startup_rows": 1728, "startup_exterior_intersection": 1213,
            "shortcut_union_rows": 4154, "active_rows": 1030,
        })

    def test_fixed_pilot_orchestration_reaches_a_complete_record(self) -> None:
        rows = [pilot_row(label, index, elapsed)
                for label, index in POINTS for elapsed in CLOCKS]
        stream = io.StringIO()
        with patch("reference.reference_arithmetic_pilot.rust_rows",
                   return_value=(rows, "0"*64)), patch(
                       "reference.reference_arithmetic_pilot.evaluate_one",
                       side_effect=fake_evaluate), redirect_stdout(stream):
            run(Path(__file__))
        records = [json.loads(item) for item in stream.getvalue().splitlines()]
        self.assertEqual(records[-1]["event"], "complete")
        self.assertTrue(records[-1]["input_classes_match"])

    def test_independent_tracking_vector_has_all_42_exact_startup_zeros(self) -> None:
        row = pilot_row("axis", (0, 0, 1), 0)
        with mp.workdps(80):
            values, iterations = tracking_values(rational_point(row))
        self.assertEqual(len(values), 42)
        self.assertTrue(all(value == 0 for value in values))
        self.assertGreater(iterations, 0)

    def test_malformed_producer_shape_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            parse_row("ARITH_PILOT\tshort")
        zeros = ",".join(["0"]*42)
        with self.assertRaises(ValueError):
            parse_row(f"ARITH_PILOT\taxis\t0,0,1\t0\tstartup\t0,0,0\t0,0,0\t0\t0")
        with self.assertRaises(ValueError):
            parse_row(f"ARITH_PILOT\taxis\t0,0,1\t0\tstartup\t0,0\t0,0,0\t0\t{zeros}")
        row = pilot_row("axis", (0, 0, 1), 0)
        with self.assertRaises(ArithmeticError):
            evaluate_one(replace(row, category="exterior"), "rational", 80)
        bad_words = ("0000000000000001",)+row.value_bits[1:]
        with self.assertRaises(ArithmeticError):
            evaluate_one(replace(row, value_bits=bad_words), "rational", 80)

    def test_fixed_row_set_and_class_counts_cannot_be_spoofed(self) -> None:
        rows = [pilot_row(label, index, elapsed)
                for label, index in POINTS for elapsed in CLOCKS]
        with self.assertRaises(ValueError):
            validate_rows(rows[:-1])
        with self.assertRaises(ValueError):
            validate_rows([replace(rows[0], category="active"), *rows[1:]])


if __name__ == "__main__":
    unittest.main()
