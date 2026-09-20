"""First-step stdout gate, durable h32 bundle validation and completeness."""

import hashlib
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_bundles as bundles  # noqa: E402
from test_h32_contract import FAKE_SOURCE, IDENTITY, plan_template  # noqa: E402,F401
from test_h32_snapshot import COEFF_BYTES, GOOD_COEFF, build_snapshot  # noqa: E402
STATE = hashlib.sha256(GOOD_COEFF).hexdigest()
GOOD_LINE = (
    "attempt=1 clock=32 integration_seconds=900.123456789 rhs_evaluate_seconds=800.0 "
    "rhs_timed_calls=12 outside_rhs_evaluate_seconds=100.1 cache_hit_miss=[7, 5] "
    "observer_seconds=None ratios=[1.5877938231527918e-8, 4.324683895722008e-8] "
    "publication=Step state_sha256=Some(\"" + STATE + "\") steady_allocations=0"
)


class StdoutGateTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()

    def test_accepts_the_good_line(self):
        ok, reason, state = bundles.gate_first_step_line(GOOD_LINE, self.plan)
        self.assertTrue(ok, reason)
        self.assertEqual(state, STATE)

    def test_rejects_ratios_above_local_limit_or_nonfinite(self):
        for ratios in ("[1.5, 0.2]", "[inf, 0.2]", "[nan, 0.2]", "[1e400, 0.2]",
                       "[0.1]", "[0.1, 0.2, 0.3]", "[abc, 0.2]"):
            line = GOOD_LINE.replace("ratios=[1.5877938231527918e-8, 4.324683895722008e-8]",
                                     f"ratios={ratios}")
            ok, reason, _ = bundles.gate_first_step_line(line, self.plan)
            self.assertFalse(ok, ratios)

    def test_rejects_malformed_counters_and_tokens(self):
        mutations = (
            ("attempt=1", "attempt=2"),
            ("clock=32", "clock=64"),
            ("clock=32", "clock=32.5"),
            ("rhs_timed_calls=12", "rhs_timed_calls=11"),
            ("cache_hit_miss=[7, 5]", "cache_hit_miss=[8, 4]"),
            ("steady_allocations=0", "steady_allocations=1"),
            ("publication=Step", "publication=Node"),
            ("integration_seconds=900.123456789", "integration_seconds=1.0e9"),
            ("integration_seconds=900.123456789", "integration_seconds=2000.0"),
            ("integration_seconds=900.123456789", "integration_seconds=900.1s"),
        )
        for old, new in mutations:
            line = GOOD_LINE.replace(old, new) + " rhs_timed_calls=12"
            ok, _reason, _ = bundles.gate_first_step_line(line, self.plan)
            self.assertFalse(ok, f"{old}->{new}")

    def test_duplicate_tokens_refused(self):
        line = GOOD_LINE + " steady_allocations=0"
        ok, reason, _ = bundles.gate_first_step_line(line, self.plan)
        self.assertFalse(ok)
        self.assertIn("duplicated", reason)


def small_plan() -> dict:
    plan = plan_template()
    plan["capture"]["coefficient_bytes"] = COEFF_BYTES
    return plan


