"""Composite teardown: drain, identity-bound termination and complete reaping.

Extracted from ``h32_children`` so every file stays under 500 lines; the
public entry points are re-exported from ``h32_children`` unchanged.

* ``drain_group`` TERMs every live group member after re-reading its frozen
  identity IMMEDIATELY before its own TERM, KILLs survivors after the pinned
  grace with the same per-signal revalidation, and VERIFIES disappearance —
  members still present after KILL stay an ADVERSE preserved outcome.  A
  live-but-changed leader PID means the PID was recycled and the drain is
  refused entirely: NOTHING is signalled.  An absent or zombie leader keeps
  the group drainable: the pgid is still occupied by our own descendants.
* direct children are always reaped afterwards, escalating TERM→KILL on the
  direct handle (identity-bound; a drifted PINNED identity is never
  signalled) even when the group leader already exited;
* a watchdog's adverse exit status or adverse drain outcome can never be
  erased into a clean "reaped";
* the whole PPID descendant tree (setsid+closed-stdio escapees included) is
  swept via :mod:`h32_owner`, and every adopted child / untracked zombie is
  waitpid-reaped; a cleanup phase that raises NEVER suppresses the other
  phases and NEVER marks the teardown clean.
"""

from __future__ import annotations

import os
import signal
import time
from pathlib import Path
from typing import Callable, Optional, cast

import h32_owner as owner
from h32_children_core import GRACE_SECONDS
from h32_owner import PROTECTED_PIDS, Identity, identity_matches, read_identity
from h32_types import ChildHandle, LogFn

_ADVERSE_TOKENS = ("identity_changed_no_signal", "refused_not_owned_group",
                   "unconfirmed", "killed_group_survivors")


def group_member_identities(proc_root: Path, pgid: int) -> dict[int, Identity]:
    """Frozen PID/starttime/cmdline identity of every live group member."""
    members: dict[int, Identity] = {}
    try:
        entries = list(Path(proc_root).iterdir())
    except OSError:
        return members
    for entry in entries:
        if not entry.name.isdigit():
            continue
        identity = read_identity(int(entry.name), proc_root)
        if identity is None or identity.state == "Z":
            continue  # a zombie occupies no signals; its parent must reap it
        if identity.pgid == pgid and identity.pid not in PROTECTED_PIDS:
            members[identity.pid] = identity
    return members


def group_members(proc_root: Path, pgid: int) -> list[int]:
    """Every live PID that currently belongs to the process group."""
    return sorted(group_member_identities(proc_root, pgid))


def _group_confirmed_gone(proc_root: Path, pgid: int, sleep: Callable[[float], None],
                         deadline: float, now: Callable[[], float]) -> bool:
    while now() < deadline:
        if not group_member_identities(proc_root, pgid):
            return True
        sleep(0.05)
    return not group_member_identities(proc_root, pgid)


def drain_group(expected: Identity, proc_root: Path, kill: Callable[[int, int], None],
                sleep: Callable[[float], None], grace_seconds: int = GRACE_SECONDS,
                now: Callable[[], float] = time.monotonic,
                kill_confirm_seconds: float = 5.0) -> str:
    """TERM every group member, KILL survivors after the pinned grace — with
    an immutable-identity re-read IMMEDIATELY before every individual TERM
    and every individual KILL — and finally VERIFY group disappearance,
    preserving an adverse ``killed_group_survivors`` outcome when members
    are still there."""
    if expected.pgid != expected.pid or expected.pgid < 2 or expected.pid in PROTECTED_PIDS:
        return "refused_not_owned_group"
    live = read_identity(expected.pid, proc_root)
    if live is not None and live.state != "Z" and not identity_matches(live, expected):
        return "identity_changed_no_signal"
    frozen = group_member_identities(proc_root, expected.pgid)
    if not frozen:
        return "group_already_gone"
    for member, identity in frozen.items():
        # identity re-validation happens INSIDE signal_with_identity_check,
        # immediately before this member's own TERM.
        owner.signal_with_identity_check(member, signal.SIGTERM, identity, proc_root, kill)
    if _group_confirmed_gone(proc_root, expected.pgid, sleep, now() + grace_seconds, now):
        return "terminated_group"
    for member in group_member_identities(proc_root, expected.pgid):
        frozen_member = frozen.get(member)
        if frozen_member is None:
            continue  # joined after the freeze: never signalled blind
        # identity re-validation happens INSIDE signal_with_identity_check,
        # immediately before this member's own KILL.
        owner.signal_with_identity_check(member, signal.SIGKILL, frozen_member,
                                         proc_root, kill)
    if _group_confirmed_gone(proc_root, expected.pgid, sleep,
                             now() + kill_confirm_seconds, now):
        return "killed_group_terminated"
    return "killed_group_survivors"


