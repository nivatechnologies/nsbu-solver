"""Frozen Sulaco plan binding and readiness gates."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_contract as contract  # noqa: E402
import h32_bundles as bundles  # noqa: E402


FAKE_SOURCE = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
FAKE_CASE = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"

# Field order mirrors the production writer's format string; the values are
# test fakes.  Extra legitimate writer fields must validate, unknown drift
# must not (see test_h32_snapshot.IdentityFieldTests).
PROFILE_IDENTITY = {
    "source": FAKE_SOURCE,
    "case": FAKE_CASE,
    "profile": "n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995",
    "backend": "rustfft-6.4.1-avx-avx2-fma",
    "library_source": "1ed699568be70dedf72492324be08600d4407c02",
    "prototype_source": "b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5",
    "provider": "parallel-reduced-v2-force-w3-parallel8-attempt-cache",
    "rhs_w3": "layout768-width3-bidirectional-add21812652048",
    "force_w3": "layout512-width3-forward-add4343035792",
    "rhs_timer": "timed-rhs-single-measure",
    "clock": "std-time-Instant",
    "scope": "evaluate-inclusive",
    "overhead": "included",
    "retained": "1024",
    "force_samples": "512",
    "observer_force_samples": "1024",
    "observer_conservative": "1024",
    "observer_execution": "offline-baccus-required",
    "host_provenance": "explicit-reviewed-host-required",
    "sampling_workers": "8",
    "rhs_w3_persistent_callers": "3",
    "rhs_fft_helpers": "8",
    "rhs_fft_total_workers": "11",
    "provider_w3_persistent_callers": "3",
    "provider_fft_helpers": "8",
    "provider_fft_total_workers": "11",
    "method": "cox-matthews",
    "schedule": "h32-clocks0-through2048-then-h64-through4096",
    "endpoint": "4096",
    "maximum_attempts": "96",
    "advective_limit": "3.3",
    "execution_cap": "207627647760",
    "artifact_cap": "549755813888",
    "schema": "p10-avx-n512-m512-h32-observer-state-v1",
    "attempt_schema": "p10-avx-scheduled-attempt-v3",
    "resume": "unsupported",
    "numa": "whole-host-unbound-all-visible-cpus-memory",
    "external_stop": "pgid-watchdog-v3-confirmed-identity-absolute-deadline",
}
IDENTITY = ";".join(f"{key}={value}" for key, value in PROFILE_IDENTITY.items())


def plan_template() -> dict:
    return {
        "schema": contract.PLAN_SCHEMA,
        "host": "sulaco",
        "from_rest": True,
        "resume": "unsupported",
        "qualification": False,
        "source_binding": FAKE_SOURCE,
        "identity_source": FAKE_SOURCE,
        "profile": "n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995",
        "schedule_identity": "h32-clocks0-through2048-then-h64-through4096",
        "attempt_schema": "p10-avx-scheduled-attempt-v3",
        "observer_state_schema": "p10-avx-n512-m512-h32-observer-state-v1",
        "profile_identity": dict(PROFILE_IDENTITY),
        "binary_sha256": "1" * 64,
        "watchdog_sha256": "2" * 64,
        "preflight_sha256": "3" * 64,
        "guard": {
            "first_step_clock": 32,
            "rhs_calls": 12, "cache_hits": 7, "cache_misses": 5,
            "steady_allocations": 0, "maximum_local_error_ratio": 1.0,
            "maximum_first_step_integration_seconds": 1910,
            "first_step_wall_seconds": 2103,
            "absolute_deadline_epoch": 1789999999,
            "minimum_launch_margin_seconds": 3600,
        },
        "capture": {
            "all_committed_states": 96, "epoch": 1,
            "coefficient_bytes": 3233808384,
        },
        "resources": {
            "exact_capture_peak_bytes": 207627647760,
            "memory_floor_bytes": 241987386128,
            "address_space_limit_bytes": 274877906944,
            "disk_bound_bytes": 310453075968,
            "disk_floor_bytes": 344812814336,
            "probe_path": ".",
        },
        "barrier": {"decision_margin_seconds": 3600, "armed_receipt_seconds": 60},
    }


def write_plan(tmp: Path, plan: dict) -> tuple[Path, str]:
    path = tmp / "frozen-plan.json"
    raw = json.dumps(plan, indent=2).encode()
    path.write_bytes(raw)
    return path, contract.sha256_bytes(raw)


class PlanBindingTests(unittest.TestCase):
    def test_parses_the_same_bounded_bytes_it_hashed(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            path, digest = write_plan(tmp, plan_template())
            plan, observed = contract.read_frozen_plan(path, digest)
            self.assertEqual(observed, digest)
            self.assertEqual(plan["host"], "sulaco")

    def test_tampered_byte_and_oversized_plan_are_refused(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            path, digest = write_plan(tmp, plan_template())
            path.write_bytes(path.read_bytes() + b"\n")
            with self.assertRaises(contract.Refusal) as caught:
                contract.read_frozen_plan(path, digest)
            self.assertEqual(caught.exception.code, 67)
            big = tmp / "big.json"
            big.write_bytes(b"{" + b" " * contract.MAX_PLAN_BYTES + b"}")
            with self.assertRaises(contract.Refusal):
                contract.read_frozen_plan(big, "0" * 64)

    def test_mandatory_expectation_must_be_lowercase_hex(self):
        with tempfile.TemporaryDirectory() as name:
            path, _ = write_plan(Path(name), plan_template())
            for bad in ("ABC", "0" * 63, 17, None):
                with self.assertRaises(contract.Refusal):
                    contract.read_frozen_plan(path, bad)

    def test_plan_shape_refuses_non_sulaco_and_weak_memory_floor(self):
        for mutate in (
            lambda p: p.update(host="baccus"),
            lambda p: p.update(from_rest=False),
            lambda p: p.update(qualification=True),
            lambda p: p["resources"].update(memory_floor_bytes=1),
            lambda p: p["guard"].update(first_step_clock=64),
            lambda p: p["capture"].update(all_committed_states=48),
            lambda p: p["guard"].update(absolute_deadline_epoch="tomorrow"),
        ):
            plan = plan_template()
            mutate(plan)
            with self.assertRaises(contract.Refusal):
                contract.validate_plan_shape(plan)

    def test_memory_floor_rule_keeps_exact_measured_minimum(self):
        plan = plan_template()
        plan["resources"]["memory_floor_bytes"] = 241987189520
        contract.validate_plan_shape(plan)  # exact measured floor is allowed
        plan["resources"]["memory_floor_bytes"] = 241987189519
        with self.assertRaises(contract.Refusal):
            contract.validate_plan_shape(plan)

    def test_snapshot_bytes_must_be_derived_from_the_frozen_identity(self):
        import h32_snapshot as snap
        plan = plan_template()
        identity = snap.identity_text_from_map(plan["profile_identity"])
        plan["capture"]["snapshot_bytes"] = snap.expected_snapshot_size(
            identity, plan["capture"]["coefficient_bytes"])
        contract.validate_plan_shape(plan)  # exact derivation is accepted
        plan["capture"]["snapshot_bytes"] += 1
        with self.assertRaises(contract.Refusal):
            contract.validate_plan_shape(plan)

    def test_missing_or_inconsistent_profile_identity_map_is_refused(self):
        for mutate in (
            lambda p: p.pop("profile_identity"),
            lambda p: p["profile_identity"].update(profile="drifted"),
            lambda p: p["profile_identity"].update(bogus_field="1"),
            lambda p: p["profile_identity"].pop("external_stop"),
            lambda p: p["profile_identity"].update(endpoint="40960"),
        ):
            plan = plan_template()
            mutate(plan)
            with self.assertRaises(contract.Refusal):
                contract.validate_plan_shape(plan)


class ReadinessDecisionTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()

    def _env(self, **extra):
        base = {contract.OPTIN_ENV: "1", contract.REVIEWED_HOST_ENV: "sulaco"}
        base.update(extra)
        return base.get

    def test_host_and_optin_and_deadline_and_resources(self):
        self.assertTrue(contract.decide_host("Sulaco", self.plan) == [])
        self.assertTrue(contract.decide_host("baccus", self.plan)[0].startswith("host_mismatch"))
        self.assertEqual(contract.decide_optin(self._env(), self.plan), [])
        missing = contract.decide_optin(lambda _key: None, self.plan)
        self.assertEqual(len(missing), 2)
        now = self.plan["guard"]["absolute_deadline_epoch"]
        self.assertEqual(contract.decide_deadline(self.plan, now - 7200), [])
        self.assertTrue(contract.decide_deadline(self.plan, now + 1))
        self.assertTrue(contract.decide_deadline(self.plan, now - 1800))  # margin
        good = contract.decide_resources(241987386128, 344812814336, self.plan)
        self.assertEqual(good, [])
        bad = contract.decide_resources(1, 1, self.plan)
        self.assertEqual(len(bad), 2)

    def test_zero_mem_available_is_a_broken_reading_not_free_memory(self):
        reasons = contract.decide_resources(0, 344812814336, self.plan)
        self.assertIn("mem_available_broken_reading(0)", reasons)
        self.assertFalse(any("below_floor" in reason for reason in reasons))

    def test_probe_path_resolves_to_the_real_output_filesystem(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            existing_parent = tmp / "data"
            existing_parent.mkdir()
            resolved = contract.resolve_probe_path(existing_parent / "out" / "deeper", tmp)
            self.assertEqual(resolved, existing_parent)
            self.assertEqual(contract.resolve_probe_path(tmp / "out", tmp), tmp)
            self.assertEqual(contract.resolve_probe_path(Path("/nope/nope/deeper"),
                                                         tmp / "fb"), Path("/"))

    def test_watchdog_env_pinning_and_barrier_precondition(self):
        self.assertEqual(contract.decide_watchdog_env(lambda _k: None), [])
        self.assertEqual(contract.decide_watchdog_env(
            lambda k: "5" if k == contract.WATCHDOG_POLL_ENV else None), [])
        self.assertTrue(contract.decide_watchdog_env(lambda _k: "9"))
        env = contract.pinned_watchdog_env({contract.WATCHDOG_POLL_ENV: "99"})
        self.assertEqual(env[contract.WATCHDOG_POLL_ENV], "5")
        with tempfile.TemporaryDirectory() as name:
            barrier = Path(name)
            self.assertEqual(contract.decide_barrier_precondition(barrier), [])
            (barrier / "barrier-armed.json").write_text("{}")
            self.assertTrue(contract.decide_barrier_precondition(barrier)[0]
                            .startswith("barrier_armed_receipt_preexists"))

    def test_composed_readiness_and_exit_codes(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            stage = tmp / "stage"
            stage.mkdir()
            (stage / "solver").write_bytes(b"x")
            (stage / "pgid-watchdog-v3.sh").write_bytes(b"y")
            (stage / "preflight.stdout").write_bytes(b"z")
            self.plan["binary_sha256"] = contract.sha256_file(stage / "solver")
            self.plan["watchdog_sha256"] = contract.sha256_file(stage / "pgid-watchdog-v3.sh")
            self.plan["preflight_sha256"] = contract.sha256_file(stage / "preflight.stdout")
            reasons = contract.evaluate_readiness(
                self.plan, stage, hostname="sulaco",
                mem_available=241987386128, disk_free=344812814336,
                env_get=self._env(), now=1789900000,
                output_path=tmp / "output", logs_path=tmp / "logs",
                barrier_dir=stage / "barrier")
            self.assertEqual(reasons, [])
            self.assertEqual(contract.refusal_exit_code(reasons), 64)
            self.assertEqual(contract.refusal_exit_code(["host_mismatch(x)"]), 65)
            self.assertEqual(contract.refusal_exit_code(["stage_hash_mismatch(solver)"]), 67)
            bad = contract.evaluate_readiness(
                self.plan, stage, hostname="baccus", mem_available=0, disk_free=0,
                env_get=lambda _k: None, now=1789900000,
                output_path=stage / "solver", logs_path=stage,
                barrier_dir=stage / "barrier")
            self.assertTrue(any(r.startswith("host_mismatch") for r in bad))
            self.assertTrue(any(r.startswith("target_exists") for r in bad))
            self.assertTrue(any(r.startswith("logs_dir_nonempty") for r in bad))


if __name__ == "__main__":
    unittest.main()
