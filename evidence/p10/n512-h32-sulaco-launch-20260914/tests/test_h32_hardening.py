"""Launch-blocker hardening: sticky ledger, phase-independent teardown with
honest retry, identity-bound shell watchdog, host process-exclusivity census
and the verified-subreaper census tooling."""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_children as kids  # noqa: E402
import h32_teardown as teardown  # noqa: E402
import h32_contract as contract  # noqa: E402
import h32_launch_supervisor as supervisor  # noqa: E402
import h32_receipts as receipts  # noqa: E402
import leak_census  # noqa: E402
import process_guard as guard  # noqa: E402

PACKET = Path(__file__).resolve().parents[1]
WATCHDOG = PACKET / "watchdog" / "pgid-watchdog-v3.sh"
PROC = Path("/proc")
DEVNULL = {"stdin": subprocess.DEVNULL, "stdout": subprocess.DEVNULL,
           "stderr": subprocess.DEVNULL}


def watchdog_identity(child_pid: int) -> tuple[str, str]:
    identity = kids.stable_identity(child_pid, PROC, time.sleep, samples=100)
    assert identity is not None, "child identity never stabilized"
    return str(identity.starttime), identity.cmdline_sha256


class LedgerStickyTests(unittest.TestCase):
    def test_uncertain_entry_is_never_overwritten(self):
        ledger = receipts.ReceiptLedger()
        ledger.record("release.token", "written")
        ledger.record("decision-receipt.json", "uncertain")
        again = ledger.record("decision-receipt.json", "written")
        self.assertEqual(again, "uncertain")
        self.assertEqual(ledger.uncertain, ["decision-receipt.json"])
        self.assertIn("receipt_uncertainty_unresolved",
                      ledger.refuse_reason() or "")

    def test_clean_entry_may_be_updated(self):
        ledger = receipts.ReceiptLedger()
        ledger.record("launch-receipt.json", "written")
        ledger.record("launch-receipt.json", "error:disk")
        self.assertEqual(ledger.uncertain, ["launch-receipt.json"])


class TeardownRetryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.log: list[str] = []
        self.solver = subprocess.Popen(["sleep", "300"],
                                       start_new_session=True, **DEVNULL)

    def _children(self) -> dict:
        # stable_identity: reading immediately after Popen can catch the
        # pre-exec cmdline (the fork of the parent); production spawns pin
        # through the same double-read stabilization.
        identity = kids.stable_identity(self.solver.pid, PROC, time.sleep,
                                        samples=100)
        assert identity is not None
        return {"solver": self.solver, "watchdog": None,
                "identity": identity, "watchdog_identity": None,
                "cleaned": False}

    def test_exception_in_one_phase_never_suppresses_the_others(self):
        children = self._children()
        with mock.patch.object(teardown, "_tree_sweep",
                               side_effect=RuntimeError("boom")):
            outcome = kids.cleanup_children(children, self.log.append,
                                            grace_seconds=2)
        report = children["cleanup_report"]
        self.assertFalse(children["cleaned"], "failed phase must NOT set cleaned")
        self.assertIn("solver_drain", report["phases_completed"])
        self.assertIn("watchdog_stop", report["phases_completed"])
        self.assertIn("reap_everything", report["phases_completed"])
        self.assertNotIn("owned_tree_sweep", report["phases_completed"])
        self.assertTrue(any("owned_tree_sweep" in e for e in
                            report["cleanup_errors"]), report["cleanup_errors"])
        self.assertEqual(outcome, "terminated_group")
        self.assertIsNotNone(self.solver.poll())  # solver phase ran fully

    def test_retry_redoes_work_and_never_answers_already_cleaned(self):
        children = self._children()
        with mock.patch.object(teardown, "_tree_sweep",
                               side_effect=RuntimeError("boom")):
            kids.cleanup_children(children, self.log.append, grace_seconds=2)
        outcome = kids.cleanup_children(children, self.log.append,
                                        grace_seconds=2)
        self.assertNotEqual(outcome, "already_cleaned")
        self.assertTrue(children["cleaned"])
        self.assertEqual(children["cleanup_report"]["cleanup_errors"], [])
        # A third call short-circuits (cleaned=True): no phase re-runs, and
        # the stored truthful outcome is echoed instead of redoing anything.
        before = len(self.log)
        third = kids.cleanup_children(children, self.log.append)
        self.assertEqual(third, children["outcome"])
        self.assertEqual(len(self.log), before, "cleaned cleanup re-ran")

    def test_adverse_watchdog_exit_is_never_erased_by_reaping(self):
        watchdog = subprocess.Popen([sys.executable, "-c", "raise SystemExit(70)"],
                                    start_new_session=True, **DEVNULL)
        watchdog.wait(timeout=5)
        outcome = kids.stop_watchdog(watchdog, self.log.append)
        self.assertTrue(outcome.startswith("reaped_adverse_exit_70"), outcome)