def _bound_direct_step(child: ChildHandle, step: str, proc_root: Path,
                       log: LogFn, expected: Optional[Identity] = None) -> bool:
    """Freeze the direct child's live identity, re-validate it, THEN signal.

    A direct child that is still unreaped is provably our own process (its
    PID cannot be recycled until we reap it), but the signal is still only
    dispatched after the identity read twice agrees on a stable live
    state; an unstable/oscillating identity receives no signal.  When a
    PINNED identity exists and the live process has DRIFTED from it, the
    signal is refused unconditionally (same contract as the group drain)."""
    first = read_identity(child.pid, proc_root)
    if first is None or first.state == "Z":
        return False  # already gone or a zombie awaiting reap: no signal needed
    if expected is not None and not identity_matches(first, expected):
        log(f"direct_step_refused_pinned_identity_drift pid={child.pid} step={step}")
        return False
    second = read_identity(child.pid, proc_root)
    if second != first:
        log(f"direct_step_refused_unstable_identity pid={child.pid} step={step}")
        return False
    try:
        getattr(child, step)()
    except (OSError, AttributeError):  # pragma: no cover - reaped meanwhile
        return False
    return True


def reap(child: Optional[ChildHandle], log: LogFn, timeout: int = GRACE_SECONDS,
         proc_root: Path = Path("/proc"),
         expected: Optional[Identity] = None) -> bool:
    """True only when the direct child is confirmed reaped (exit observed).

    An unconfirmed direct child is ESCALATED: TERM, wait, then KILL — every
    signal identity-bound immediately before dispatch — so a leader that
    ignored the group drain still cannot outrun its supervisor."""
    if child is None:
        return True
    try:
        child.wait(timeout=timeout)
    except Exception:  # noqa: BLE001 - bounded reaping only
        pass
    if child.poll() is None:
        log(f"reap_escalate_direct_child pid={getattr(child, 'pid', '?')}")
        for step, wait_seconds in (("terminate", 5), ("kill", 5)):
            _bound_direct_step(child, step, proc_root, log, expected)
            try:
                child.wait(timeout=wait_seconds)
            except Exception:  # noqa: BLE001 - bounded reaping only; keep escalating
                pass
    if child.poll() is None:
        log(f"reap_incomplete pid={getattr(child, 'pid', '?')}")
    return child.poll() is not None


def drain_one(role: str, child: Optional[ChildHandle], identity: Optional[Identity],
              proc_root: Path, log: LogFn, grace_seconds: int) -> str:
    """Drain one owned process group with an immutable-identity re-check.

    A drifted or refused identity receives NOTHING: no group signal, no
    direct terminate.  The adverse outcome is returned unchanged so cleanup
    records the drift and the launch fails truthfully."""
    if child is None:
        return "absent"
    if child.poll() is not None:
        log(f"{role}_already_exited draining_group_members")
        if identity is not None:
            return drain_group(identity, proc_root, os.kill, time.sleep,
                               grace_seconds=grace_seconds)
        return "leader_exited_unconfirmed"
    if identity is None:
        # The pinned identity never stabilized: a LIVE, unreaped direct child
        # is still provably ours, so its identity is FROZEN NOW (double read)
        # and the live group drained against that fresh binding.  Descendants
        # must never survive an unconfirmed handshake.
        log(f"{role}_unconfirmed_identity_direct_term pid={child.pid}")
        if not _bound_direct_step(child, "terminate", proc_root, log):
            return "unconfirmed_direct_term"
        try:
            live_group = os.getpgid(child.pid)
        except OSError:
            return "unconfirmed_direct_term"
        if child.poll() is None and live_group == child.pid:
            live = read_identity(child.pid, proc_root)
            if live is not None:
                outcome = drain_group(live, proc_root, os.kill, time.sleep,
                                      grace_seconds=grace_seconds)
                log(f"{role}_unconfirmed_group_drain outcome={outcome}")
                return f"unconfirmed_direct_term+{outcome}"
        return "unconfirmed_direct_term"
    outcome = drain_group(identity, proc_root, os.kill, time.sleep,
                          grace_seconds=grace_seconds)
    log(f"{role}_drain outcome={outcome}")
    if outcome in ("identity_changed_no_signal", "refused_not_owned_group"):
        log(f"{role}_group_refused_no_signal pid={child.pid}")
    return outcome


