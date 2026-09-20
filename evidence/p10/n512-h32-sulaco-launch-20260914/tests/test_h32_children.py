"""Ownership identity, interruption windows, group drain and reaping."""

import json
import os
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_children as kids  # noqa: E402
from process_guard import DETACHED, guard_child  # noqa: E402

def fake_proc(tmp: Path, pid: int, *, pgid: int, starttime: int = 7,
              state: str = "S", cmdline: bytes = b"/stage/solver\0run\0") -> None:
    proc = tmp / str(pid)
    proc.mkdir(exist_ok=True)
    post = [state, "1", str(pgid)] + ["0"] * 16 + [str(starttime)] + ["0"] * 4
    (proc / "stat").write_text(f"{pid} (solver) " + " ".join(post))
    (proc / "cmdline").write_bytes(cmdline)


class ParseTests(unittest.TestCase):
    def test_stat_parse_and_cmdline_sha(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            fake_proc(tmp, 4242, pgid=4242)
            identity = kids.read_identity(4242, tmp)
            self.assertIsNotNone(identity)
            assert identity is not None
            self.assertEqual((identity.pid, identity.pgid, identity.starttime), (4242, 4242, 7))
            self.assertTrue(kids.identity_matches(identity, identity))
            zombie = kids.read_identity(4242, tmp)
            assert zombie is not None
            (tmp / "4242" / "stat").write_text(
                (tmp / "4242" / "stat").read_text().replace(" S ", " Z ", 1))
            self.assertFalse(kids.identity_matches(kids.read_identity(4242, tmp), identity))

    def test_stable_identity_requires_session_leader_then_accepts(self):
        with tempfile.TemporaryDirectory() as name:
            tmp = Path(name)
            fake_proc(tmp, 515, pgid=1)  # not yet its own session
            self.assertIsNone(kids.stable_identity(515, tmp, lambda _s: None, samples=3))
            fake_proc(tmp, 515, pgid=515)
            identity = kids.stable_identity(515, tmp, lambda _s: None, samples=3)
            self.assertIsNotNone(identity)


class DrainTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.killed: list[tuple[int, int]] = []

    def kill(self, pid: int, signum: int) -> None:
        self.killed.append((pid, signum))
        if signum == signal.SIGKILL and (self.root / str(pid)).exists():
            import shutil

            shutil.rmtree(self.root / str(pid))

    def identity(self, pid: int = 900) -> kids.Identity:
        return kids.Identity(pid, pid, "S", 7, kids.sha256_bytes(b"/stage/solver\0run\0"))

    def test_recycled_leader_pid_blocks_all_group_signals(self):
        fake_proc(self.root, 900, pgid=900, starttime=7, cmdline=b"other\0")
        outcome = kids.drain_group(self.identity(), self.root, self.kill, lambda _s: None)
        self.assertEqual(outcome, "identity_changed_no_signal")
        self.assertEqual(self.killed, [])

    def test_all_members_terminated_killed_then_group_disappearance_verified(self):
        fake_proc(self.root, 900, pgid=900)
        fake_proc(self.root, 901, pgid=900)
        fake_proc(self.root, 902, pgid=900)
        clock = {"t": 0.0}

        def sleep(_seconds: float) -> None:
            clock["t"] += 30.0  # exhaust the grace window after two polls

        outcome = kids.drain_group(self.identity(), self.root, self.kill, sleep,
                                   grace_seconds=60, now=lambda: clock["t"])
        terms = [pid for pid, signum in self.killed if signum == signal.SIGTERM]
        kills = [pid for pid, signum in self.killed if signum == signal.SIGKILL]
        self.assertEqual(outcome, "killed_group_terminated")  # disappearance verified
        self.assertEqual(sorted(terms), [900, 901, 902])
        self.assertEqual(sorted(kills), [900, 901, 902])

    def test_persisting_survivor_after_kill_preserves_adverse_outcome(self):
        fake_proc(self.root, 900, pgid=900)
        fake_proc(self.root, 901, pgid=900)
        clock = {"t": 0.0}

        def sleep(_seconds: float) -> None:
            clock["t"] += 30.0

        def kill_never_clears(pid: int, signum: int) -> None:
            self.killed.append((pid, signum))  # nothing ever dies

        outcome = kids.drain_group(self.identity(), self.root, kill_never_clears, sleep,
                                   grace_seconds=60, now=lambda: clock["t"])
        self.assertEqual(outcome, "killed_group_survivors")
        self.assertEqual(sorted(pid for pid, sig in self.killed if sig == signal.SIGKILL),
                         [900, 901])

    def test_member_whose_identity_drifted_mid_drain_is_never_killed(self):
        fake_proc(self.root, 900, pgid=900)
        fake_proc(self.root, 901, pgid=900, starttime=7)
        clock = {"t": 0.0, "mutated": False}

        def sleep(_seconds: float) -> None:
            clock["t"] += 30.0
            if not clock["mutated"]:  # a recycled PID appears at grace expiry
                fake_proc(self.root, 901, pgid=900, starttime=99)
                clock["mutated"] = True

        outcome = kids.drain_group(self.identity(), self.root, self.kill, sleep,
                                   grace_seconds=60, now=lambda: clock["t"])
        kills = sorted(pid for pid, signum in self.killed if signum == signal.SIGKILL)
        self.assertEqual(kills, [900])  # drifted member never gets a KILL
        self.assertEqual(outcome, "killed_group_survivors")  # 901 still there: adverse

    def test_zombie_leader_still_drains_group_descendants(self):
        fake_proc(self.root, 900, pgid=900, state="Z")
        fake_proc(self.root, 901, pgid=900)
        outcome = kids.drain_group(self.identity(), self.root, self.kill,
                                   lambda _s: None, grace_seconds=0)
        self.assertIn(outcome, ("terminated_group", "killed_group_survivors",
                                "killed_group_terminated"))
        self.assertIn(901, [pid for pid, _sig in self.killed])

    def test_protected_and_foreign_groups_are_refused(self):
        protected = kids.Identity(303603, 303603, "S", 7, "0" * 64)
        self.assertEqual(
            kids.drain_group(protected, self.root, self.kill, lambda _s: None),
            "refused_not_owned_group")
        foreign = kids.Identity(900, 1, "S", 7, "0" * 64)
        self.assertEqual(
            kids.drain_group(foreign, self.root, self.kill, lambda _s: None),
            "refused_not_owned_group")


class RealProcessCleanupTests(unittest.TestCase):
    def test_leader_exit_still_drains_group_and_reaps_direct_child(self):
        leader = subprocess.Popen(
            ["/bin/sh", "-c", "sleep 60 & exec sleep 60"],
            start_new_session=True, **DETACHED)
        guard_child(self, leader)
        deadline = time.time() + 10
        while time.time() < deadline:
            pgid = os.getpgid(leader.pid)
            members = [pid for pid in kids.group_members(Path("/proc"), pgid)]
            if len(members) >= 1:
                break
            time.sleep(0.05)
        pgid = os.getpgid(leader.pid)
        stat_text = (Path("/proc") / str(leader.pid) / "stat").read_text()
        _state, group, starttime = kids.parse_proc_stat(stat_text)
        identity = kids.Identity(leader.pid, pgid, "S", starttime,
                                 kids.sha256_bytes((Path("/proc") / str(leader.pid) / "cmdline").read_bytes()))
        children = {"solver": leader, "watchdog": None, "identity": identity, "cleaned": False}
        logs: list[str] = []
        leader.terminate()  # direct child dies; its backgrounded descendant stays
        deadline = time.time() + 10
        while leader.poll() is None and time.time() < deadline:
            time.sleep(0.05)
        outcome = kids.cleanup_children(children, logs.append)
        reap_direct = leader.returncode is not None
        deadline = time.time() + 15
        survivors = []
        while time.time() < deadline:
            survivors = [pid for pid in kids.group_members(Path("/proc"), pgid)
                         if pid != leader.pid]
            if not survivors:
                break
            time.sleep(0.1)
        self.assertTrue(reap_direct, "direct child must be reaped even after exit")
        self.assertFalse(survivors, f"descendants survived: {survivors}")
        self.assertIn(outcome, ("terminated_group", "group_already_gone",
                                "killed_group_survivors", "killed_group_terminated"))


class InterruptionTests(unittest.TestCase):
    def test_sigterm_inside_deferred_window_is_queued_then_raised_after(self):
        seen: list[str] = []

        def body() -> None:
            previous = kids.install_signal_handlers()
            try:
                with kids.deferred_signals():
                    os.kill(os.getpid(), signal.SIGTERM)
                    time.sleep(0.2)
                    seen.append("still-inside")
            finally:
                kids.restore_signal_handlers(previous)

        with self.assertRaises(kids.Terminated):
            body()
        self.assertEqual(seen, ["still-inside"])

    def test_owned_cleanup_runs_once_for_keyboard_interrupt(self):
        calls: list[str] = []

        def body() -> int:
            raise KeyboardInterrupt

        def cleanup() -> None:
            calls.append("cleanup")

        code = kids.run_with_owned_cleanup(body, cleanup, lambda _m: None)
        self.assertEqual(code, 70)
        self.assertEqual(calls, ["cleanup"])

    def test_abort_recorder_runs_even_with_clean_cleanup(self):
        calls: list[str] = []

        def body() -> int:
            raise RuntimeError("boom")

        code = kids.run_with_owned_cleanup(
            body, lambda: calls.append("cleanup"), lambda _m: None,
            on_abort=lambda descriptor: calls.append(f"abort:{descriptor}"))
        self.assertEqual(code, 70)
        self.assertEqual(calls, ["cleanup", "abort:abort_RuntimeError"])

    def test_signal_description_is_truthful(self):
        self.assertEqual(kids.describe_signal(kids.Terminated(15)), "signal_SIGTERM")
        self.assertEqual(kids.describe_signal(KeyboardInterrupt()),
                         "signal_KeyboardInterrupt")
        self.assertEqual(kids.describe_signal(RuntimeError("x")), "abort_RuntimeError")

    def test_cleanup_never_rebinds_a_changed_identity(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            fake_proc(root, 700, pgid=700, starttime=11)
            identity = kids.Identity(700, 700, "S", 7, kids.sha256_bytes(b"original"))
            killed: list[tuple[int, int]] = []

            class FakeSolver:
                pid = 700

                def __init__(self) -> None:
                    self.terminates = 0

                def poll(self):
                    return None

                def terminate(self) -> None:
                    self.terminates += 1

                def wait(self, timeout=None):
                    del timeout

            solver = FakeSolver()
            children = {"solver": solver, "watchdog": None, "identity": identity,
                        "cleaned": False}
            outcome = kids.cleanup_children(children, lambda _m: None, proc_root=root)
            self.assertIn("identity_changed_no_signal", outcome)
            # r5: a drifted identity receives NOTHING — no group signal, no
            # direct terminate, no reap-escalation signal.  Cleanup fails
            # truthfully instead.
            self.assertEqual(solver.terminates, 0)
            self.assertEqual(killed, [])
            report = children["cleanup_report"]
            self.assertTrue(report["identity_drift"])
            self.assertIn("solver", report["unconfirmed"])
            self.assertTrue(any("identity_refused_no_signal" in item
                                for item in report["adverse"]))
            outcome2 = kids.cleanup_children(children, lambda _m: None, proc_root=root)
            self.assertEqual(outcome2, outcome)  # idempotent, no second wave

    def test_watchdog_is_stopped_even_without_solver(self):
        watchdog = subprocess.Popen(["sleep", "30"], start_new_session=True,
                                    **DETACHED)
        guard_child(self, watchdog)
        logs: list[str] = []
        kids.cleanup_children({"solver": None, "watchdog": watchdog,
                               "identity": None, "cleaned": False}, logs.append)
        self.assertIsNotNone(watchdog.poll())


def pinned(child: subprocess.Popen) -> kids.Identity:
    deadline = time.time() + 10
    while time.time() < deadline:
        identity = kids.stable_identity(child.pid, Path("/proc"), time.sleep, samples=4)
        if identity is not None:
            return identity
        time.sleep(0.05)
    raise AssertionError("identity never stabilized")


class TeardownPathTests(unittest.TestCase):
    """Success/error/interruption paths must drain and reap BOTH children."""

    def spawn_pair(self):
        solver = subprocess.Popen(["sleep", "300"], start_new_session=True,
                                  **DETACHED)
        watchdog = subprocess.Popen(["sleep", "300"], start_new_session=True,
                                    **DETACHED)
        guard_child(self, solver, watchdog)
        children = {"solver": solver, "watchdog": watchdog, "cleaned": False,
                    "identity": pinned(solver), "watchdog_identity": pinned(watchdog)}
        return solver, watchdog, children

    def wait_exit(self, child, timeout=30):
        deadline = time.time() + timeout
        while child.poll() is None and time.time() < deadline:
            time.sleep(0.05)
        return child.poll() is not None

    def test_success_path_stops_watchdog_and_reaps_both(self):
        solver, watchdog, children = self.spawn_pair()
        solver.terminate()
        self.assertTrue(self.wait_exit(solver))
        logs: list[str] = []
        kids.cleanup_children(children, logs.append, grace_seconds=5)
        self.assertTrue(self.wait_exit(watchdog), "watchdog must be terminated+reaped")
        self.assertIsNotNone(solver.poll())
        report = children["cleanup_report"]
        self.assertEqual(report["unconfirmed"], [])

    def test_kill_escalation_reaches_a_term_ignoring_grandchild(self):
        leader = subprocess.Popen(
            ["/bin/sh", "-c", "trap '' TERM; sleep 300 & trap '' TERM; exec sleep 300"],
            start_new_session=True, **DETACHED)
        guard_child(self, leader)
        identity = pinned(leader)
        deadline = time.time() + 10
        members: list[int] = []
        while time.time() < deadline:
            members = kids.group_members(Path("/proc"), identity.pgid)
            if len(members) >= 2:
                break
            time.sleep(0.05)
        self.assertGreaterEqual(len(members), 2, "grandchild must exist")
        outcome = kids.drain_group(identity, Path("/proc"), os.kill, time.sleep,
                                   grace_seconds=2)
        self.assertIn(outcome, ("killed_group_survivors", "killed_group_terminated"))
        deadline = time.time() + 15
        while kids.group_members(Path("/proc"), identity.pgid) and time.time() < deadline:
            time.sleep(0.1)
        self.assertEqual(kids.group_members(Path("/proc"), identity.pgid), [])
        leader.wait(timeout=15)

    def test_reap_escalates_a_term_ignoring_direct_child(self):
        child = subprocess.Popen(
            ["/bin/sh", "-c", "trap '' TERM; exec sleep 300"],
            start_new_session=True, **DETACHED)
        guard_child(self, child)
        logs: list[str] = []
        self.assertTrue(kids.reap(child, logs.append, timeout=2),
                        f"reap must escalate TERM->KILL: {logs}")
        self.assertTrue(any("reap_escalate" in line for line in logs), logs)

    def test_watchdog_drain_preserves_an_adverse_exit_status(self):
        child = subprocess.Popen(["/bin/sh", "-c", "exit 70"],
                                 start_new_session=True, **DETACHED)
        guard_child(self, child)
        logs: list[str] = []
        outcome = kids.stop_watchdog(child, logs.append)
        self.assertEqual(outcome, "reaped_adverse_exit_70")

    def test_repaired_watchdog_keeps_escalating_after_leader_exit(self):
        watchdog_script = (Path(__file__).resolve().parents[1] / "watchdog"
                           / "pgid-watchdog-v3.sh")
        leader = subprocess.Popen(
            ["/bin/sh", "-c", "(trap '' TERM; exec sleep 120) & exec sleep 2"],
            start_new_session=True, **DETACHED)
        guard_child(self, leader)
        identity = pinned(leader)
        env = dict(os.environ, WATCHDOG_POLL_SECONDS="1", WATCHDOG_GRACE_SECONDS="2")
        with tempfile.TemporaryDirectory() as name:
            log_path = Path(name) / "watchdog.log"
            watchdog = subprocess.Popen(
                ["/bin/sh", str(watchdog_script), str(identity.pid), str(identity.pgid),
                 str(identity.starttime), identity.cmdline_sha256,
                 str(int(time.time()) + 120), str(log_path)],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, env=env, start_new_session=True)
            guard_child(self, watchdog)
            deadline = time.time() + 90
            while watchdog.poll() is None and time.time() < deadline:
                time.sleep(0.2)
            self.assertEqual(watchdog.poll(), 0, "watchdog must clear the group and exit 0")
            text = log_path.read_text()
            self.assertIn("group_escalation_TERM reason=leader_identity_lost_", text)
            self.assertIn("group_cleared_after_KILL", text)
            self.assertEqual(kids.group_members(Path("/proc"), identity.pgid), [])

    def test_changed_identity_gets_no_signal_from_a_real_drain(self):
        child = subprocess.Popen(["sleep", "300"], start_new_session=True,
                                 **DETACHED)
        guard_child(self, child)
        observed = pinned(child)
        forged = kids.Identity(observed.pid, observed.pgid, observed.state,
                               observed.starttime, "0" * 64)
        outcome = kids.drain_group(forged, Path("/proc"), os.kill, time.sleep,
                                   grace_seconds=1)
        self.assertEqual(outcome, "identity_changed_no_signal")
        self.assertIsNone(child.poll(), "mismatched identity must have received no signal")
        child.terminate()
        child.wait(timeout=15)

    def run_driver_path(self, mode: str) -> dict:
        script = f'''
import json, os, signal, subprocess, sys, time
from pathlib import Path
sys.path.insert(0, {str(Path(__file__).resolve().parents[1] / "scripts")!r})
import h32_children as kids, h32_receipts as receipts
root = Path(sys.argv[1])
def pin(child):
    deadline = time.time() + 10
    while time.time() < deadline:
        identity = kids.stable_identity(child.pid, Path("/proc"), time.sleep, samples=4)
        if identity is not None:
            return identity
        time.sleep(0.05)
    raise SystemExit("identity never stabilized")
children = {{"solver": None, "watchdog": None, "identity": None,
           "watchdog_identity": None, "cleaned": False}}
def body():
    detached = {{"stdin": subprocess.DEVNULL, "stdout": subprocess.DEVNULL,
                 "stderr": subprocess.DEVNULL}}
    solver = subprocess.Popen(["sleep", "300"], start_new_session=True, **detached)
    # register EACH pid the instant it exists: an external sweep still knows
    # this group even if we die before the next spawn completes.
    (root / "spawn.json").write_text(json.dumps([solver.pid]))
    watchdog = subprocess.Popen(["sleep", "300"], start_new_session=True, **detached)
    # descendants never inherit the OUTER test pipes (no orphan can stall a
    # capture-pipe drain); the test can sweep these groups even if this
    # process itself is SIGKILLed at the timeout.
    (root / "spawn.json").write_text(json.dumps([solver.pid, watchdog.pid]))
    children.update(solver=solver, watchdog=watchdog, identity=pin(solver),
                    watchdog_identity=pin(watchdog))
    if {mode!r} == "interrupt":
        os.kill(os.getpid(), signal.SIGINT)
        time.sleep(0.5)
        return 0
    raise RuntimeError("boom")
def cleanup():
    kids.cleanup_children(children, lambda m: None, grace_seconds=10)
    report = children.get("cleanup_report", {{}})
    receipts.create_json(root / "failure-receipt.json", {{"report": report}})
    (root / "polls.json").write_text(json.dumps(
        {{"solver": children["solver"].poll(), "watchdog": children["watchdog"].poll()}}))
code = kids.run_with_owned_cleanup(body, cleanup, lambda m: None)
# UNCONDITIONAL final sweep: nothing may outlive this driver on any path.
for child in (children["solver"], children["watchdog"]):
    if child is not None:
        try:
            os.killpg(os.getpgid(child.pid), signal.SIGKILL)
        except OSError:
            pass
        try:
            child.wait(timeout=5)
        except Exception:
            pass
print(code)
'''
        with tempfile.TemporaryDirectory() as name:
            script_path = Path(name) / "driver_path.py"
            script_path.write_text(script)
            spawned: list[int] = []
            try:
                done = subprocess.run([sys.executable, str(script_path), name],
                                      capture_output=True, text=True, timeout=120)
                returncode = done.returncode
            finally:
                if (Path(name) / "spawn.json").is_file():
                    spawned = json.loads((Path(name) / "spawn.json").read_text())
                for pid in spawned:  # unconditional: also on timeout/failure
                    try:
                        os.killpg(os.getpgid(pid), signal.SIGKILL)
                    except OSError:
                        pass
            self.assertEqual(returncode, 0, done.stderr)
            self.assertEqual(done.stdout.strip(), "70", mode)
            receipt = json.loads((Path(name) / "failure-receipt.json").read_text())
            polls = json.loads((Path(name) / "polls.json").read_text())
        self.assertIsNotNone(polls["solver"], "solver must be reaped")
        self.assertIsNotNone(polls["watchdog"], "watchdog must be reaped")
        self.assertEqual(receipt["report"]["unconfirmed"], [], mode)
        return receipt

    def test_error_path_reaps_both_and_writes_failure_receipt(self):
        self.run_driver_path("error")

    def test_sigint_path_reaps_both_and_writes_failure_receipt(self):
        self.run_driver_path("interrupt")


if __name__ == "__main__":
    unittest.main()
