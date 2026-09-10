"""Contract tests for the language-neutral CRAP report generator."""
import json
import subprocess
import sys
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


class CrapCliTests(unittest.TestCase):
    """Exercise the module entry point under coverage.py's subprocess patch."""

    def run_cli(self, directory: Path, language: str, metrics: str,
                coverage: str) -> tuple[subprocess.CompletedProcess[str], Path]:
        """Run a complete CLI invocation with isolated evidence files."""
        metrics_path = directory / "metrics.json"
        coverage_path = directory / "coverage.json"
        output_path = directory / "report.json"
        metrics_path.write_text(metrics)
        coverage_path.write_text(coverage)
        completed = subprocess.run(
            [sys.executable, "-m", "quality.check_crap", language,
             str(metrics_path), str(coverage_path), str(output_path)],
            cwd=Path(__file__).parents[2], check=False, capture_output=True,
            text=True,
        )
        return completed, output_path

    def test_python_cli_writes_a_passing_report(self) -> None:
        """A branchless observed function produces a successful JSON report."""
        with tempfile.TemporaryDirectory() as temporary:
            completed, output = self.run_cli(
                Path(temporary), "python",
                '{"sample.py":[{"type":"function","name":"ok","lineno":1,'
                '"endline":2,"complexity":2,"methods":[],"closures":[]}]}',
                '{"files":{"sample.py":{"executed_branches":[],"missing_branches":[],'
                '"executed_lines":[1,2]}}}',
            )
            report = crap.obj(parse(output.read_text()))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(crap.number(report["maximum"]), 2)
        self.assertEqual(crap.obj(crap.seq(report["functions"])[0])["function"], "ok")

    def test_rust_cli_writes_report_before_failing_crap_gate(self) -> None:
        """The threshold failure still leaves its report available to the caller."""
        with tempfile.TemporaryDirectory() as temporary:
            completed, output = self.run_cli(
                Path(temporary), "rust",
                '{"name":"src/sample.rs","kind":"unit","spaces":[{"name":"bad",'
                '"kind":"function","start_line":1,"end_line":3,"spaces":[],'
                '"metrics":{"cyclomatic":{"sum":5}}}]}\n',
                '{"data":[{"files":[{"filename":"/tmp/src/sample.rs",'
                '"branches":[],"segments":[]}]}]}',
            )
            report = crap.obj(parse(output.read_text()))
        self.assertEqual(completed.returncode, 1)
        self.assertIn("per-function CRAP must be <25", completed.stderr)
        self.assertEqual(crap.number(report["maximum"]), 30)
        self.assertEqual(crap.obj(crap.seq(report["functions"])[0])["function"], "bad")

    def test_cli_rejects_empty_and_malformed_evidence(self) -> None:
        """No report is emitted when evidence cannot yield valid measurements."""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            empty, empty_output = self.run_cli(directory, "python", "{}", '{"files":{}}')
            empty_output_exists = empty_output.exists()
            malformed, malformed_output = self.run_cli(
                directory, "python", "[]", '{"files":{}}')
            malformed_output_exists = malformed_output.exists()
        self.assertNotEqual(empty.returncode, 0)
        self.assertIn("no functions measured", empty.stderr)
        self.assertFalse(empty_output_exists)
        self.assertNotEqual(malformed.returncode, 0)
        self.assertIn("expected object", malformed.stderr)
        self.assertFalse(malformed_output_exists)

    def test_cli_rejects_an_invalid_language_before_writing(self) -> None:
        """argparse owns language validation and does not create a report."""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            metrics = directory / "metrics.json"
            coverage = directory / "coverage.json"
            output = directory / "report.json"
            metrics.write_text("{}")
            coverage.write_text('{"files":{}}')
            completed = subprocess.run(
                [sys.executable, "-m", "quality.check_crap", "go", str(metrics),
                 str(coverage), str(output)],
                cwd=Path(__file__).parents[2], check=False, capture_output=True,
                text=True,
            )
            output_exists = output.exists()
        self.assertEqual(completed.returncode, 2)
        self.assertIn("invalid choice", completed.stderr)
        self.assertFalse(output_exists)


if __name__ == "__main__":
    unittest.main()
