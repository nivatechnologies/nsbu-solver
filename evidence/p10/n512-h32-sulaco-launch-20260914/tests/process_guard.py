"""Shared test process guards.

Every runner process that imports this module becomes a VERIFIED child
subreaper (PR_SET_CHILD_SUBREAPER set and PR_GET_CHILD_SUBREAPER confirmed),
mirroring the production launch: any orphaned test descendant is reparented
to the runner instead of init and can therefore be terminated, reaped and
PROVEN gone.  In addition to the pipe-detached spawns and the unconditional
end-of-test group sweep, every runner now performs an unconditional FINAL
kill+waitpid drain (per test and at exit) so a stranded live descendant or
zombie is impossible by construction — the shipped external census counts
states including Z and must observe zero.
"""

import atexit
import os
import signal
import subprocess
import sys
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import h32_owner as owner  # noqa: E402

DETACHED = {"stdin": subprocess.DEVNULL, "stdout": subprocess.DEVNULL,
            "stderr": subprocess.DEVNULL}

# Verified subreaper state for this runner process.
SUBREAPER_ACTIVE = owner.enable_child_subreaper()

_ACTIVE: list[subprocess.Popen] = []


def live_tracked_pids() -> set[int]:
    """PIDs of still-live guarded Popen handles (their status belongs to the
    handle and is never stolen by a generic reap)."""
    return {child.pid for child in _ACTIVE if child.poll() is None}


def reap_adopted_zombies() -> list[int]:
    """Reap adopted zombie children nobody has a live handle for."""
    return owner.reap_untracked_zombies(Path("/proc"), os.getpid(),
                                        live_tracked_pids(), os.waitpid)


def kill_and_reap_remaining(deadline_seconds: float = 10.0) -> list[int]:
    """UNCONDITIONAL final drain: SIGKILL every remaining child group and
    waitpid-reap every child (zombies included) until none remain; returns
    any PID still present after the deadline (never silent)."""
    me = os.getpid()
    my_pgid = os.getpgid(me)
    for pid in owner.child_pids(Path("/proc"), me):
        try:
            # NEVER killpg our own group: a child sharing our pgid would make
            # the sweep SIGKILL the runner itself.
            child_pgid = os.getpgid(pid)
            if child_pgid != my_pgid:
                os.killpg(child_pgid, signal.SIGKILL)
        except OSError:
            pass
        try:
            os.kill(pid, signal.SIGKILL)
        except OSError:
            pass
    deadline = time.time() + deadline_seconds
    while time.time() < deadline:
        reap_adopted_zombies()
        try:
            while True:
                pid, _status = os.waitpid(-1, os.WNOHANG)
                if pid == 0:
                    break
        except ChildProcessError:
            pass
        reap_adopted_zombies()
        if not owner.child_pids(Path("/proc"), me):
            return []
        time.sleep(0.05)
    return owner.child_pids(Path("/proc"), me)


def _final_drain() -> None:
    remaining = kill_and_reap_remaining()
    if remaining:
        # Never suppressed: an unreaped descendant must be LOUD.
        print(f"PROCESS_GUARD_UNREAPED_CHILDREN {remaining}", file=sys.stderr,
              flush=True)


atexit.register(_final_drain)


def guard_child(test: unittest.TestCase, *children: subprocess.Popen) -> None:
    """UNCONDITIONAL end-of-test sweep (runs on success, failure and error):
    SIGKILL each child's whole group, reap the direct handle, then drain
    every adopted zombie so NO descendant and NO zombie can outlive the test
    on ANY adverse path.  Test spawns also run with DETACHED fds so a
    surviving descendant can never hold an external capture pipe open and
    stall the test runner."""

    def _sweep() -> None:
        my_pgid = os.getpgid(os.getpid())
        for child in children:
            try:
                child_pgid = os.getpgid(child.pid)
                if child_pgid != my_pgid:
                    os.killpg(child_pgid, signal.SIGKILL)
            except OSError:
                pass
            try:
                child.wait(timeout=5)
            except Exception:  # noqa: BLE001 - bounded sweep only
                pass
        reap_adopted_zombies()
        leftover = kill_and_reap_remaining(deadline_seconds=5.0)
        if leftover:
            raise AssertionError(f"unreaped test descendants: {leftover}")

    for child in children:
        if child not in _ACTIVE:
            _ACTIVE.append(child)
    test.addCleanup(_sweep)