class ShellWatchdogIdentityTests(unittest.TestCase):
    """The REAL packet watchdog must never signal a drifted identity."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.log = self.root / "watchdog.log"
        self.env = dict(os.environ, WATCHDOG_POLL_SECONDS="1",
                        WATCHDOG_GRACE_SECONDS="2")

    def _spawn_solver(self, script: str) -> subprocess.Popen:
        return subprocess.Popen([sys.executable, "-c", script],
                                start_new_session=True, **DEVNULL)

    def _watchdog(self, pid: int, starttime: str, sha: str,
                  deadline: int) -> subprocess.Popen:
        return subprocess.Popen(
            ["/bin/sh", str(WATCHDOG), str(pid), str(os.getpgid(pid)),
             starttime, sha, str(deadline), str(self.log)],
            env=self.env, **DEVNULL)

    def test_exec_drift_after_term_is_never_killed_and_watchdog_fails(self):
        exec_file = self.root / "executed"
        script = self.root / "drifter.py"
        # The child execs a DIFFERENT cmdline on SIGTERM (same pid/starttime,
        # changed cmdline) — the exact exec/PID-recycle drift pattern.
        new_cmd = (f"import pathlib, time; pathlib.Path({str(exec_file)!r})"
                   ".write_text('1'); time.sleep(120)")
        script.write_text(
            "import os, signal, sys, time\n"
            f"NEW_CMD = {new_cmd!r}\n"
            "def _exec_on_term(signum, frame):\n"
            "    os.execv(sys.executable, [sys.executable, '-c', NEW_CMD])\n"
            "signal.signal(signal.SIGTERM, _exec_on_term)\n"
            "time.sleep(120)\n")
        child = subprocess.Popen([sys.executable, str(script)],
                                 start_new_session=True, **DEVNULL)
        starttime, sha = watchdog_identity(child.pid)
        watchdog = self._watchdog(child.pid, starttime, sha,
                                  int(time.time()) + 1)
        done = watchdog.wait(timeout=120)
        self.assertEqual(done, 70, self.log.read_text())
        text = self.log.read_text()
        self.assertIn("member_KILL_refused_identity_drift", text)
        self.assertIn("group_survivors_after_KILL", text)
        self.assertTrue(exec_file.exists())
        self.assertIsNone(child.poll(), "drifted identity must NOT be killed")
        guard.guard_child(self, child)
        term_waves = [line for line in text.splitlines()
                      if "group_escalation_TERM" in line]
        self.assertTrue(term_waves and all("sent=1" in line
                                           for line in term_waves))
        refused = [line for line in text.splitlines()
                   if "member_KILL_refused" in line]
        self.assertEqual(len(refused), 1)
        kill_waves = [line for line in text.splitlines()
                      if "grace_expired_sending_KILL" in line]
        self.assertEqual(len(kill_waves), 1)
        self.assertIn("sent=0", kill_waves[0],
                      "the drifted member must NEVER be KILLed")

    def test_attachment_mismatch_refuses_everything_with_65(self):
        child = self._spawn_solver("import time; time.sleep(120)")
        try:
            starttime, _sha = watchdog_identity(child.pid)
            watchdog = self._watchdog(child.pid, starttime, "0" * 64,
                                      int(time.time()) + 60)
            self.assertEqual(watchdog.wait(timeout=30), 65)
            self.assertIn("attachment_identity_mismatch", self.log.read_text())
            self.assertIsNone(child.poll(), "refusal means: NO signal ever")
        finally:
            guard.guard_child(self, child)


class ExclusivityCensusTests(unittest.TestCase):
    def _fake_proc(self, tmp: Path, pid: int, cmdline: str,
                   ppid: int = 1) -> None:
        d = tmp / str(pid)
        d.mkdir()
        fields = ["S", str(ppid), str(pid)] + ["0"] * 17 + ["7"]
        (d / "stat").write_text(f"{pid} (x) " + " ".join(fields))
        (d / "cmdline").write_bytes(cmdline.encode().replace(b" ", b"\0"))

    def test_decision_flags_each_fingerprint(self):
        reasons = contract.decide_process_exclusivity([
            (11, "/stage/solver run --probe"),
            (12, "/bin/sh /stage/pgid-watchdog-v3.sh 1 2 3 4 5 /tmp/log"),
            (13, "python3 scripts/h32_launch_supervisor.py --stage x"),
            (14, "/stage/p10-avx-scheduled-endpoint run"),
            (15, "totally unrelated --solver-ish process"),
        ])
        self.assertEqual(len(reasons), 4, reasons)
        self.assertTrue(all("host_exclusivity_violation" in r for r in reasons))
        self.assertEqual(contract.decide_process_exclusivity(
            [(15, "totally unrelated process")]), [])

    def test_host_observations_exclude_own_ancestry_only(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            self._fake_proc(tmp, os.getpid(), "self process")
            self._fake_proc(tmp, 99991, "/stage/solver run --probe")
            observations = supervisor.host_process_observations(tmp)
            pids = {pid for pid, _ in observations}
            self.assertNotIn(os.getpid(), pids)
            self.assertIn(99991, pids)

    def test_unreadable_proc_fails_closed(self):
        with mock.patch.object(supervisor, "host_process_observations",
                               side_effect=OSError("proc gone")):
            observations, reasons = supervisor._host_census({}, 1)
        self.assertEqual(observations, [])
        self.assertEqual(len(reasons), 1)
        self.assertIn("host_process_census_unreadable", reasons[0])


class CensusToolTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(guard.SUBREAPER_ACTIVE)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def _job(self, script: str, label: str) -> list[leak_census.ProcInfo]:
        path = self.root / f"{label}.py"
        path.write_text(script)
        rc, leaks = leak_census.run_job(label, [sys.executable, str(path)],
                                        self.root, self.root, 60)
        self.assertEqual(rc, 0)
        return leaks

    def test_setsid_escapee_is_counted_and_swept(self):
        leaks = self._job(
            "import subprocess\n"
            "d = {k: subprocess.DEVNULL for k in ('stdin', 'stdout', 'stderr')}\n"
            "subprocess.Popen(['sleep', '300'], start_new_session=True, **d)\n",
            "escape")
        self.assertEqual(len(leaks), 1)
        self.assertEqual(leaks[0].state, "S")
        self.assertEqual(leak_census.cleanup_after(leaks, os.getpid()), [])

    def test_zombie_under_live_orphaned_intermediate_is_counted(self):
        (self.root / "mid.py").write_text(
            "import subprocess, time\n"
            "d = {k: subprocess.DEVNULL for k in ('stdin', 'stdout', 'stderr')}\n"
            "subprocess.Popen(['sleep', '0.2'], **d)\n"
            "time.sleep(60)\n")
        leaks = self._job(
            "import subprocess\n"
            "d = {k: subprocess.DEVNULL for k in ('stdin', 'stdout', 'stderr')}\n"
            f"subprocess.Popen({[sys.executable, str(self.root / 'mid.py')]!r},"
            " start_new_session=True, **d)\n",
            "zomb")
        states = {info.state for info in leaks}
        self.assertIn("Z", states, f"zombie leak not counted: {states}")
        self.assertEqual(leak_census.cleanup_after(leaks, os.getpid()), [])

    def test_read_all_parses_fake_tree(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            d = tmp / "4242"
            d.mkdir()
            fields = ["S", "1", "4242"] + ["0"] * 16 + ["7", "0", "0", "0"]
            (d / "stat").write_text("4242 (fake solver) " + " ".join(fields))
            (d / "cmdline").write_bytes(b"sleep\0300")
            snap = leak_census.read_all(tmp)
            info = snap[4242]
            self.assertEqual((info.ppid, info.pgid, info.starttime,
                              info.state), (1, 4242, 7, "S"))


class GuardTests(unittest.TestCase):
    def test_guard_idle_drain_is_silent_and_empty(self):
        self.assertTrue(guard.SUBREAPER_ACTIVE)
        self.assertEqual(guard.kill_and_reap_remaining(deadline_seconds=1.0),
                         [])
        self.assertEqual(guard.reap_adopted_zombies(), [])


if __name__ == "__main__":
    unittest.main()
