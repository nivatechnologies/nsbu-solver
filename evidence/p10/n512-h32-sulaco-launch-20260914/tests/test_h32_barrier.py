"""Barrier token derivation, armed receipt authentication and receipts."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_barrier as bar  # noqa: E402
from test_h32_contract import plan_template  # noqa: E402

SECRET = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
NONCE = "0a" * 32
STATE = "0b" * 32


class TokenDerivationTests(unittest.TestCase):
    def test_matches_the_cross_language_vector(self):
        self.assertEqual(
            bar.derive_token(SECRET, "release", NONCE, 32, STATE),
            "cabadcab44f27de96f99372b711e0bb082b73b36be6aeec02ba19aba1e18cd98",
        )
        self.assertEqual(
            bar.derive_token(SECRET, "abort", NONCE, 32, STATE),
            "4c2f9f96a54e99565d589ae8a47d827692f06cc2c1caa1fcdb407804ebd8dcf5",
        )

    def test_every_field_is_bound(self):
        base = bar.derive_token(SECRET, "release", NONCE, 32, STATE)
        self.assertNotEqual(base, bar.derive_token(SECRET, "abort", NONCE, 32, STATE))
        self.assertNotEqual(base, bar.derive_token("f" * 64, "release", NONCE, 32, STATE))
        self.assertNotEqual(base, bar.derive_token(SECRET, "release", "0c" * 32, 32, STATE))
        self.assertNotEqual(base, bar.derive_token(SECRET, "release", NONCE, 64, STATE))
        self.assertNotEqual(base, bar.derive_token(SECRET, "release", NONCE, 32, "0c" * 32))

    def test_malformed_inputs_are_refused(self):
        for call in (
            lambda: bar.derive_token("XYZ", "release", NONCE, 32, STATE),
            lambda: bar.derive_token(SECRET, "release", "AA" * 32, 32, STATE),
            lambda: bar.derive_token(SECRET, "sidestep", NONCE, 32, STATE),
            lambda: bar.derive_token(SECRET, "release", NONCE, 0, STATE),
            lambda: bar.derive_token(SECRET, "release", NONCE, True, STATE),
        ):
            with self.assertRaises(ValueError):
                call()


class ArmedReceiptTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)

    def armed(self, **overrides) -> Path:
        payload = {
            "schema": bar.ARMED_SCHEMA, "attempt": 1, "clock": 32, "nonce": NONCE,
            "state_sha256": STATE, "deadline_epoch": 1789999999, "qualification": False,
        }
        payload.update(overrides)
        path = self.dir / bar.ARMED_RECEIPT
        path.write_text(json.dumps(payload))
        return path

    def test_accepts_matching_handshake(self):
        ok, reason, _armed = bar.read_armed_receipt(
            self.armed(), self.plan, NONCE, SECRET, STATE)
        self.assertTrue(ok, reason)

    def test_refuses_every_divergence(self):
        for overrides, fragment in (
            ({"nonce": "ff" * 32}, "nonce_mismatch"),
            ({"clock": 64}, "clock_mismatch"),
            ({"attempt": 2}, "clock_mismatch"),
            ({"schema": "other"}, "schema_mismatch"),
            ({"qualification": True}, "qualification_not_false"),
            ({"state_sha256": "zz"}, "state_malformed"),
            ({"deadline_epoch": "soon"}, "deadline_malformed"),
        ):
            ok, reason, _ = bar.read_armed_receipt(
                self.armed(**overrides), self.plan, NONCE, SECRET, STATE)
            self.assertFalse(ok, overrides)
            self.assertIn(fragment, reason)

    def test_state_binding_and_missing_file(self):
        ok, reason, _ = bar.read_armed_receipt(
            self.armed(), self.plan, NONCE, SECRET, "c" * 64)
        self.assertFalse(ok)
        self.assertIn("state_mismatch", reason)
        ok, reason, _ = bar.read_armed_receipt(
            self.dir / "absent.json", self.plan, NONCE, SECRET, STATE)
        self.assertFalse(ok)
        self.assertIn("unreadable", reason)


class OneShotProtocolTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)

    def armed_line(self, clock=32, state="a" * 64):
        return ("temporal_barrier armed attempt=1 clock=" + str(clock)
                + " state_sha256=" + state + " deadline=1789999999")

    def test_single_armed_line_at_the_armed_clock_is_clean(self):
        self.assertIsNone(bar.one_shot_violation(self.armed_line() + "\n", 32))
        self.assertIsNone(bar.one_shot_violation("nothing yet\n", 32))

    def test_rearm_or_wrong_clock_is_a_violation(self):
        double = self.armed_line() + "\n" + self.armed_line(64) + "\n"
        self.assertIn("rearmed", bar.one_shot_violation(double, 32))
        late = self.armed_line(64) + "\n"
        self.assertIn("wrong_clock", bar.one_shot_violation(late, 32))

    def test_existing_tokens_are_visible_and_never_issued_twice(self):
        self.assertEqual(bar.existing_tokens(self.dir), [])
        status, _reason = bar.issue_token(self.dir, "release", SECRET, NONCE, 32, STATE)
        self.assertEqual(status, "written")
        self.assertEqual(bar.existing_tokens(self.dir), [bar.RELEASE_FILE])
        status, reason = bar.issue_token(self.dir, "abort", SECRET, NONCE, 32, STATE)
        self.assertEqual(status, "written")
        self.assertEqual(bar.existing_tokens(self.dir), [bar.ABORT_FILE, bar.RELEASE_FILE])
        self.assertEqual((self.dir / bar.RELEASE_FILE).read_text().strip(),
                         bar.derive_token(SECRET, "release", NONCE, 32, STATE))


class IssueAndReceiptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)

    def test_tokens_are_create_only_with_uncertainty(self):
        status, reason = bar.issue_token(self.dir, "release", SECRET, NONCE, 32, STATE)
        self.assertEqual(status, "written")
        self.assertIn("release", reason)
        issued = (self.dir / bar.RELEASE_FILE).read_text().strip()
        self.assertEqual(issued, bar.derive_token(SECRET, "release", NONCE, 32, STATE))
        status, reason = bar.issue_token(self.dir, "release", SECRET, NONCE, 32, STATE)
        self.assertEqual(status, "uncertain_exists")
        self.assertIn("preexists", reason)
        self.assertEqual((self.dir / bar.RELEASE_FILE).read_text().strip(), issued)

    def test_json_receipt_never_overwrites(self):
        path = self.dir / "launch-receipt.json"
        self.assertEqual(bar.write_json_receipt(path, {"a": 1}), "written")
        self.assertEqual(bar.write_json_receipt(path, {"a": 2}), "uncertain_exists")
        self.assertEqual(json.loads(path.read_text())["a"], 1)


if __name__ == "__main__":
    unittest.main()
