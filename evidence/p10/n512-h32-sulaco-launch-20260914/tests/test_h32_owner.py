"""Owner-primitive tests: verified subreaper, persistent adoption, immutable
identity, identity-bound signaling and complete reaping.

Every process-spawning method ends with ZERO children (live or zombie) of the
test runner: the runner itself is a VERIFIED subreaper (process_guard import),
so anything we fail to reap here would be a leak the external census sees.
"""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_children as kids  # noqa: E402
import process_guard as guard  # noqa: E402  (subreaper + final drain)
import h32_owner as owner  # noqa: E402

PROC = Path("/proc")
DEVNULL = {"stdin": subprocess.DEVNULL, "stdout": subprocess.DEVNULL,
           "stderr": subprocess.DEVNULL}


def spawn(*args: str, setsid: bool = True) -> subprocess.Popen:
    # Test spawns ALWAYS own their process group: a guard killpg of a child
    # that shares the runner's group would signal the runner itself.
    return subprocess.Popen(list(args), start_new_session=setsid, **DEVNULL)


class SubreaperTests(unittest.TestCase):
    def test_subreaper_is_enabled_and_verifies(self):
        self.assertTrue(guard.SUBREAPER_ACTIVE)
        self.assertTrue(owner.enable_child_subreaper())
        self.assertTrue(owner.child_subreaper_active())

    def test_adopted_zombie_is_reapable_by_this_process(self):
        child = spawn("sleep", "0.05")
        pid = child.pid
        deadline = time.time() + 10
        while (owner.read_identity(pid, PROC) is not None
               and time.time() < deadline):
            time.sleep(0.05)
        child.wait(timeout=5)
        self.assertIsNone(owner.read_identity(pid, PROC))

    def test_escapee_reparented_to_subreaper_when_driver_dies(self):
        with tempfile.TemporaryDirectory() as name:
            script = Path(name, "driver.py")
            script.write_text(
                "import subprocess\n"
                "d = {'stdin': subprocess.DEVNULL, 'stdout': subprocess.DEVNULL,"
                " 'stderr': subprocess.DEVNULL}\n"
                "subprocess.Popen(['sleep', '300'], start_new_session=True, **d)\n")
            subprocess.run([sys.executable, str(script)], timeout=30, **DEVNULL)
            deadline = time.time() + 5
            kids_here = []
            while time.time() < deadline:
                kids_here = owner.child_pids(PROC, os.getpid())
                if kids_here:
                    break
                time.sleep(0.05)
            self.assertTrue(kids_here, "escapee was not adopted by subreaper")
            owner.terminate_owned({os.getpid()}, PROC, os.kill, time.sleep,
                                  exclude={os.getpid()}, grace_seconds=2.0)
            owner.reap_all(os.waitpid)
            self.assertEqual(owner.child_pids(PROC, os.getpid()), [])


