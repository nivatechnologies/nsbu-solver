"""Contract tests for the language-neutral CRAP report generator."""
import json
import tempfile
import unittest
from pathlib import Path
from typing import cast

from quality import check_crap as crap


def json_value(value: object) -> crap.Json:
    """Give fixture literals the same recursively JSON-safe shape as parsed input."""
    return cast(crap.Json, json.loads(json.dumps(value)))


def parse(value: str) -> crap.Json:
    """Parse a compact fixture without partially inferred nested literals."""
    return cast(crap.Json, json.loads(value))


class CrapMeasurementTests(unittest.TestCase):
    """Keep report parsing and coverage attribution independently executable."""

    def test_primitives_reject_invalid_measurements(self) -> None:
        self.assertEqual(crap.fraction(0, 0), 1)
        self.assertEqual(crap.score(2, 1), 2)
        with self.assertRaises(ValueError):
            crap.fraction(2, 1)
        with self.assertRaises(ValueError):
            crap.score(0, 1)
        with self.assertRaises(ValueError):
            crap.number(True)
        with self.assertRaises(ValueError):
            crap.obj([])
        with self.assertRaises(ValueError):
            crap.seq({})

    def test_python_report_uses_branch_origins_and_body_execution(self) -> None:
        metrics = parse('''{"sample.py":[
          {"type":"function","name":"branched","lineno":1,"endline":4,"complexity":3,"methods":[],"closures":[]},
          {"type":"class","name":"Box","lineno":6,"endline":12,"complexity":1,"methods":[
            {"type":"method","name":"body","lineno":7,"endline":9,"complexity":1,"methods":[],"closures":[]}
          ],"closures":[]}
        ]}''')
        coverage = json_value({"files": {"sample.py": {
            "executed_branches": [[2, 3]], "missing_branches": [[2, 4]],
            "executed_lines": [1, 8],
        }}})
        rows = crap.python_report(metrics, coverage)
        self.assertEqual([row["function"] for row in rows], ["branched", "body"])
        self.assertEqual(rows[0]["branch_coverage_fraction"], 0.5)
        self.assertEqual(rows[1]["branch_coverage_fraction"], 1)

    def test_rust_coverage_merges_branch_coordinates_and_uses_segments(self) -> None:
        branched = crap.obj(json_value({
            "branches": [[2, 1, 2, 3, 1, 0], [2, 1, 2, 3, 0, 1]], "segments": [],
        }))
        self.assertEqual(crap.rust_coverage(branched, 1, 3), 1)
        branchless = crap.obj(json_value({
            "branches": [], "segments": [[2, 1, 1, True, False, False]],
        }))
        self.assertEqual(crap.rust_coverage(branchless, 1, 3), 1)

    def test_rust_report_matches_one_source_file(self) -> None:
        unit = parse('''{"name":"src/sample.rs","kind":"unit","spaces":[
          {"name":"sample","kind":"function","start_line":1,"end_line":4,
           "spaces":[],"metrics":{"cyclomatic":{"sum":2}}}
        ]}''')
        coverage = json_value({"data": [{"files": [{"filename": "/tmp/src/sample.rs",
            "branches": [[2, 1, 2, 3, 1, 1]], "segments": []}]}]})
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "metrics.json"
            report.write_text(json.dumps(unit) + "\n")
            rows = crap.rust_report(report, coverage)
        self.assertEqual(rows[0]["function"], "sample")
        self.assertEqual(rows[0]["crap"], 2)


if __name__ == "__main__":
    unittest.main()
