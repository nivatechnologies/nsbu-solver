"""Driver-level decision evaluation, armed-line parsing and measurement."""

import os
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_barrier as bar  # noqa: E402
import h32_children as kids  # noqa: E402
import h32_contract as contract  # noqa: E402
import h32_launch_supervisor as driver  # noqa: E402
import h32_receipts as receipts  # noqa: E402
import h32_run as flow  # noqa: E402
from test_h32_bundles import STATE as BUNDLE_STATE, make_bundle, small_plan  # noqa: E402
from test_h32_contract import plan_template, write_plan  # noqa: E402

GOOD_ARMED = ("temporal_barrier armed attempt=1 clock=32 state_sha256=" + "a" * 64
              + " deadline=1789999999")


class ArmedLineTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()

    def test_accepts_and_rejects(self):
        ok, reason, state = driver.parse_armed_line(GOOD_ARMED, self.plan, "0a" * 32)
        self.assertTrue(ok, reason)
        self.assertEqual(state, "a" * 64)
        for line, fragment in (
            (GOOD_ARMED.replace("attempt=1", "attempt=2"), "attempt_mismatch"),
            (GOOD_ARMED.replace("clock=32", "clock=64"), "clock_mismatch"),
            (GOOD_ARMED + " nonce=deadbeef", "leaks_nonce"),
            (GOOD_ARMED.replace("a" * 64, "short"), "state_malformed"),
            (GOOD_ARMED.replace("deadline=1789999999", "deadline=whenever"), "deadline_malformed"),
            (GOOD_ARMED.replace("clock=32", "clock"), "malformed"),
            (GOOD_ARMED + " clock=32", "duplicated"),
        ):
            ok, reason, _ = driver.parse_armed_line(line, self.plan, "0a" * 32)
            self.assertFalse(ok, line)
            self.assertIn(fragment, reason)


class DecisionTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()

    def test_release_only_when_every_gate_passed(self):
        decision = driver.evaluate_decision(self.plan, True, True, (True, "pass"), [])
        self.assertTrue(decision["release"])
        for args, fragment in (
            ((False, True, (True, "pass"), []), "stdout"),
            ((True, False, (True, "pass"), []), "armed"),
            ((True, True, (False, "bundle_invalid(x)"), []), "bundle_invalid"),
            ((True, True, (True, "pass"), ["rss_outside_cap"]), "rss"),
        ):
            decision = driver.evaluate_decision(self.plan, *args)
            self.assertFalse(decision["release"])
            self.assertTrue(any(fragment in reason for reason in decision["refusal_reasons"]))

    def test_receipt_schema_names_and_uncertainty_passthrough(self):
        with tempfile.TemporaryDirectory() as name:
            stage = Path(name)
            logs: list[str] = []
            status = driver.write_receipt(stage, "launch-receipt.json", {"a": 1}, logs.append)
            self.assertEqual(status, "written")
            written = stage / "launch-receipt.json"
            self.assertIn("sulaco-temporal-h32-launch-receipt-v1", written.read_text())
            status = driver.write_receipt(stage, "launch-receipt.json", {"a": 2}, logs.append)
            self.assertEqual(status, "uncertain_exists")
            self.assertTrue(any("receipt_uncertainty" in line for line in logs))


class MeasureTests(unittest.TestCase):
    def setUp(self):
        self.plan = plan_template()
        self.patches = {}

    def measure(self, mem, rss, disk, barrier_present=True):
        original = (driver.real_mem_available_bytes, driver.real_rss_bytes,
                    driver.real_disk_free_bytes)
        driver.real_mem_available_bytes = lambda *a, **k: mem
        driver.real_rss_bytes = lambda *a, **k: rss
        driver.real_disk_free_bytes = lambda *a, **k: disk
        try:
            return driver.measure_resources(self.plan, 123, barrier_present)
        finally:
            (driver.real_mem_available_bytes, driver.real_rss_bytes,
             driver.real_disk_free_bytes) = original

    def test_accepts_within_caps(self):
        resources, reasons = self.measure(250_000_000_000, 200_000_000_000,
                                          310_453_075_968 - 3_233_886_208)
        self.assertEqual(reasons, [])
        self.assertEqual(resources["solver_rss_bytes"], 200_000_000_000)

    def test_refuses_oversized_rss_and_short_disk(self):
        _res, reasons = self.measure(250_000_000_000, 208_000_000_000, 1_000)
        self.assertEqual(len(reasons), 2)
        _res, reasons = self.measure(-1, 1000, 999_999_999_999, barrier_present=False)
        self.assertIn("barrier_dir_missing", reasons)
        self.assertIn("mem_available_unreadable", reasons)

    def test_release_time_memory_floor_is_meaningful(self):
        _res, reasons = self.measure(200_000_000_000, 1000, 400_000_000_000)
        self.assertIn("mem_available_below_release_floor(have=200000000000,"
                      "floor=241987386128)", reasons)