class IdentityTests(unittest.TestCase):
    def test_parse_proc_stat_and_identity_roundtrip(self):
        state, pgid, starttime = owner.parse_proc_stat(
            "42 (my prog) S 7 42 42 0 -1 123 4 5 6 7 8 9 10 11 15 18 1 99 100")
        self.assertEqual((state, pgid, starttime), ("S", 42, 100))
        identity = owner.read_identity(os.getpid(), PROC)
        self.assertIsNotNone(identity)
        self.assertTrue(owner.identity_matches(identity, identity))
        self.assertIsNone(owner.read_identity(2 ** 22 + 7, PROC))

    def test_sha256_bytes_is_stable(self):
        self.assertEqual(
            owner.sha256_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")

    def test_signal_with_identity_check_refuses_drift_and_protected(self):
        child = spawn("sleep", "30")
        try:
            identity = owner.read_identity(child.pid, PROC)
            assert identity is not None
            wrong = owner.Identity(identity.pid, identity.pgid, identity.state,
                                   identity.starttime, "0" * 64)
            killed: list[int] = []
            outcome = owner.signal_with_identity_check(
                child.pid, signal.SIGKILL, wrong, PROC,
                lambda pid, sig: killed.append(pid))
            self.assertEqual(outcome, "refused_identity_drift")
            self.assertEqual(killed, [])
            self.assertIsNone(child.poll())
            outcome = owner.signal_with_identity_check(
                303603, signal.SIGKILL, identity, PROC,
                lambda pid, sig: killed.append(pid))
            self.assertEqual(outcome, "protected_never_signalled")
            self.assertEqual(killed, [])
        finally:
            guard.guard_child(self, child)

    def test_live_descendants_include_nested_and_owner_direct(self):
        outer = spawn("sleep", "30")
        try:
            members = owner.live_descendant_identities(
                PROC, {os.getpid()}, {os.getpid()})
            self.assertIn(outer.pid, members)
            self.assertEqual(members[outer.pid].pid, outer.pid)
        finally:
            guard.guard_child(self, outer)


class TerminateOwnedTests(unittest.TestCase):
    def test_kill_ignoring_nested_tree_is_swept_and_reaped(self):
        # Roots themselves are NOT sweep members (direct handles are ended
        # by the drain phases in production); the sweep must reach a
        # TERM-IGNORING child AND its nested grandchild via SIGKILL.
        with tempfile.TemporaryDirectory() as name:
            holder = Path(name, "holder.py")
            holder.write_text(
                "import signal, subprocess, time\n"
                "signal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
                "d = {'stdin': subprocess.DEVNULL, 'stdout': subprocess.DEVNULL,"
                " 'stderr': subprocess.DEVNULL}\n"
                "subprocess.Popen(['sleep', '300'], **d)\n"
                "time.sleep(300)\n")
            wrapper = spawn(sys.executable, "-c",
                            f"import subprocess, time\n"
                            f"p = subprocess.Popen({[sys.executable, str(holder)]!r}, "
                            "**{'stdin': subprocess.DEVNULL, 'stdout': subprocess.DEVNULL, "
                            "'stderr': subprocess.DEVNULL})\n"
                            "time.sleep(300)\n")
            deadline = time.time() + 10
            while (not owner.child_pids(PROC, wrapper.pid)
                   and time.time() < deadline):
                time.sleep(0.05)
            nested = owner.child_pids(PROC, wrapper.pid)
            self.assertTrue(nested)
            sweep = owner.terminate_owned({wrapper.pid}, PROC, os.kill,
                                          time.sleep, exclude={os.getpid()},
                                          grace_seconds=2.0)
            self.assertEqual(sweep["survivors"], [])
            # Reap the TRACKED handle first; a wildcard reap would steal its
            # exit status (production teardown uses this same order).
            wrapper.kill()
            wrapper.wait(timeout=5)
            kids.pump_adoptions(lambda line: None, PROC, tracked=set())
            self.assertEqual(owner.child_pids(PROC, os.getpid()), [])

    def test_protected_pids_are_excluded_from_member_freeze(self):
        members = owner.live_descendant_identities(PROC, {os.getpid()},
                                                   {os.getpid()})
        self.assertFalse(set(members) & set(owner.PROTECTED_PIDS))


class PumpTests(unittest.TestCase):
    def test_pump_reaps_untracked_zombie_without_stealing_status(self):
        with tempfile.TemporaryDirectory() as name:
            script = Path(name, "exit_now.py")
            script.write_text("import os\nos._exit(0)\n")
            untracked = spawn(sys.executable, str(script))
            tracked = spawn("sleep", "0.05")
            deadline = time.time() + 10
            while time.time() < deadline:
                identity = owner.read_identity(untracked.pid, PROC)
                if identity is None or identity.state == "Z":
                    break
                time.sleep(0.05)
            reaped = kids.pump_adoptions(lambda line: None, PROC,
                                         tracked={tracked.pid})
            self.assertGreaterEqual(reaped, 1)
            self.assertEqual(owner.read_identity(untracked.pid, PROC), None)
            status = tracked.wait(timeout=10)  # status NOT stolen
            self.assertEqual(status, 0)
            self.assertEqual(owner.child_pids(PROC, os.getpid()), [])


if __name__ == "__main__":
    unittest.main()
