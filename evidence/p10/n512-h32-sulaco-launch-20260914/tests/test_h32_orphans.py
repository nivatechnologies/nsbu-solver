"""Orphaned-descendant and pipe-drain hang regressions (own module so every
file in this packet stays under 500 lines)."""

import json
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
import process_guard as guard  # noqa: E402  (VERIFIED subreaper + final reap)
from h32_owner import child_pids, read_identity  # noqa: E402


class OrphanSweepTests(unittest.TestCase):
    def test_detached_orphans_cannot_stall_a_pipe_drain_and_always_die(self):
        # REGRESSION (orphaned sleep/pipe-drain hang): a SIGKILLed driver
        # leaves its sleep descendants alive; because they never inherit the
        # OUTER capture pipes the runner's drain completes immediately, and
        # the unconditional external sweep provably leaves ZERO group members
        # behind — no descendant survives the driver's own violent death.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            script = root / "dirty_driver.py"
            script.write_text(
                "import json, os, signal, subprocess\n"
                "detached = {'stdin': subprocess.DEVNULL, 'stdout': subprocess.DEVNULL, 'stderr': subprocess.DEVNULL}\n"
                "pair = [subprocess.Popen(['sleep', '300'], start_new_session=True, **detached) for _ in range(2)]\n"
                f"open({str(root / 'spawn.json')!r}, 'w').write(json.dumps([p.pid for p in pair]))\n"
                "os.kill(os.getpid(), signal.SIGKILL)\n")
            started = time.monotonic()
            done = subprocess.run([sys.executable, str(script)],
                                  capture_output=True, text=True, timeout=30)
            drained = time.monotonic() - started
            self.assertEqual(done.returncode, -signal.SIGKILL)
            self.assertLess(drained, 10.0,
                            "pipe drain stalled behind an orphaned sleep")
            spawned = json.loads((root / "spawn.json").read_text())
            self.assertTrue(any(kids.group_members(Path("/proc"), pid)
                                for pid in spawned),
                            "orphan descendants must actually exist to prove the sweep")
            for pid in spawned:  # the unconditional sweep
                try:
                    os.killpg(os.getpgid(pid), signal.SIGKILL)
                except OSError:
                    pass
            deadline = time.time() + 10
            while any(kids.group_members(Path("/proc"), pid) for pid in spawned) \
                    and time.time() < deadline:
                time.sleep(0.1)
            for pid in spawned:
                self.assertEqual(kids.group_members(Path("/proc"), pid), [])
            # REGRESSION (zombie leak): the killed orphans were adopted by
            # this runner's VERIFIED subreaper, so they must be REAPED here
            # — an external census must never see a zombie child of us.
            deadline = time.time() + 10
            while guard.reap_adopted_zombies() and time.time() < deadline:
                time.sleep(0.05)
            self.assertEqual(guard.reap_adopted_zombies(), [])
            lingering = []
            for pid in child_pids(Path("/proc"), os.getpid()):
                identity = read_identity(pid, Path("/proc"))
                if identity is not None:
                    lingering.append((pid, identity.state))
            self.assertEqual(lingering, [],
                             f"zombie/live children survived: {lingering}")



if __name__ == "__main__":
    unittest.main()
