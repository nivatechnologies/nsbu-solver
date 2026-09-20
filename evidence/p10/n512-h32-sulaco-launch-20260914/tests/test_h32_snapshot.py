"""Exact snapshot/identity validation; payload layout mirrors artifact.rs.

The synthetic payloads replicate ``artifact/snapshot.rs::write_snapshot``
byte-for-byte (magic, u64 LE identity length, identity, four u128 LE clock
words elapsed/target/epoch/accepted_steps, coefficients, trailing sha256 over
the coefficients only).  The fixture tests consume the real Rust-writer
output written WITH THE REAL STAGED PRODUCTION PROFILE IDENTITY and validate
that identity against the frozen identity document.
"""

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_snapshot as snap  # noqa: E402
from test_h32_contract import PROFILE_IDENTITY, plan_template  # noqa: E402

PLAN = plan_template()
PACKET = Path(__file__).resolve().parents[1]
FIXTURE = PACKET / "tests" / "fixtures" / "n512-h32-snapshot-minimal.bin"
LAYOUT = PACKET / "tests" / "fixtures" / "snapshot-layout.json"
IDENTITY_DOC = PACKET / "docs" / "identity-expected.json"
MISSING = ("fixture missing: run the Rust fixture test first "
           "(artifact::tests::real_writer_emits_the_minimal_h32_snapshot_fixture "
           "with NSBU_SNAPSHOT_FIXTURE_OUT / NSBU_SNAPSHOT_LAYOUT_OUT / "
           "NSBU_IDENTITY_DOC_OUT)")
COEFF_BYTES = 4096
CLOCKS = (32, 4096, 1, 1)
GOOD_COEFF = bytes((index % 253) + 1 for index in range(COEFF_BYTES))
STATE = hashlib.sha256(GOOD_COEFF).hexdigest()


def build_snapshot(identity: str, clocks=CLOCKS, coeff: bytes = GOOD_COEFF) -> bytes:
    identity_bytes = identity.encode("utf-8")
    raw = snap.MAGIC + len(identity_bytes).to_bytes(8, "little") + identity_bytes
    for word in clocks:
        raw += int(word).to_bytes(16, "little")
    raw += coeff + hashlib.sha256(coeff).digest()
    return raw


def layout_expectations(layout: dict, raw: bytes | None = None) -> tuple[str, tuple, int, str]:
    if "fields" in layout:
        by_name = {field["name"]: field for field in layout["fields"]}
        clock_doc = layout["fixture_state"]["clock_fields"]
        clocks = tuple(int(clock_doc[key]) for key in
                       ("elapsed_ticks", "target_ticks", "epoch", "accepted_steps"))
        identity_field = by_name["identity"]
        payload = by_name["coefficient_payload"]
        if raw is None:
            raise AssertionError("unusable layout document")
        blob = raw[identity_field["offset"]:identity_field["offset"] + identity_field["length"]]
        if len(blob) != identity_field["length"] or b"\x00" in blob:
            raise AssertionError(MISSING)
        identity = blob.decode("utf-8")
        body = raw[payload["offset"]:payload["offset"] + payload["length"]]
        if len(body) != payload["length"]:
            raise AssertionError(MISSING)
        return identity, clocks, payload["length"], hashlib.sha256(body).hexdigest()
    identity = layout.get("identity", layout.get("identity_string", ""))
    clocks = layout.get("clocks", layout.get("clock_fields", layout.get("clocks_u128")))
    if isinstance(clocks, dict):
        clocks = [clocks.get(key) for key in ("elapsed", "target", "epoch", "accepted_steps")]
    clocks = tuple(int(str(word)) for word in clocks)
    coefficient = layout.get("coefficient_bytes", layout.get("coefficient_byte_count"))
    state = layout.get("state_sha256", layout.get("state"))
    return str(identity), clocks, int(coefficient), str(state)


