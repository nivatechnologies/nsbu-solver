"""Verified Linux child-subreaper ownership of every solver/watchdog descendant.

This is the production ownership core (blocker repair r5):

* ``enable_child_subreaper`` sets PR_SET_CHILD_SUBREAPER and VERIFIES it via
  PR_GET_CHILD_SUBREAPER; the launch refuses to start when the kernel will
  not promise persistent adoption, so an orphaned descendant can never
  escape to init.  Once enabled, every solver/watchdog descendant that
  outlives its parent — including one that called ``setsid()`` and closed
  all three stdio descriptors — is reparented to THIS process and therefore
  stays reachable by identity-bearing termination and ``waitpid``.
* every signal is identity-bound: the immutable PID/PGID/starttime/cmdline
  identity is re-frozen and re-validated IMMEDIATELY before every individual
  TERM and every KILL; a member whose identity drifted (or whose identity
  cannot be read as live and stable) is NEVER signalled and the sweep
  outcome records the refusal so launch/cleanup fail truthfully.
* all adopted children and untracked zombies are reaped via waitpid until
  none remain; ``pump_adoptions`` keeps reaping during the run so adopted
  processes cannot accumulate as zombies.
* protected PIDs are never bound or signalled.
"""

from __future__ import annotations

import ctypes
import hashlib
import os
import re
import signal
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Optional, TypedDict

PR_SET_CHILD_SUBREAPER = 36
PR_GET_CHILD_SUBREAPER = 37
# Fixed PIDs that belong to unrelated jobs; never bind to or signal them.
PROTECTED_PIDS = frozenset({303603, 112351})


def _load_libc() -> ctypes.CDLL:
    return ctypes.CDLL("libc.so.6", use_errno=True)


# Honest ctypes boundary: libc's prctl returns a C int and CDLL's default
# restype converts it to a Python int, so the status is consumed DIRECTLY
# (compared against 0 in an if-test) rather than laundered through a cast.
# Anything that is not the zero status — a negative errno or a non-integer
# from a foreign libc object — takes the refusal branch (fail closed).
def enable_child_subreaper(libc: Optional[ctypes.CDLL] = None) -> bool:
    """Set AND VERIFY PR_SET_CHILD_SUBREAPER; returns the VERIFIED state."""
    try:
        libc = libc if libc is not None else _load_libc()
        if libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
            return False
        return child_subreaper_active(libc)
    except (OSError, AttributeError):
        return False


def child_subreaper_active(libc: Optional[ctypes.CDLL] = None) -> bool:
    """PR_GET_CHILD_SUBREAPER == 1 (verified, not assumed)."""
    try:
        libc = libc if libc is not None else _load_libc()
        value = ctypes.c_int(0)
        if libc.prctl(PR_GET_CHILD_SUBREAPER, ctypes.byref(value), 0, 0, 0) != 0:
            return False
        return value.value == 1
    except (OSError, AttributeError):
        return False


# --- immutable process identity -------------------------------------------------


@dataclass(frozen=True)
class Identity:
    pid: int
    pgid: int
    state: str
    starttime: int
    cmdline_sha256: str


_STAT_AFTER_COMM = re.compile(r"\) (.*)$")


def parse_proc_stat(stat_text: str) -> tuple[str, int, int]:
    match = _STAT_AFTER_COMM.search(stat_text)
    if not match:
        raise ValueError("malformed_stat")
    fields = match.group(1).split()
    if len(fields) < 20:
        raise ValueError("short_stat")
    return fields[0], int(fields[2]), int(fields[19])


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def read_identity(pid: int, proc_root: Path) -> Optional[Identity]:
    proc_root = Path(proc_root)
    try:
        stat_text = (proc_root / str(pid) / "stat").read_text(encoding="utf-8", errors="replace")
        cmdline = (proc_root / str(pid) / "cmdline").read_bytes()
    except (FileNotFoundError, NotADirectoryError, PermissionError):
        return None
    try:
        state, pgid, starttime = parse_proc_stat(stat_text)
    except ValueError:
        return None
    return Identity(int(pid), pgid, state, starttime, sha256_bytes(cmdline))


