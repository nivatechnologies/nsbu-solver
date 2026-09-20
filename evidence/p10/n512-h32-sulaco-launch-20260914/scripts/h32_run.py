"""Guarded launch flow: spawn, one-shot barrier, receipts and full teardown.

Lifted out of the thin driver so every production file stays under 500 lines,
and hardened per the r5 review: the watchdog script is sha256-pinned at
spawn, BOTH children are identity-frozen at spawn and reaped on every exit
path, the supervisor process is a VERIFIED child-subreaper (persistent
adoption; adopted processes are reaped during the run and swept with
identity-bound signals at teardown), no token or success receipt is written
while any doubt (uncertain receipt, identity drift/refusal, solver death,
pre-existing token, re-arm, cleanup exception, tree survivor) is
outstanding, a non-durable decision receipt hard-aborts BEFORE any release
token, and one single absolute first-step/release deadline is enforced after
every wait, validation and receipt (memory floor re-measured at the same
instant).
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path
from typing import IO, Callable, Optional, cast

sys.path.insert(0, str(Path(__file__).resolve().parent))
import h32_barrier as bar  # noqa: E402
import h32_bundles as bundles  # noqa: E402
import h32_children as kids  # noqa: E402
import h32_contract as contract  # noqa: E402
import h32_owner as owner  # noqa: E402
import h32_receipts as receipts  # noqa: E402
from h32_owner import Identity  # noqa: E402
from h32_types import ChildHandle, Env, LogFn, MeasureFn, Plan  # noqa: E402

BarrierFlow = bar.BarrierFlow
parse_armed_line = bar.parse_armed_line

ATTACH_CONFIRM_SECONDS = 120
TERMINAL_GRACE_SECONDS = 120
POLL_SECONDS = contract.PINNED_WATCHDOG_POLL_SECONDS
PROC = Path("/proc")


def first_step_wall_seconds(plan: Plan, now: int) -> int:
    deadline = int(plan["guard"]["absolute_deadline_epoch"])
    return max(0, min(int(plan["guard"]["first_step_wall_seconds"]), deadline - now))


def write_receipt(stage: Path, name: str, payload: dict[str, object], log: LogFn,
                  ledger: Optional[receipts.ReceiptLedger] = None) -> str:
    filled: dict[str, object] = dict(payload)
    filled.setdefault(
        "schema", f"p10-n512-m512-sulaco-temporal-h32-{name.replace('.json', '')}-v1")
    filled.setdefault("qualification", False)
    filled.setdefault("prepared_utc", time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
    status = receipts.create_json(Path(stage) / name, filled)
    if status != receipts.WRITTEN:
        log(f"receipt_uncertainty: {name} status={status}")
    if ledger is not None:
        ledger.record(name, status)
    return status


def evaluate_decision(plan: Plan, stdout_line_ok: bool, armed_ok: bool,
                      durable: tuple[bool, str],
                      resource_reasons: list[str]) -> dict[str, object]:
    reasons: list[str] = []
    if not stdout_line_ok:
        reasons.append("stdout_first_step_gate_failed")
    if not armed_ok:
        reasons.append("armed_handshake_unauthenticated")
    if not durable[0]:
        reasons.append(f"durable_bundle_invalid({durable[1]})")
    reasons += resource_reasons
    return {"release": not reasons, "refusal_reasons": reasons}


def solver_release_gate(children: dict[str, object]) -> Optional[str]:
    """(b) Solver must still be alive with the immutable spawn-time identity."""
    solver = cast("Optional[ChildHandle]", children.get("solver"))
    identity = cast("Optional[Identity]", children.get("identity"))
    if solver is None or identity is None:
        return "solver_untracked"
    if solver.poll() is not None:
        return "solver_exited_before_token"
    if not kids.identity_matches(kids.read_identity(identity.pid, PROC), identity):
        return "solver_identity_drift_before_token"
    return None


def verify_watchdog_bytes(stage: Path, plan: Plan, log: LogFn) -> bool:
    """Pin the watchdog script by sha256 immediately before spawning it."""
    path = Path(stage) / "pgid-watchdog-v3.sh"
    try:
        observed = contract.sha256_file(path)
    except OSError as error:
        log(f"refused: watchdog_bytes_unreadable({error})")
        return False
    if observed != plan["watchdog_sha256"]:
        log(f"refused: watchdog_sha256_mismatch(expected={plan['watchdog_sha256']},"
            f"observed={observed})")
        return False
    return True


def spawn_solver(stage: Path, output: Path, log_files: dict[str, IO[bytes]],
                 env: Env, plan: Plan, children: dict[str, object],
                 log: LogFn) -> bool:
    now = int(time.time())
    children["first_step_timer_start"] = time.monotonic()
    wall = first_step_wall_seconds(plan, now)
    wall_deadline = now + wall
    children["wall_deadline"] = wall_deadline
    # ONE absolute deadline governs the whole first-step/release pipeline.
    children["release_deadline"] = wall_deadline
    with kids.deferred_signals():
        solver = subprocess.Popen(
            [str(stage / "solver"), "run", str(output)],
            stdout=log_files["solver.stdout"], stderr=log_files["solver.stderr"],
            stdin=subprocess.DEVNULL, env=env, start_new_session=True,
            preexec_fn=kids.make_preexec(int(plan["resources"]["address_space_limit_bytes"])),
        )
        children["solver"] = solver
        identity = kids.stable_identity(solver.pid, PROC, time.sleep)
        if identity is not None:
            children["identity"] = identity
    if children.get("identity") is None:
        log("refused: solver identity handshake failed")
        return False
    return True


def spawn_watchdog(stage: Path, logs: Path, identity: Identity, deadline: int,
                   children: dict[str, object], log: LogFn) -> bool:
    if not verify_watchdog_bytes(stage, plan_of(children), log):
        return False
    with kids.deferred_signals():
        watchdog = subprocess.Popen(
            ["/bin/sh", str(stage / "pgid-watchdog-v3.sh"), str(identity.pid),
             str(identity.pgid), str(identity.starttime), identity.cmdline_sha256,
             str(deadline), str(logs / "watchdog.log")],
            stdout=_open_exclusive(logs / "watchdog.stdout"),
            stderr=_open_exclusive(logs / "watchdog.stderr"),
            stdin=subprocess.DEVNULL, env=contract.pinned_watchdog_env(os.environ),
            start_new_session=True, preexec_fn=kids.restore_child_signal_mask,
        )
        children["watchdog"] = watchdog
        children["watchdog_identity"] = kids.stable_identity(
            watchdog.pid, PROC, time.sleep, samples=40)
    if children.get("watchdog_identity") is None:
        log("refused: watchdog stable identity handshake failed; terminating child")
        return False
    if not _wait_started(watchdog, logs / "watchdog.log", identity.pid, identity.pgid,
                         time.time() + ATTACH_CONFIRM_SECONDS):
        log("refused: watchdog did not confirm start; terminating unguarded child")
        return False
    log(f"guarded_run solver_pid={identity.pid} pgid={identity.pgid} "
        f"starttime={identity.starttime} cmdline_sha256={identity.cmdline_sha256}")
    return True


def plan_of(children: dict[str, object]) -> Plan:
    return cast(Plan, children["plan"])


def _open_exclusive(path: Path) -> IO[bytes]:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    return os.fdopen(os.open(str(path), flags, 0o644), "wb")


def _wait_started(watchdog: ChildHandle, log_path: Path, solver_pid: int,
                  pgid: int, deadline: float) -> bool:
    while time.time() < deadline:
        try:
            lines = log_path.read_text(errors="replace").splitlines()
        except FileNotFoundError:
            lines = []
        if kids.watchdog_started(lines, solver_pid, pgid):
            return True
        if watchdog.poll() is not None:
            return False
        time.sleep(0.5)
    return False


def _tracked_pids(children: dict[str, object]) -> set[int]:
    return {pid for pid in (getattr(children.get(key), "pid", None)
                            for key in ("solver", "watchdog"))
            if isinstance(pid, int)}


def teardown_and_receipt(children: dict[str, object], stage: Path, log: LogFn,
                         ledger: Optional[receipts.ReceiptLedger] = None,
                         ) -> list[str]:
    """Terminate+reap BOTH children AND the whole adopted descendant tree;
    failure receipt if any member is doubtful.

    Returns the list of doubts (empty on a clean full teardown).  Called on
    the success, error and interruption paths alike; adverse drain outcomes
    (group/tree survivors after KILL, identity refusals, watchdog adverse
    exit and cleanup-phase exceptions) are preserved."""
    kids.cleanup_children(children, log, proc_root=PROC)
    report = cast("dict[str, object]", children.get("cleanup_report", {}))
    unconfirmed = cast("list[str]", report.get("unconfirmed", []))
    doubts = [f"member_unconfirmed({name})" for name in unconfirmed]
    if report.get("identity_drift"):
        doubts.append(f"identity_drift(solver={report.get('solver_outcome')},"
                      f"watchdog={report.get('watchdog_outcome')})")
    adverse = cast("list[str]", report.get("adverse", []))
    doubts += [f"adverse({item})" for item in adverse]
    cleanup_errors = cast("list[str]", report.get("cleanup_errors", []))
    doubts += [f"cleanup_error({item})" for item in cleanup_errors]
    if report.get("tree_survivors"):
        doubts.append(f"owned_tree_survivors({report['tree_survivors']})")
    if doubts:
        write_receipt(stage, "failure-receipt.json",
                      {"stage": "teardown", "doubts": doubts, "report": report},
                      log, ledger)
    return doubts


def _await_first_step_gate(logs: Path, plan: Plan, children: dict[str, object],
                           release_deadline: float,
                           sleep: Callable[[float], None] = time.sleep,
                           ) -> tuple[bool, str, Optional[str]]:
    """Poll stdout for the committed first-step line until the ONE absolute
    release deadline; validates the first committed line through the bundle
    gate; returns (gate_ok, reason, state_hash)."""
    gate_ok, gate_reason, state_hash = False, "first_step_line_absent", None
    tracked = _tracked_pids(children)
    solver = cast("ChildHandle", children.get("solver"))
    while time.time() < release_deadline:
        owner.pump_adoptions(log=lambda _m: None, proc_root=PROC, tracked=tracked)
        try:
            text = (logs / "solver.stdout").read_text(errors="replace")
        except FileNotFoundError:
            text = ""
        for line in text.splitlines():
            if bundles.is_committed_first_step(line, plan):
                gate_ok, gate_reason, state_hash = bundles.gate_first_step_line(line, plan)
                break
        if gate_ok or solver.poll() is not None:
            return gate_ok, gate_reason, state_hash
        sleep(POLL_SECONDS)
    return gate_ok, gate_reason, state_hash


def _release_check(plan: Plan, flow: BarrierFlow, output: Path,
                   children: dict[str, object], state_hash: str,
                   ledger: receipts.ReceiptLedger, decision: dict[str, object],
                   release_deadline: float) -> list[str]:
    """Re-check every release doubt and re-apply the ONE absolute deadline;
    mutates the decision and returns the doubts."""
    if not decision["release"]:
        return []
    doubts = release_doubts(plan, flow, output, children, state_hash, ledger)
    release, extra = bar.finalise_release_decision(bool(decision["release"]), doubts,
                                                   time.time(), release_deadline)
    decision["release"] = release
    reasons = cast("list[str]", decision["refusal_reasons"])
    reasons += extra
    decision["refusal_reasons"] = reasons
    return doubts


def _issue_barrier_decision(flow: BarrierFlow, decision: dict[str, object],
                            state_hash: str, ledger: receipts.ReceiptLedger,
                            release_deadline: float,
                            children: dict[str, object]) -> tuple[str, str]:
    """At most ONE create-only token per attempt; never over a pre-existing
    token; the abort decision still issues its one abort token when allowed."""
    status, reason = "not_attempted", "no_token_permitted"
    preexisting = bar.existing_tokens(flow.dir)
    if decision["release"]:
        status, reason = flow.decide_and_issue(decision, state_hash, release_deadline)
    elif not preexisting and not flow.token_issued and solver_release_gate(children) is None:
        status, reason = flow.decide_and_issue(decision, state_hash)
    elif preexisting:
        reason = f"token_preexisting({','.join(preexisting)}): never overwritten"
    ledger.record(bar.RELEASE_FILE if decision["release"] else bar.ABORT_FILE, status)
    return status, reason


def guard_first_step(plan: Plan, flow: BarrierFlow, stage: Path, logs: Path,
                     output: Path, children: dict[str, object], log: LogFn,
                     ledger: receipts.ReceiptLedger,
                     measure: MeasureFn) -> tuple[int, Optional[str]]:
    release_deadline = cast(float, children["release_deadline"])
    identity = cast(Identity, children.get("identity"))
    gate_ok, gate_reason, state_hash = _await_first_step_gate(
        logs, plan, children, release_deadline)
    if not gate_ok or state_hash is None:
        log(f"refused: first_step_gate {gate_reason}")
        write_receipt(stage, "failure-receipt.json",
                      {"stage": "first_step_gate", "reason": gate_reason}, log, ledger)
        return 73, None
    armed_ok, armed_reason = flow.wait_armed(
        logs / "solver.stdout", state_hash, release_deadline)
    if not armed_ok:
        log(f"refused: barrier_armed {armed_reason}")
        write_receipt(stage, "failure-receipt.json",
                      {"stage": "barrier_armed", "reason": armed_reason}, log, ledger)
        return 78, None
    durable = bundles.crosscheck_durable_bundle(output, state_hash, plan)
    resources, resource_reasons = measure(plan, identity.pid, flow.dir.is_dir(),
                                          contract.resolve_probe_path(output, stage))
    decision = evaluate_decision(plan, True, True, durable, resource_reasons)
    elapsed = time.monotonic() - cast(float, children.get(
        "first_step_timer_start", time.monotonic()))
    doubts = _release_check(plan, flow, output, children, state_hash, ledger, decision,
                            release_deadline)
    decision.update(resources, first_step_state_sha256=state_hash,
                    first_step_elapsed_seconds=round(elapsed, 6),
                    release_doubts=doubts, release_deadline_epoch=release_deadline,
                    solver_pid=identity.pid, solver_pgid=identity.pgid)
    decision_status = write_receipt(stage, "decision-receipt.json", decision, log, ledger)
    if decision["release"] and decision_status != receipts.WRITTEN:
        decision["release"] = False
        refusal_reasons = cast("list[str]", decision["refusal_reasons"])
        refusal_reasons.append(f"decision_receipt_not_durable({decision_status})")
        decision["refusal_reasons"] = refusal_reasons
        log("refused: decision receipt not durable; hard abort before RELEASE")
    if decision["release"]:
        # re-check immediately BEFORE issuing: one-shot discipline is enforced
        # at the moment of the token, not merely at decision time.
        doubts = _release_check(plan, flow, output, children, state_hash, ledger, decision,
                                release_deadline)
    status, reason = _issue_barrier_decision(flow, decision, state_hash, ledger,
                                            release_deadline, children)
    log(f"barrier_decision release={decision['release']} reasons={decision['refusal_reasons']} "
        f"token_status={status} ({reason})")
    if not decision["release"]:
        write_receipt(stage, "failure-receipt.json",
                      {"stage": "barrier_decision",
                       "reasons": decision["refusal_reasons"]}, log, ledger)
        return 79, state_hash
    if status != receipts.WRITTEN:
        write_receipt(stage, "failure-receipt.json",
                      {"stage": "barrier_token", "reason": status}, log, ledger)
        return 79, state_hash
    return 0, state_hash


def release_doubts(plan: Plan, flow: BarrierFlow, output: Path,
                   children: dict[str, object], state_hash: str,
                   ledger: receipts.ReceiptLedger) -> list[str]:
    """Pre-RELEASE gates only: (a) no receipt uncertainty this attempt, (b) the
    solver is alive with its pinned identity, one-shot discipline (never a
    second token, never over a pre-existing token) and (c) the durable bundle
    still revalidates byte-for-byte at the moment of release."""
    doubts: list[str] = []
    refuse = ledger.refuse_reason()
    if refuse:
        doubts.append(refuse)
    gate = solver_release_gate(children)
    if gate:
        doubts.append(gate)
    if flow.token_issued:
        doubts.append("token_already_issued_this_attempt")
    preexisting = bar.existing_tokens(flow.dir)
    if preexisting:
        doubts.append(f"token_preexisting({','.join(preexisting)})")
    if not doubts and bundles.crosscheck_durable_bundle(output, state_hash, plan)[0] is False:
        doubts.append("bundle_revalidation_failed_before_token")
    return doubts


def finish(plan: Plan, flow: BarrierFlow, stage: Path, logs: Path, output: Path,
           children: dict[str, object], state_hash: Optional[str], log: LogFn,
           ledger: receipts.ReceiptLedger) -> int:
    deadline = int(plan["guard"]["absolute_deadline_epoch"])
    solver = cast(ChildHandle, children["solver"])
    tracked = _tracked_pids(children)
    while solver.poll() is None and int(time.time()) < deadline + TERMINAL_GRACE_SECONDS:
        owner.pump_adoptions(log, proc_root=PROC, tracked=tracked)
        violation = bar.one_shot_violation(
            (logs / "solver.stdout").read_text(errors="replace"),
            int(plan["guard"]["first_step_clock"]))
        if violation:
            break
        time.sleep(POLL_SECONDS)
    exit_code = solver.poll()
    doubts = teardown_and_receipt(children, stage, log, ledger)
    stdout_text = (logs / "solver.stdout").read_text(errors="replace")
    violation = bar.one_shot_violation(stdout_text, int(plan["guard"]["first_step_clock"]))
    if violation:
        log(f"refused: {violation}")
        write_receipt(stage, "failure-receipt.json",
                      {"stage": "barrier_one_shot_violation", "reason": violation},
                      log, ledger)
        return 79
    captures = bundles.count_committed_capture_dirs(output) if output.is_dir() else 0
    revalidated = (state_hash is not None and flow.released
                   and bundles.crosscheck_durable_bundle(output, state_hash, plan)[0])
    reaped_report = cast("dict[str, object]", children.get("cleanup_report", {}))
    complete = (exit_code == 0 and revalidated and doubts == []
                and ledger.refuse_reason() is None
                and bundles.terminal_reached(stdout_text)
                and bundles.complete_capture_set(output, plan)
                and reaped_report.get("unconfirmed", ["?"]) == [])
    if complete:
        status = write_receipt(stage, "completion-receipt.json", {
            "solver_exit_code": exit_code, "committed_capture_dirs": captures,
            "first_step_state_sha256": state_hash,
            "first_step_bundle_revalidated": True,
            "terminal_line_present": bundles.terminal_reached(stdout_text),
            "all_children_reaped": True}, log, ledger)
        log(f"complete captures={captures} exit={exit_code} completion_status={status}")
        return 0 if status == receipts.WRITTEN else 76
    write_receipt(stage, "failure-receipt.json", {
        "stage": "terminal", "solver_exit_code": exit_code,
        "committed_capture_dirs": captures,
        "terminal_line_present": bundles.terminal_reached(stdout_text),
        "complete_capture_set": bool(complete),
        "completion_doubts": doubts + ([ledger.refuse_reason()] if ledger.refuse_reason() else []),
        "first_step_revalidated": bool(revalidated), "released": flow.released}, log, ledger)
    log(f"incomplete captures={captures} exit={exit_code} terminal="
        f"{bundles.terminal_reached(stdout_text)}")
    return 76


def run_guarded(children: dict[str, object], plan: Plan, stage: Path, logs: Path,
                output: Path, log_files: dict[str, IO[bytes]], log: LogFn,
                ledger: receipts.ReceiptLedger, measure: MeasureFn) -> int:
    now = int(time.time())
    children["plan"] = plan
    flow = BarrierFlow(plan, stage, output, logs, log)
    env = flow.environment(dict(os.environ, **{
        contract.OPTIN_ENV: "1",
        contract.REVIEWED_HOST_ENV: str(plan["host"]).strip().lower()}), now)
    if not spawn_solver(stage, output, log_files, env, plan, children, log):
        return 77
    identity = cast(Identity, children["identity"])
    write_receipt(stage, "launch-receipt.json", {
        "solver_pid": identity.pid, "solver_pgid": identity.pgid,
        "proc_starttime": identity.starttime, "cmdline_sha256": identity.cmdline_sha256,
        "barrier_nonce_sha256": contract.sha256_bytes(flow.nonce.encode()),
        "frozen_plan_sha256": plan.get("_observed_plan_sha256", "absent"),
        "binary_sha256": plan["binary_sha256"], "host": plan["host"],
    }, log, ledger)
    if not spawn_watchdog(stage, logs, identity,
                          int(plan["guard"]["absolute_deadline_epoch"]), children, log):
        return 74
    code, state_hash = guard_first_step(plan, flow, stage, logs, output, children,
                                        log, ledger, measure)
    if code == 0:
        return finish(plan, flow, stage, logs, output, children, state_hash, log, ledger)
    teardown_and_receipt(children, stage, log, ledger)
    return code