class WallBudgetTests(unittest.TestCase):
    def test_wall_budget_is_bounded_by_measured_sulaco_evidence(self):
        plan = plan_template()
        self.assertEqual(driver.first_step_wall_seconds(plan, 1789900000), 2103)
        self.assertEqual(driver.first_step_wall_seconds(plan, 1789999000), 999)
        self.assertEqual(driver.first_step_wall_seconds(plan, 1789999999), 0)


class OneShotBarrierFlowTests(unittest.TestCase):
    STATE = "0b" * 32

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.stage = self.root / "stage"
        (self.stage / "barrier").mkdir(parents=True)
        self.plan = small_plan()
        self.out = self.root / "out"
        make_bundle(self.out, self.plan)
        self.STATE = BUNDLE_STATE
        self.flow = flow.BarrierFlow(self.plan, self.stage, self.out,
                                     self.root / "logs", lambda _m: None)

    def test_exactly_one_token_of_any_kind_per_attempt(self):
        status, _reason = self.flow.decide_and_issue({"release": True}, self.STATE)
        self.assertEqual(status, "written")
        self.assertTrue((self.stage / "barrier" / bar.RELEASE_FILE).is_file())
        for decision in ({"release": True}, {"release": False}):
            status, reason = self.flow.decide_and_issue(decision, self.STATE)
            self.assertEqual(status, "refused_protocol_violation", decision)
            self.assertIn("already_issued", reason)
        self.assertEqual(sorted(p.name for p in (self.stage / "barrier").iterdir()),
                         [bar.RELEASE_FILE])

    def _children(self, alive=True, drift=False):
        identity = kids.read_identity(os.getpid(), Path("/proc"))
        if drift:
            identity = kids.Identity(identity.pid, identity.pgid, identity.state,
                                     identity.starttime, "0" * 64)

        class Solver:
            def poll(self_inner):
                return None if alive else 0

        return {"solver": Solver(), "identity": identity}

    def test_pre_token_doubts_refuse_uncertain_dead_drifted_and_preexisting(self):
        ledger = receipts.ReceiptLedger()
        doubts = flow.release_doubts(self.plan, self.flow, self.out,
                                        self._children(), self.STATE, ledger)
        self.assertEqual(doubts, [])
        ledger.record("launch-receipt.json", receipts.UNCERTAIN)
        doubts = flow.release_doubts(self.plan, self.flow, self.out,
                                        self._children(), self.STATE, ledger)
        self.assertTrue(any("receipt_uncertainty" in d for d in doubts))
        ledger2 = receipts.ReceiptLedger()
        doubts = flow.release_doubts(self.plan, self.flow, self.out,
                                        self._children(alive=False), self.STATE, ledger2)
        self.assertIn("solver_exited_before_token", doubts)
        doubts = flow.release_doubts(self.plan, self.flow, self.out,
                                        self._children(drift=True), self.STATE, ledger2)
        self.assertIn("solver_identity_drift_before_token", doubts)
        (self.stage / "barrier" / bar.ABORT_FILE).write_text("preexisting\n")
        doubts = flow.release_doubts(self.plan, self.flow, self.out,
                                        self._children(), self.STATE, ledger2)
        self.assertTrue(any("token_preexisting" in d for d in doubts))


