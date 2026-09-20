"""Create-only receipt durability and the per-attempt uncertainty ledger."""

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_receipts as receipts  # noqa: E402


class CreateOnlyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)

    def test_bytes_and_json_are_never_overwritten(self):
        path = self.root / "release.token"
        self.assertEqual(receipts.create_bytes(path, b"first\n", 0o600), receipts.WRITTEN)
        self.assertEqual(receipts.create_bytes(path, b"second\n"), receipts.UNCERTAIN)
        self.assertEqual(path.read_bytes(), b"first\n")
        target = self.root / "launch-receipt.json"
        self.assertEqual(receipts.create_json(target, {"a": 1}), receipts.WRITTEN)
        self.assertEqual(receipts.create_json(target, {"a": 2}), receipts.UNCERTAIN)
        self.assertEqual(json.loads(target.read_text())["a"], 1)

    def test_written_proves_durable_content_and_directory(self):
        path = self.root / "nested" / "deep-receipt.json"
        path.parent.mkdir()
        self.assertEqual(receipts.create_json(path, {"ok": True}), receipts.WRITTEN)
        self.assertTrue(path.is_file())
        self.assertTrue(json.loads(path.read_text())["ok"])

    def test_error_on_unreachable_parent_is_reported_not_swallowed(self):
        status = receipts.create_bytes(self.root / "absent-dir" / "x.json", b"{}")
        self.assertTrue(status.startswith("error("))


class ReceiptLedgerTests(unittest.TestCase):
    def setUp(self):
        self.ledger = receipts.ReceiptLedger()

    def test_clean_ledger_permits_release(self):
        self.ledger.record("launch-receipt.json", receipts.WRITTEN)
        self.ledger.record("release.token", receipts.WRITTEN)
        self.assertIsNone(self.ledger.refuse_reason())

    def test_any_uncertain_write_blocks_every_later_token(self):
        self.ledger.record("decision-receipt.json", receipts.UNCERTAIN)
        reason = self.ledger.refuse_reason()
        self.assertIsNotNone(reason)
        self.assertIn("decision-receipt.json", reason)

    def test_error_statuses_also_count_as_uncertainty(self):
        self.ledger.record("completion-receipt.json", "error(enospc)")
        self.assertIn("completion-receipt.json", self.ledger.uncertain)


class DirectoryFsyncTests(unittest.TestCase):
    def test_directory_fsync_targets_the_containing_directory(self):
        with tempfile.TemporaryDirectory() as name:
            opened = []
            original = os.open

            def spy(path, flags, *args, **kwargs):
                opened.append((path, flags))
                return original(path, flags, *args, **kwargs)

            os.open = spy
            try:
                receipts.create_json(Path(name) / "r.json", {"a": 1})
            finally:
                os.open = original
        dirs = [path for path, flags in opened
                if not str(path).endswith(".json") and flags & getattr(os, "O_DIRECTORY", 0)]
        self.assertEqual(dirs, [name])


if __name__ == "__main__":
    unittest.main()