def identity_matches(observed: Optional[Identity], expected: Identity) -> bool:
    return (
        observed is not None
        and observed.state != "Z"
        and observed.pid == expected.pid
        and observed.pgid == expected.pgid
        and observed.starttime == expected.starttime
        and observed.cmdline_sha256 == expected.cmdline_sha256
    )


# --- descendant discovery (PPID tree, not just the original process group) ------


def _stat_fields(entry: Path) -> Optional[list[str]]:
    try:
        return entry.joinpath("stat").read_text().rsplit(") ", 1)[1].split()
    except (OSError, IndexError):
        return None


def child_pids(proc_root: Path, parent_pid: int) -> list[int]:
    """Every live process whose PPID is ``parent_pid`` (zombies included)."""
    found: list[int] = []
    proc_root = Path(proc_root)
    try:
        entries = list(proc_root.iterdir())
    except OSError:
        return found
    for entry in entries:
        if not entry.name.isdigit():
            continue
        fields = _stat_fields(entry)
        if fields is None or len(fields) < 3:
            continue
        try:
            ppid = int(fields[1])
        except ValueError:
            continue
        if ppid == parent_pid:
            found.append(int(entry.name))
    return found


def live_descendant_identities(proc_root: Path, roots: set[int],
                               exclude: set[int]) -> dict[int, Identity]:
    """Frozen identity of every LIVE (non-zombie) transitive descendant of the
    root PIDs, including descendants reparented to the subreaper itself.

    Setsid+closed-stdio escapees remain in the PPID tree while their parent
    lives and become DIRECT children of the owner when the parent dies, so
    including the owner PID as a root makes the census complete without ever
    scanning the whole host.
    """
    members: dict[int, Identity] = {}
    frontier = list(roots)
    seen: set[int] = set()
    while frontier:
        parent = frontier.pop()
        for pid in child_pids(proc_root, parent):
            if pid in seen or pid in exclude:
                continue
            seen.add(pid)
            frontier.append(pid)
            if pid in PROTECTED_PIDS:
                continue
            identity = read_identity(pid, proc_root)
            if identity is not None and identity.state != "Z":
                members[pid] = identity
    return members


def _kill_quietly(kill: Callable[[int, int], None], pid: int, signum: int) -> bool:
    try:
        kill(pid, signum)
        return True
    except OSError:
        return False


def signal_with_identity_check(pid: int, signum: int, expected: Identity,
                               proc_root: Path,
                               kill: Callable[[int, int], None]) -> str:
    """Re-read the identity IMMEDIATELY before this one signal; on ANY drift
    or unreadable live state the signal is withheld."""
    if pid in PROTECTED_PIDS:
        return "protected_never_signalled"
    current = read_identity(pid, proc_root)
    if current is None or current.state == "Z":
        return "absent_or_zombie"
    if not identity_matches(current, expected):
        return "refused_identity_drift"
    return "sent" if _kill_quietly(kill, pid, signum) else "send_failed"


class OwnedSweep(TypedDict):
    term_sent: list[int]
    kill_sent: list[int]
    drift_refused: list[int]
    protected_skipped: list[int]
    survivors: list[int]
    outcome: str


def _term_wave(frozen: dict[int, Identity], proc_root: Path, kill: Callable[[int, int], None],
               report: OwnedSweep) -> None:
    for pid, identity in frozen.items():
        if pid in PROTECTED_PIDS:
            report["protected_skipped"].append(pid)
            continue
        verdict = signal_with_identity_check(pid, signal.SIGTERM, identity, proc_root, kill)
        if verdict == "sent":
            report["term_sent"].append(pid)
        elif verdict == "refused_identity_drift":
            report["drift_refused"].append(pid)


def _kill_wave(live: dict[int, Identity], frozen: dict[int, Identity], proc_root: Path,
               kill: Callable[[int, int], None], report: OwnedSweep) -> None:
    for pid, identity in live.items():
        if pid in PROTECTED_PIDS:
            report["protected_skipped"].append(pid)
            continue
        if frozen.get(pid) is None or not identity_matches(identity, frozen[pid]):
            report["drift_refused"].append(pid)  # drifted after TERM: never KILL
            continue
        # identity_matches above already re-validated starttime+cmdline NOW;
        # re-check once more against THIS enumeration sample before the KILL.
        verdict = signal_with_identity_check(pid, signal.SIGKILL, identity, proc_root, kill)
        if verdict == "sent":
            report["kill_sent"].append(pid)
        elif verdict == "refused_identity_drift":
            report["drift_refused"].append(pid)


