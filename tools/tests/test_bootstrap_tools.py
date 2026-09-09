"""Exercise failures that could otherwise admit an incomplete review package."""

from __future__ import annotations

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_repository import check_repository


CHECKOUT = Path(__file__).resolve().parents[2]


class BootstrapTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="nsbu-bootstrap-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "checkout"
        shutil.copytree(CHECKOUT, self.root, ignore=shutil.ignore_patterns(
            ".git", ".venv", "work", "target", "runs", "checkpoints", "__pycache__", ".DS_Store"))

    def assert_check_fails(self, expected: str) -> None:
        report = check_repository(self.root)
        self.assertEqual(report["status"], "failed")
        self.assertIn(expected, json.dumps(report))

    def invoke_runner(self, *args: str, optimized: bool = False) -> subprocess.CompletedProcess:
        command = [sys.executable]
        if optimized:
            command.append("-O")
        command.extend([str(self.root / "tools/verify_design.py"), *args])
        return subprocess.run(command, cwd=self.temporary.name, capture_output=True,
                              text=True, timeout=30)

    def test_complete_checkout_and_independent_cwd(self) -> None:
        command = [sys.executable, str(self.root / "tools/check_repository.py")]
        result = subprocess.run(command, cwd=self.temporary.name, capture_output=True,
                                text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "passed")

    def test_missing_review_input(self) -> None:
        (self.root / "docs/design/CONSTRUCTION_LEDGER.md").unlink()
        self.assert_check_fails("docs/design/CONSTRUCTION_LEDGER.md")

    def test_missing_hidden_workflow(self) -> None:
        (self.root / ".github/workflows/checks.yml").unlink()
        self.assert_check_fails(".github/workflows/checks.yml")

    def test_changed_baseline_bytes(self) -> None:
        with (self.root / "docs/design/COMPLETE_DESIGN.md").open("a", encoding="utf-8") as handle:
            handle.write("\nChanged input.\n")
        self.assert_check_fails("Preserved component differs")

    def test_changed_original_manifest(self) -> None:
        path = self.root / "docs/design/navier-runtime-review-manifest.json"
        path.write_text(path.read_text(encoding="utf-8") + "\n", encoding="utf-8")
        self.assert_check_fails("Original review manifest SHA-256 mismatch")

    def test_changed_benchmark_copy(self) -> None:
        path = self.root / "benchmarks/similarity-mms-v2.json"
        path.write_bytes(path.read_bytes() + b"\n")
        self.assert_check_fails("Benchmark copy differs")

    def test_missing_documentation_target(self) -> None:
        with (self.root / "README.md").open("a", encoding="utf-8") as handle:
            handle.write("\n[Unshipped document](docs/unshipped.md)\n")
        self.assert_check_fails("Missing local Markdown target")

    def test_private_adapter_is_rejected(self) -> None:
        (self.root / "navier-runtime-niva-adapter.md").write_text("private fixture", encoding="utf-8")
        self.assert_check_fails("Private material in public source")

    def test_optimized_runner_refused_and_stale_success_replaced(self) -> None:
        report = self.root / "work/refusal.json"
        report.parent.mkdir()
        report.write_text('{"status":"passed"}', encoding="utf-8")
        result = self.invoke_runner("--output", "work/refusal.json", optimized=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Optimized Python is refused", result.stdout)
        self.assertEqual(json.loads(report.read_text(encoding="utf-8"))["status"], "failed")

    def test_preserved_evidence_cannot_be_overwritten(self) -> None:
        name = "docs/design/navier-runtime-verification-results.json"
        before = (self.root / name).read_bytes()
        result = self.invoke_runner("--output", name)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual((self.root / name).read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