def stop_watchdog(watchdog: Optional[ChildHandle], log: LogFn,
                  identity: Optional[Identity] = None,
                  proc_root: Path = Path("/proc"),
                  grace_seconds: int = GRACE_SECONDS,
                  natural_exit_seconds: float = 1.0,
                  sleep: Callable[[float], None] = time.sleep,
                  now: Callable[[], float] = time.monotonic) -> str:
    """Pinned-identity drain of the watchdog group, then reap confirmation.

    An ADVERSE drain outcome (group survivors after KILL, refused or drifted
    identity, unreaped direct child, adverse exit code) is ALWAYS preserved
    in the returned outcome — reaping the direct handle can never erase it
    into a plain "reaped".  A short, signal-free observation window first
    lets an already-deciding watchdog publish its OWN status instead of
    having a supervisor TERM overwrite it with -15 (bounded: production
    watchdogs only stop after leader loss or the deadline)."""
    if watchdog is None:
        return "absent"
    deadline = now() + natural_exit_seconds
    while watchdog.poll() is None and now() < deadline:
        sleep(0.01)
    outcome = drain_one("watchdog", watchdog, identity, proc_root, log, grace_seconds)
    if not reap(watchdog, log, proc_root=proc_root, expected=identity):
        return f"{outcome}+unreaped"
    code = getattr(watchdog, "returncode", None)
    if code and code > 0:  # signal death (negative) is the supervisor's own drain
        return f"reaped_adverse_exit_{code}"
    if any(token in outcome for token in _ADVERSE_TOKENS):
        return f"reaped_adverse_{outcome}"
    return "reaped"


def _tree_sweep(children: dict[str, object], log: LogFn, proc_root: Path,
                grace_seconds: int, owner_pid: int) -> owner.OwnedSweep:
    """Sweep every live PPID descendant of the solver, the watchdog AND the
    subreaper itself (adopted escapees), identity-bound end to end."""
    roots: set[int] = {owner_pid}
    for key in ("solver", "watchdog"):
        child = children.get(key)
        pid = getattr(child, "pid", None)
        if isinstance(pid, int):
            roots.add(pid)
    sweep = owner.terminate_owned(roots, proc_root, os.kill, time.sleep,
                                  exclude={os.getpid()},
                                  grace_seconds=grace_seconds)
    log(f"owned_tree_sweep outcome={sweep['outcome']} term={len(sweep['term_sent'])} "
        f"kill={len(sweep['kill_sent'])} drift_refused={len(sweep['drift_refused'])} "
        f"survivors={sweep['survivors']}")
    return sweep


def _reap_everything(children: dict[str, object], log: LogFn, proc_root: Path,
                     owner_pid: int) -> tuple[list[str], int]:
    """Confirm-reap both direct handles then waitpid-drain every adopted
    child and untracked zombie.  Returns (unconfirmed, reaped_count)."""
    unconfirmed: list[str] = []
    solver = cast("Optional[ChildHandle]", children.get("solver"))
    expected = cast("Optional[Identity]", children.get("identity"))
    if solver is not None and not reap(solver, log, proc_root=proc_root,
                                       expected=expected):
        unconfirmed.append("solver")
    tracked = {pid for pid in (getattr(solver, "pid", None),
                               getattr(children.get("watchdog"), "pid", None))
               if isinstance(pid, int)}
    adopted = owner.reap_all(os.waitpid)
    adopted += owner.reap_untracked_zombies(proc_root, owner_pid, tracked, os.waitpid)
    if adopted:
        log(f"adopted_reaped count={len(adopted)} pids={sorted(adopted)}")
    return unconfirmed, len(adopted)


def _fold_report(report: dict[str, object], outcome: str, watchdog_outcome: str,
                 sweep: Optional[owner.OwnedSweep], unconfirmed: list[str],
                 adopted_count: int) -> None:
    """Fold every phase outcome into the machine-readable cleanup_report:
    drift/adverse tokens from EITHER drain, the sweep's survivors, drift
    refusals and protected presences are all surfaced, never suppressed."""
    drift = any(token in outcome for token in _ADVERSE_TOKENS)
    if watchdog_outcome.startswith(("identity_changed", "refused_not_owned",
                                    "killed_group_survivors", "reaped_adverse")):
        drift = True
    adverse: list[str] = []
    if watchdog_outcome.startswith("reaped_adverse_exit"):
        adverse.append(watchdog_outcome)
    for label, value in (("solver", outcome), ("watchdog", watchdog_outcome)):
        if "killed_group_survivors" in value:
            adverse.append(f"{label}_group_survivors_after_kill")
        if "identity_changed_no_signal" in value or "refused_not_owned_group" in value:
            adverse.append(f"{label}_identity_refused_no_signal")
    if sweep is not None:
        if sweep["survivors"]:
            adverse.append(f"owned_tree_survivors({sweep['survivors']})")
        if sweep["drift_refused"]:
            adverse.append(f"owned_tree_drift_refused({sweep['drift_refused']})")
            drift = True
        if sweep["protected_skipped"]:
            adverse.append(f"owned_tree_protected_present({sweep['protected_skipped']})")
    report.update({
        "solver_outcome": outcome, "watchdog_outcome": watchdog_outcome,
        "unconfirmed": unconfirmed, "identity_drift": bool(drift),
        "adverse": adverse,
        "tree_outcome": sweep["outcome"] if sweep else "tree_sweep_error",
        "tree_survivors": sweep["survivors"] if sweep else [],
        "tree_drift_refused": sweep["drift_refused"] if sweep else [],
        "adopted_reaped": adopted_count,
    })