def fixture_expectations(raw: bytes) -> tuple[str, tuple, int, str]:
    """The documented snapshot-layout.json (written by the Rust fixture test)
    is the only accepted source of expectations; without it the run must fail
    loudly with a pointer to the Rust writer test."""
    if not LAYOUT.is_file():
        raise AssertionError(MISSING)
    return layout_expectations(json.loads(LAYOUT.read_text()), raw)


def production_plan(identity_map: dict) -> dict:
    plan = plan_template()
    plan["identity_source"] = identity_map["source"]
    plan["profile"] = identity_map["profile"]
    plan["schedule_identity"] = identity_map["schedule"]
    plan["attempt_schema"] = identity_map["attempt_schema"]
    plan["observer_state_schema"] = identity_map["schema"]
    plan["profile_identity"] = dict(identity_map)
    return plan


class IdentityFieldTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()
        self.base = snap.identity_text_from_map(PROFILE_IDENTITY)

    def check(self, text, plan=None):
        return snap.validate_identity_fields(text, plan or self.plan)

    def test_full_writer_identity_with_every_legitimate_field_accepts(self):
        self.assertEqual(self.check(self.base), [])

    def test_endpoint40960_rejected_by_endpoint4096_field(self):
        reasons = self.check(self.base.replace(";endpoint=4096;", ";endpoint=40960;"))
        self.assertTrue(any("endpoint" in reason for reason in reasons))
        reasons = self.check(self.base.replace(";endpoint=4096;", ";endpoint4096;"))
        self.assertTrue(any("malformed" in reason for reason in reasons))
        reasons = self.check(self.base.replace(";endpoint=4096;", ";endpoint=4096x;"))
        self.assertTrue(any("endpoint" in reason for reason in reasons))

    def test_duplicated_missing_and_unexpected_fields_refused(self):
        reasons = self.check(self.base + ";endpoint=4096")
        self.assertTrue(any("duplicated" in reason for reason in reasons))
        reasons = self.check(self.base.replace(";method=cox-matthews;", ";"))
        self.assertTrue(any("missing" in reason for reason in reasons))
        reasons = self.check(self.base + ";bogus=1")
        self.assertTrue(any("unexpected" in reason for reason in reasons))
        reasons = self.check(self.base.replace(";external_stop=pgid-watchdog-v3",
                                               ";external_stop=X"))
        self.assertTrue(any("external_stop" in reason for reason in reasons))

    def test_plan_without_frozen_map_is_refused(self):
        plan = plan_template()
        plan.pop("profile_identity")
        reasons = self.check(self.base, plan)
        self.assertTrue(any("plan_profile_identity_missing" in reason for reason in reasons))

    def test_map_inconsistent_with_plan_identity_is_refused(self):
        plan = plan_template()
        plan["profile_identity"]["profile"] = "drifted"
        reasons = self.check(self.base, plan)
        self.assertTrue(any("plan_profile_identity_inconsistent" in r for r in reasons))