def make_bundle(root: Path, plan: dict, mutate=None, state_mutate=None) -> None:
    step = root / "step-001-clock-0032"
    step.mkdir(parents=True)
    attempt = {
        "schema": plan["attempt_schema"], "identity": IDENTITY, "attempt": 1,
        "attempted_from": 0, "attempted_to": 32, "ticks": 32, "outcome": "committed",
        "rhs_calls": 12, "cache_hits": 7, "cache_misses": 5,
        "integration_seconds": 900.123456789, "rhs_evaluate_seconds": 800.0,
        "rhs_timed_calls": 12, "outside_rhs_evaluate_seconds": 100.1,
        "observer_seconds": None, "error_ratio_l2": 1.58e-8,
        "error_ratio_h1": 4.32e-8, "steady_allocations": 0,
    }
    record = {
        "schema": plan["observer_state_schema"], "identity": IDENTITY,
        "resumable": False, "clock": 32, "epoch": 1, "accepted_steps": 1,
        "coefficient_bytes": int(plan["capture"]["coefficient_bytes"]), "state_sha256": STATE,
        "observation_status": "CapturedActualState", "offline_observer_node": False,
        "observer_execution": "offline-baccus-required", "qualification": False,
    }
    if mutate is not None:
        mutate(attempt, record)
    (step / "attempt.json").write_text(json.dumps(attempt, indent=2))
    (step / "record.json").write_text(json.dumps(record, indent=2))
    payload = build_snapshot(IDENTITY, clocks=(32, 4096, 1, 1), coeff=GOOD_COEFF)
    if state_mutate is not None:
        payload = state_mutate(payload)
    (step / "state.bin").write_bytes(payload)
    (root / "rest.json").write_text(json.dumps(
        {"schema": bundles.REST_SCHEMA, "clock": 0, "state_payload": False,
         "observation_status": "RestExact", "balance": "REST"}))


class DurableBundleTests(unittest.TestCase):
    def setUp(self):
        self.plan = small_plan()
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)

    def check(self, mutate=None, state_mutate=None, label="case"):
        bundle_root = self.root / label
        bundle_root.mkdir()
        make_bundle(bundle_root, self.plan, mutate, state_mutate)
        return bundles.crosscheck_durable_bundle(bundle_root, STATE, self.plan)

    def test_accepts_exact_bundle(self):
        ok, reason = self.check()
        self.assertTrue(ok, reason)

    def test_rejects_missing_and_partial_and_tiny_state(self):
        (self.root / "p1").mkdir()
        ok, _ = bundles.crosscheck_durable_bundle(self.root / "p1", STATE, self.plan)
        self.assertFalse(ok)

        def shrink(attempt, record):
            record["coefficient_bytes"] = 12345

        ok, reason = self.check(shrink)
        self.assertIn("coefficient_bytes", reason)

    def test_rejects_malformed_or_mismatched_records(self):
        for mutate, fragment in (
            (lambda a, r: a.update(outcome="rejected"), "outcome"),
            (lambda a, r: a.update(rhs_calls=11), "rhs_calls"),
            (lambda a, r: a.update(cache_misses=6, cache_hits=6), "cache"),
            (lambda a, r: a.update(error_ratio_l2=float("nan")), "error_ratio_l2"),
            (lambda a, r: a.update(error_ratio_h1=1.5), "error_ratio_h1"),
            (lambda a, r: a.update(identity="profile=wrong"), "profile"),
            (lambda a, r: a.update(identity=a["identity"].replace("endpoint=4096",
                                                                  "endpoint=40960")), "endpoint"),
            (lambda a, r: r.update(identity=r["identity"].replace(
                "endpoint=4096;", "endpoint=40960;")), "endpoint"),
            (lambda a, r: r.update(identity=r["identity"].replace(
                f"source={FAKE_SOURCE}", "source=0" * 32)), "source"),
            (lambda a, r: r.update(epoch=2), "epoch"),
            (lambda a, r: r.update(resumable=True), "resumable"),
            (lambda a, r: r.update(qualification=True), "qualification"),
            (lambda a, r: r.update(state_sha256="ff" * 32), "state_sha"),
            (lambda a, r: r.update(schema="wrong"), "schema"),
        ):
            root = self.root / f"case-{fragment}-{id(mutate)}"
            root.mkdir()
            make_bundle(root, self.plan, mutate)
            ok, reason = bundles.crosscheck_durable_bundle(root, STATE, self.plan)
            self.assertFalse(ok, fragment)
            self.assertIn(fragment, reason)

    def test_attempt_and_record_identities_must_be_identical(self):
        def diverge(attempt, record):
            record["identity"] = attempt["identity"] + ";case=" + "f" * 64 + "0"

        ok, reason = self.check(diverge, label="diverge")
        self.assertFalse(ok)
        self.assertIn("identity", reason)

    def test_state_payload_corruption_is_rejected(self):
        def flip(raw):
            index = 200
            return raw[:index] + bytes([raw[index] ^ 0xFF]) + raw[index + 1:]

        ok, reason = self.check(state_mutate=flip, label="flip")
        self.assertFalse(ok)
        self.assertIn("state_payload_invalid", reason)

    def test_state_payload_truncation_and_padding_are_rejected(self):
        for name, mutate in (("trunc", lambda raw: raw[:-1]),
                             ("pad", lambda raw: raw + b"\x00" * 512)):
            ok, reason = self.check(state_mutate=mutate, label=name)
            self.assertFalse(ok, name)
            self.assertIn("state_payload_invalid", reason)
            self.assertIn("snapshot_size_mismatch", reason)

    def test_state_payload_sparse_hole_is_rejected(self):
        root = self.root / "sparse"
        root.mkdir()
        make_bundle(root, self.plan)
        state = root / "step-001-clock-0032" / "state.bin"
        full = state.stat().st_size
        os.truncate(state, 0)
        os.truncate(state, full)  # extends back to size with a pure hole: zero blocks
        ok, reason = bundles.crosscheck_durable_bundle(root, STATE, self.plan)
        self.assertFalse(ok)
        self.assertIn("state_payload_invalid", reason)

    def test_duplicate_json_keys_are_malformed(self):
        root = self.root / "dup"
        root.mkdir()
        make_bundle(root, self.plan)
        path = root / "step-001-clock-0032" / "attempt.json"
        text = path.read_text().replace('"attempt": 1', '"attempt": 1, "attempt": 1', 1)
        path.write_text(text)
        ok, reason = bundles.crosscheck_durable_bundle(root, STATE, self.plan)
        self.assertFalse(ok)
        self.assertIn("duplicate_key", reason)

    def test_partial_sibling_is_refused(self):
        root = self.root / "partial"
        root.mkdir()
        make_bundle(root, self.plan)
        (root / "step-001-clock-0032.partial").mkdir()
        ok, reason = bundles.crosscheck_durable_bundle(root, STATE, self.plan)
        self.assertFalse(ok)
        self.assertIn("partial", reason)

    def test_state_size_must_be_exact(self):
        root = self.root / "state"
        root.mkdir()
        make_bundle(root, self.plan)
        state = root / "step-001-clock-0032" / "state.bin"
        with state.open("r+b") as stream:
            stream.truncate(state.stat().st_size + 4096)
        ok, reason = bundles.crosscheck_durable_bundle(root, STATE, self.plan)
        self.assertFalse(ok)
        self.assertIn("snapshot_size_mismatch", reason)