def cleanup_children(children: dict[str, object], log: LogFn,
                     proc_root: Path = Path("/proc"),
                     grace_seconds: int = GRACE_SECONDS,
                     owner_pid: Optional[int] = None) -> str:
    """Full teardown of EVERYTHING this process owns on every exit path.

    * every phase runs INDEPENDENTLY and UNCONDITIONALLY: a phase that raises
      never suppresses the other phases or the survivor report;
    * the teardown is only marked ``cleaned`` when every phase COMPLETED —
      a mid-phase exception leaves ``cleaned`` false so a retry redoes the
      incomplete work instead of falsely answering ``already_cleaned``;
    * any direct child that cannot be confirmed reaped, any group member or
      owned-tree descendant that survived the pinned grace, any identity
      drift/refusal and any cleanup exception is recorded in the
      machine-readable ``cleanup_report`` so the caller must write a
      create-only failure receipt.
    """
    if children.get("cleaned"):
        return cast("str", children.get("outcome", "already_cleaned"))
    owner_pid = os.getpid() if owner_pid is None else owner_pid
    cleanup_errors: list[str] = []
    phases_completed: list[str] = []
    report: dict[str, object] = {"cleanup_errors": cleanup_errors,
                                 "phases_completed": phases_completed}
    outcome = "no_solver"
    completed_all = True
    try:
        solver = cast("Optional[ChildHandle]", children.get("solver"))
        watchdog = cast("Optional[ChildHandle]", children.get("watchdog"))

        def _phase(name: str, run: Callable[[], object]) -> object:
            nonlocal completed_all
            try:
                value = run()
            except Exception as error:  # noqa: BLE001 - reported + retried later
                completed_all = False
                cleanup_errors.append(f"{name}:{error!r}")
                log(f"cleanup_phase_error {name}: {error!r}")
                return None
            phases_completed.append(name)
            return value

        drain_identity = cast("Optional[Identity]", children.get("identity"))
        outcome_val = _phase("solver_drain", lambda: drain_one(
            "solver", solver, drain_identity, proc_root, log, grace_seconds)
            if solver is not None else "no_solver")
        outcome = outcome_val if isinstance(outcome_val, str) else "solver_drain_error"
        watchdog_identity = cast("Optional[Identity]",
                                 children.get("watchdog_identity"))
        watchdog_outcome_val = _phase("watchdog_stop", lambda: stop_watchdog(
            watchdog, log, watchdog_identity, proc_root, grace_seconds))
        watchdog_outcome = (watchdog_outcome_val if isinstance(watchdog_outcome_val, str)
                            else "watchdog_stop_error")
        sweep_val = _phase("owned_tree_sweep",
                           lambda: _tree_sweep(children, log, proc_root, grace_seconds,
                                               owner_pid))
        sweep = cast("Optional[owner.OwnedSweep]",
                     sweep_val if isinstance(sweep_val, dict) else None)
        reaped_val = _phase("reap_everything", lambda: _reap_everything(
            children, log, proc_root, owner_pid))
        unconfirmed, adopted_count = cast(
            "tuple[list[str], int]",
            reaped_val if isinstance(reaped_val, tuple) else (["solver", "watchdog"], 0))
        if watchdog_outcome.endswith("+unreaped") and "watchdog" not in unconfirmed:
            unconfirmed.append("watchdog")
    finally:
        report.setdefault("solver_outcome", outcome)
        report.setdefault("watchdog_outcome", "teardown_aborted")
        report.setdefault("unconfirmed", ["solver", "watchdog"])
        report.setdefault("identity_drift", True)
        report.setdefault("adverse", ["cleanup_aborted"])
        report.setdefault("cleanup_errors", [])
        children["cleanup_report"] = report
    _fold_report(report, outcome, watchdog_outcome, sweep, unconfirmed,
                 adopted_count)
    children["outcome"] = outcome
    children["cleaned"] = completed_all
    return outcome