class SyntheticPayloadTests(unittest.TestCase):
    def validate(self, raw, **over):
        kwargs = dict(identity="identity-a", clocks=CLOCKS,
                      coefficient_bytes=COEFF_BYTES, state_sha256=STATE)
        kwargs.update(over)
        return snap.validate_snapshot_bytes(raw, **kwargs)

    def test_accepts_writer_replica(self):
        self.assertEqual(self.validate(build_snapshot("identity-a")), [])

    def test_rejects_truncated_padded_and_magic_corrupt(self):
        raw = build_snapshot("identity-a")
        self.assertTrue(any("size_mismatch" in r for r in self.validate(raw[:-1])))
        self.assertTrue(any("size_mismatch" in r for r in self.validate(raw + b"\x00")))
        self.assertTrue(self.validate(b"X" + raw[1:])[0].startswith("snapshot_magic"))

    def test_rejects_wrong_identity_and_clock_fields(self):
        self.assertTrue(any("identity" in r for r in
                            self.validate(build_snapshot("identity-a"), identity="identity-b")))
        bad = self.validate(build_snapshot("identity-a", clocks=(64, 4096, 1, 1)))
        self.assertTrue(any("elapsed" in r for r in bad))
        bad = self.validate(build_snapshot("identity-a", clocks=(32, 40960, 1, 1)))
        self.assertTrue(any("target" in r for r in bad))
        dup = self.validate(build_snapshot("identity-a", clocks=(32, 32, 1, 1)))
        self.assertTrue(any("elapsed_equals_target" in r for r in dup))

    def test_rejects_payload_corruption_and_digest_swap(self):
        raw = build_snapshot("identity-a")
        cut = len(raw) - snap.DIGEST_BYTES
        payload_off = snap.IDENTITY_OFFSET + len("identity-a") + snap.CLOCK_BLOCK_BYTES + 3
        flipped = raw[:payload_off] + bytes([raw[payload_off] ^ 0xFF]) + raw[payload_off + 1:]
        reasons = self.validate(flipped)
        self.assertTrue(any("digest_mismatch" in r for r in reasons))
        self.assertTrue(any("sha_mismatch" in r for r in reasons))
        assert cut > 0
        swapped = raw[:cut] + bytes(snap.DIGEST_BYTES)
        self.assertTrue(any("digest_mismatch" in r for r in self.validate(swapped)))

    def test_rejects_all_zero_and_nul_run_payloads(self):
        zeros = build_snapshot("identity-a", coeff=bytes(COEFF_BYTES))
        self.assertTrue(any("all_zero" in r for r in self.validate(zeros)))
        sparse = bytearray(GOOD_COEFF)
        sparse[1000:1600] = bytes(600)
        reasons = self.validate(build_snapshot("identity-a", coeff=bytes(sparse)),
                                maximum_nul_run=512)
        self.assertTrue(any("nul_run" in r for r in reasons))

    def test_file_level_rejects_truncation_padding_and_sparse_holes(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            good = root / "good.bin"
            good.write_bytes(build_snapshot("identity-a"))
            kwargs = dict(identity="identity-a", clocks=CLOCKS,
                          coefficient_bytes=COEFF_BYTES, state_sha256=STATE)
            self.assertEqual(snap.validate_snapshot_file(good, **kwargs), [])
            padded = root / "padded.bin"
            padded.write_bytes(build_snapshot("identity-a") + b"\x00" * 512)
            self.assertTrue(any("size_mismatch" in r
                                for r in snap.validate_snapshot_file(padded, **kwargs)))
            sparse = root / "sparse.bin"
            with sparse.open("wb") as stream:
                stream.truncate(snap.expected_snapshot_size("identity-a", COEFF_BYTES))
            reasons = snap.validate_snapshot_file(sparse, **kwargs)
            self.assertTrue(any("sparse_hole" in r or "all_zero" in r for r in reasons), reasons)
            truncated = root / "trunc.bin"
            truncated.write_bytes(build_snapshot("identity-a")[:-4096])
            self.assertTrue(any("size_mismatch" in r
                                for r in snap.validate_snapshot_file(truncated, **kwargs)))


class RustWriterFixtureTests(unittest.TestCase):
    """Consume the REAL Rust writer output written with the REAL staged
    production profile identity; loud failure until it lands."""

    def setUp(self):
        if not FIXTURE.is_file() or not LAYOUT.is_file() or not IDENTITY_DOC.is_file():
            self.fail(MISSING)
        self.raw = FIXTURE.read_bytes()
        self.identity, self.clocks, self.coeff, self.state = fixture_expectations(self.raw)
        self.layout = json.loads(LAYOUT.read_text())
        self.document = json.loads(IDENTITY_DOC.read_text())
        self.fields = {key: value for key, value in self.document["fields"]}

    def test_accepts_the_real_rust_written_tiny_fixture(self):
        reasons = snap.validate_snapshot_bytes(
            self.raw, identity=self.identity, clocks=self.clocks,
            coefficient_bytes=self.coeff, state_sha256=self.state)
        self.assertEqual(reasons, [])
        self.assertEqual(self.identity, self.document["identity_text"])
        self.assertEqual(self.identity, self.layout["identity_text"])
        self.assertEqual(len(self.identity), self.document["identity_len"])

    def test_production_identity_validates_against_the_frozen_map(self):
        plan = production_plan(self.fields)
        self.assertEqual(snap.validate_identity_fields(self.identity, plan), [])

    def test_production_identity_adversarial_drift_is_refused(self):
        plan = production_plan(self.fields)
        for key, value in self.fields.items():
            marker = f";{key}={value}"
            if marker in self.identity:
                drifted = self.identity.replace(marker, marker + "0", 1)
            else:  # first field: the whole identity starts with key=value
                drifted = self.identity.replace(f"{key}={value}", f"{key}={value}0", 1)
            self.assertNotEqual(drifted, self.identity, f"{key} drift was a no-op")
            reasons = snap.validate_identity_fields(drifted, plan)
            self.assertTrue(reasons, f"drifted {key} accepted")
        reasons = snap.validate_identity_fields(self.identity + f";{next(iter(self.fields))}"
                                                           f"={next(iter(self.fields.values()))}",
                                                plan)
        self.assertTrue(any("duplicated" in reason for reason in reasons))
        reasons = snap.validate_identity_fields(self.identity + ";trojan=1", plan)
        self.assertTrue(any("unexpected" in reason for reason in reasons))

    def test_rejects_corrupted_truncated_padded_and_wrong_identity_fixtures(self):
        base = dict(identity=self.identity, clocks=self.clocks,
                    coefficient_bytes=self.coeff, state_sha256=self.state)
        offset = snap.IDENTITY_OFFSET + len(self.identity.encode()) + snap.CLOCK_BLOCK_BYTES + 1
        corrupted = self.raw[:offset] + bytes([self.raw[offset] ^ 0xFF]) + self.raw[offset + 1:]
        self.assertTrue(snap.validate_snapshot_bytes(corrupted, **base))
        self.assertTrue(snap.validate_snapshot_bytes(self.raw[:-1], **base))
        self.assertTrue(snap.validate_snapshot_bytes(self.raw + b"\x00", **base))
        self.assertTrue(snap.validate_snapshot_bytes(self.raw, **{**base,
                                                                  "identity": "wrong-identity"}))
        self.assertTrue(snap.validate_snapshot_bytes(self.raw, **{**base,
                                                                  "state_sha256": "f" * 64}))


class ExpectedIdentityMapPlanTests(unittest.TestCase):
    """The frozen plan's profile_identity must be COMPLETE, ORDERED and
    plan-linked; malformed or reordered maps are refused, absent plan links
    are skipped (never invented)."""

    def test_malformed_map_entry_is_refused(self):
        plan = plan_template()
        broken = dict(PROFILE_IDENTITY)
        broken["backend"] = "rustfft;6.4.1"
        plan["profile_identity"] = broken
        _fields, reasons = snap.expected_identity_map(plan)
        self.assertTrue(any("malformed" in reason for reason in reasons))

    def test_same_keyset_different_order_is_order_drift(self):
        plan = plan_template()
        keys = list(PROFILE_IDENTITY)
        reordered = keys[1:] + keys[:1]
        plan["profile_identity"] = {k: PROFILE_IDENTITY[k] for k in reordered}
        _fields, reasons = snap.expected_identity_map(plan)
        self.assertTrue(any("key_order_drift" in reason for reason in reasons),
                        reasons)

    def test_absent_plan_link_is_skipped_not_inconsistent(self):
        plan = plan_template()
        plan.pop("identity_source")
        fields, reasons = snap.expected_identity_map(plan)
        self.assertEqual(fields, dict(PROFILE_IDENTITY))
        self.assertEqual(reasons, [])


if __name__ == "__main__":
    unittest.main()