def terminate_owned(roots: set[int], proc_root: Path,
                    kill: Callable[[int, int], None], sleep: Callable[[float], None],
                    exclude: set[int], grace_seconds: float = 60.0,
                    now: Callable[[], float] = time.monotonic,
                    kill_confirm_seconds: float = 5.0) -> OwnedSweep:
    """TERM then KILL every live owned descendant with per-signal identity
    revalidation; late joiners are frozen and TERMed too; the final member
    list is reported truthfully (survivors stay survivors)."""
    report: OwnedSweep = {"term_sent": [], "kill_sent": [], "drift_refused": [],
                          "protected_skipped": [], "survivors": [], "outcome": ""}
    frozen = live_descendant_identities(proc_root, roots, exclude)
    if not frozen:
        report["outcome"] = "tree_already_empty"
        return report
    _term_wave(frozen, proc_root, kill, report)
    deadline = now() + grace_seconds
    while now() < deadline:
        live = live_descendant_identities(proc_root, roots, exclude)
        if not live:
            report["outcome"] = "terminated_tree"
            return report
        for pid, identity in live.items():
            if pid in frozen:
                continue
            frozen[pid] = identity
            _term_wave({pid: identity}, proc_root, kill, report)
        sleep(0.05)
    survivors_live = live_descendant_identities(proc_root, roots, exclude)
    _kill_wave(survivors_live, frozen, proc_root, kill, report)
    deadline = now() + kill_confirm_seconds
    while now() < deadline:
        survivors_live = live_descendant_identities(proc_root, roots, exclude)
        if not survivors_live:
            break
        sleep(0.05)
    report["survivors"] = sorted(survivors_live)
    if survivors_live:
        report["outcome"] = "tree_survivors_after_kill"
    elif report["drift_refused"]:
        report["outcome"] = "tree_clean_after_drift_refusal"
    else:
        report["outcome"] = "killed_tree_terminated"
    return report


# --- persistent adoption and complete reaping ------------------------------------


def reap_all(waitpid: Callable[[int, int], tuple[int, int]]) -> list[int]:
    """waitpid(-1, WNOHANG) until no child remains; returns every reaped PID
    (zombies of adopted escapees are CONFIRMED GONE only here)."""
    reaped: list[int] = []
    while True:
        try:
            pid, _status = waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return reaped
        if pid == 0:
            return reaped
        reaped.append(pid)


def reap_untracked_zombies(proc_root: Path, parent_pid: int, tracked: set[int],
                           waitpid: Callable[[int, int], tuple[int, int]]) -> list[int]:
    """Reap zombie children that NO live Popen handle owns (adopted escapees).

    Tracked PIDs are excluded so a live Popen.handle can never have its exit
    status stolen; untracked zombies can only be reaped by this process.
    """
    reaped: list[int] = []
    for pid in child_pids(proc_root, parent_pid):
        if pid in tracked:
            continue
        identity = read_identity(pid, proc_root)
        if identity is None or identity.state != "Z":
            continue
        try:
            pid_done, _status = waitpid(pid, os.WNOHANG)
        except (ChildProcessError, OSError):
            continue
        if pid_done == pid:
            reaped.append(pid)
    return reaped


def pump_adoptions(log: Callable[[str], None], proc_root: Path = Path("/proc"),
                   tracked: Optional[set[int]] = None) -> int:
    """Bounded persistent-adoption pass for the RUNNING supervisor.

    Only UNTRACKED zombie children (adopted escapees) are reaped, and only by
    their own PID: a live Popen-managed direct child's exit status is NEVER
    stolen by a wildcard waitpid."""
    me = os.getpid()
    reaped = reap_untracked_zombies(proc_root, me, tracked or set(), os.waitpid)
    if reaped:
        log(f"adopted_reaped count={len(reaped)} pids={sorted(reaped)}")
    return len(reaped)