class CompletenessTests(unittest.TestCase):
    def test_expected_names_and_set(self):
        plan = plan_template()
        names = bundles.expected_capture_names(plan)
        self.assertEqual(len(names), 96)
        self.assertEqual(names[0], "step-001-clock-0032")
        self.assertEqual(names[63], "step-064-clock-2048")
        self.assertEqual(names[64], "step-065-clock-2112")
        self.assertEqual(names[95], "step-096-clock-4096")
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            for bundle_name in names:
                (root / bundle_name).mkdir()
                (root / bundle_name / "state.bin").write_bytes(b"x")
                (root / bundle_name / "record.json").write_bytes(b"{}")
                (root / bundle_name / "attempt.json").write_bytes(b"{}")
            self.assertTrue(bundles.complete_capture_set(root, plan))
            self.assertEqual(bundles.count_committed_capture_dirs(root), 96)
            (root / "step-096-clock-04096.partial").mkdir()
            self.assertFalse(bundles.complete_capture_set(root, plan))
        with tempfile.TemporaryDirectory() as name:
            self.assertFalse(bundles.complete_capture_set(Path(name), plan))

    def test_terminal_line(self):
        self.assertTrue(bundles.terminal_reached(
            "x\nterminal endpoint_capture_complete_offline_observer_required clock=4096\n"))
        self.assertFalse(bundles.terminal_reached("nothing"))


if __name__ == "__main__":
    unittest.main()