class ReleaseDeadlineTests(unittest.TestCase):
    """ONE absolute first-step/release deadline enforced after every wait,
    validation and receipt, including at the final token gate."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.stage = self.root / "stage"
        (self.stage / "barrier").mkdir(parents=True)
        self.plan = small_plan()
        self.out = self.root / "out"
        make_bundle(self.out, self.plan)
        self.flow = flow.BarrierFlow(self.plan, self.stage, self.out,
                                     self.root / "logs", lambda _m: None)

    def test_final_gate_refuses_after_the_deadline_or_with_doubts(self):
        release, reasons = bar.finalise_release_decision(True, [], 100.0, 200.0)
        self.assertTrue(release)
        self.assertEqual(reasons, [])
        release, reasons = bar.finalise_release_decision(True, [], 200.0, 200.0)
        self.assertFalse(release)
        self.assertTrue(any("release_deadline_passed" in r for r in reasons))
        release, reasons = bar.finalise_release_decision(True, ["doubt(x)"], 100.0, 200.0)
        self.assertFalse(release)
        self.assertIn("doubt(x)", reasons)

    def test_token_gate_never_releases_past_the_deadline(self):
        status, reason = self.flow.decide_and_issue({"release": True}, "0b" * 32,
                                                    release_deadline=time.time() - 1)
        self.assertEqual(status, "refused_deadline")
        self.assertIn("release_deadline_passed_at_token_issue", reason)
        self.assertEqual(list((self.stage / "barrier").iterdir()), [])

    def test_barrier_environment_deadline_is_capped_by_the_plan_deadline(self):
        env = self.flow.environment({}, now=1789800000)
        want = int(self.plan["guard"]["absolute_deadline_epoch"])
        self.assertLessEqual(int(env["NSBU_N512_TEMPORAL_BARRIER_DEADLINE"]), want)


class WatchdogPinTests(unittest.TestCase):
    def test_watchdog_bytes_must_match_the_reviewed_sha256_before_spawn(self):
        with tempfile.TemporaryDirectory() as name:
            stage = Path(name)
            plan = plan_template()
            logs: list[str] = []
            (stage / "pgid-watchdog-v3.sh").write_bytes(b"tampered bytes\n")
            self.assertFalse(flow.verify_watchdog_bytes(stage, plan, logs.append))
            self.assertTrue(any("watchdog_sha256_mismatch" in line for line in logs))
            plan["watchdog_sha256"] = contract.sha256_file(stage / "pgid-watchdog-v3.sh")
            self.assertTrue(flow.verify_watchdog_bytes(stage, plan, logs.append))


class TimerBeforeSpawnTests(unittest.TestCase):
    def test_first_step_timer_starts_before_popen_so_spawn_latency_counts(self):
        with tempfile.TemporaryDirectory() as name:
            stage = Path(name) / "stage"
            stage.mkdir()
            children = {"solver": None, "identity": None, "cleaned": False}

            class NoSpawn:
                called_at = None

                def __init__(self, *args, **kwargs):
                    for stream in kwargs["stdout"], kwargs["stderr"]:
                        stream.write(b"spawned")
                    NoSpawn.called_at = time.monotonic()
                    self.pid = os.getpid() + 987654321  # definitely not in /proc

            with tempfile.NamedTemporaryFile() as out_stream, \
                    tempfile.NamedTemporaryFile() as err_stream:
                with mock.patch.object(flow.subprocess, "Popen", NoSpawn):
                    ok = flow.spawn_solver(stage, stage / "out",
                                           {"solver.stdout": out_stream,
                                            "solver.stderr": err_stream},
                                           {}, plan_template(), children, lambda _m: None)
            self.assertFalse(ok)
            self.assertIsNotNone(NoSpawn.called_at)
            self.assertIn("first_step_timer_start", children)
            self.assertLessEqual(children["first_step_timer_start"], NoSpawn.called_at)


class OutputFilesystemPreflightTests(unittest.TestCase):
    def test_disk_probe_uses_the_real_output_filesystem_not_the_cwd(self):
        remote = Path("/dev/shm")
        if not remote.is_dir():
            self.skipTest("no second filesystem available for the probe test")
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            stage = tmp / "stage"
            (stage / "barrier").mkdir(parents=True)
            for fname in ("solver", "pgid-watchdog-v3.sh", "preflight.stdout"):
                (stage / fname).write_bytes(fname.encode())
            plan = plan_template()
            plan["resources"]["probe_path"] = str(stage)
            for fname, key in (("solver", "binary_sha256"),
                               ("pgid-watchdog-v3.sh", "watchdog_sha256"),
                               ("preflight.stdout", "preflight_sha256")):
                plan[key] = contract.sha256_file(stage / fname)
            path, digest = write_plan(tmp, plan)
            path.replace(stage / "frozen-plan.json")
            output = remote / f"h32-probe-test-{os.getpid()}" / "output"
            seen: list[Path] = []
            originals = (driver.real_hostname, driver.real_mem_available_bytes,
                         driver.real_disk_free_bytes)
            driver.real_hostname = lambda: "sulaco"
            driver.real_mem_available_bytes = lambda *a, **k: 300_000_000_000
            driver.real_disk_free_bytes = lambda p: (seen.append(Path(p)), 500_000_000_000)[1]
            os.environ[contract.OPTIN_ENV] = "1"
            os.environ[contract.REVIEWED_HOST_ENV] = "sulaco"
            try:
                cfg = driver.LaunchConfig(stage=stage, output=output,
                                          logs=tmp / "logs", plan_sha256=digest)
                code = driver.run_preflight_only(cfg, lambda _m: None)
            finally:
                (driver.real_hostname, driver.real_mem_available_bytes,
                 driver.real_disk_free_bytes) = originals
                os.environ.pop(contract.OPTIN_ENV, None)
                os.environ.pop(contract.REVIEWED_HOST_ENV, None)
            self.assertEqual(code, 0)
            self.assertTrue(seen)
            for probe in seen:
                self.assertTrue(str(probe).startswith(str(remote)), str(probe))
                self.assertNotEqual(Path(os.getcwd()), probe)


if __name__ == "__main__":
    unittest.main()
