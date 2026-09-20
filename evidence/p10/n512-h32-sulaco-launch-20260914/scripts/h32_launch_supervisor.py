#!/usr/bin/env python3
"""One-off N512/M512 temporal-h32 SULACO capture launch supervisor driver.

  preflight-only  validate host, stage hashes, frozen-plan bytes, resources,
                  deadline, pinned watchdog environment and barrier readiness
                  WITHOUT launching or writing anything.
  launch          guarded run with the authenticated one-shot producer-side
                  barrier: after the durable clock32 commit the solver parks
                  and cannot begin attempt 2 until this supervisor validates
                  the exact 12 RHS / 7 hits / 5 misses / 0 steady counters,
                  finite local ratios <= 1.0, the identity fields (parsed by
                  whole-field equality, never substring) and the measured
                  resources, then explicitly releases or aborts exactly once.

From-rest only; no resume; create-only launch/decision/failure/completion
receipts; passing here does not qualify a PDE window (qualification=false).
The guarded flow itself lives in h32_run; this driver stays thin.
"""

from __future__ import annotations

import argparse
import os
import socket
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import IO, Iterable, Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))
import h32_children as kids  # noqa: E402
import h32_contract as contract  # noqa: E402
import h32_owner as owner  # noqa: E402
import h32_receipts as receipts  # noqa: E402
import h32_run as flow  # noqa: E402
from h32_run import (evaluate_decision, first_step_wall_seconds,  # noqa: F401,E402
                     parse_armed_line, write_receipt)
from h32_types import LogFn, Plan  # noqa: E402

__all__ = ["main", "evaluate_decision", "first_step_wall_seconds",
           "parse_armed_line", "write_receipt"]

DEFAULT_STAGE = "/mnt/niva-array/nsbu-solver/work/n512-h32-sulaco-capture-20260914-r5"
RECEIPT_NAMES = ("launch-receipt.json", "decision-receipt.json",
                 "failure-receipt.json", "completion-receipt.json")


@dataclass(frozen=True)
class LaunchConfig:
    """Fully-resolved, validated launch configuration the guarded flows
    receive; never a raw argparse Namespace."""

    stage: Path
    output: Path
    logs: Path
    plan_sha256: str


class _CliConfig(argparse.Namespace):
    """Typed argparse namespace: the declared attributes below match the
    parser's destinations and defaults exactly, so parse_args results are
    read with their real types instead of Any."""

    mode: str
    stage: str
    output: Optional[str]
    logs: Optional[str]
    plan_sha256: str


def real_hostname() -> str:
    return socket.gethostname()


def real_mem_available_bytes(proc_root: Path = Path("/proc")) -> int:
    try:
        for line in (proc_root / "meminfo").read_text().splitlines():
            if line.startswith("MemAvailable:"):
                return int(line.split()[1]) * 1024
    except (FileNotFoundError, ValueError, IndexError):
        return -1
    return -1


def real_rss_bytes(pid: int, proc_root: Path = Path("/proc")) -> int:
    try:
        for line in (proc_root / str(pid) / "status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except (FileNotFoundError, ValueError, IndexError):
        return -1
    return -1


def real_disk_free_bytes(path: Path) -> int:
    try:
        info = os.statvfs(str(path))
    except OSError:
        return -1
    return info.f_bavail * info.f_frsize


def _ancestor_pids(proc_root: Path, me: int) -> set[int]:
    """This process and every ancestor/descendant PID (the census never
    attributes OUR OWN process tree as an exclusivity conflict)."""
    relatives: set[int] = {me}
    pending = [me]
    children: dict[int, list[int]] = {}
    for entry in Path(proc_root).iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = entry.joinpath("stat").read_text().rsplit(") ", 1)[1].split()
            ppid = int(fields[1])
        except (OSError, IndexError, ValueError):
            continue
        children.setdefault(ppid, []).append(int(entry.name))
    while pending:
        pid = pending.pop()
        for child in children.get(pid, []):
            if child not in relatives:
                relatives.add(child)
                pending.append(child)
    while True:
        try:
            fields = (Path(proc_root) / str(me) / "stat").read_text().rsplit(") ", 1)[1].split()
            me = int(fields[1])
        except (OSError, IndexError, ValueError):
            break
        if me <= 1 or me in relatives:
            break
        relatives.add(me)
    return relatives


def host_process_observations(proc_root: Path = Path("/proc")) -> list[tuple[int, str]]:
    """HOST-WIDE census: every live process except this supervisor's own
    process tree, as (pid, cmdline text).  Feeds the pure
    contract.decide_process_exclusivity gate: one solver per host, proven by
    actual /proc state — never by hostname alone.
    """
    me = os.getpid()
    relatives = _ancestor_pids(proc_root, me)
    observations: list[tuple[int, str]] = []
    for entry in Path(proc_root).iterdir():
        if not entry.name.isdigit():
            continue
        pid = int(entry.name)
        if pid in relatives:
            continue
        try:
            raw = entry.joinpath("cmdline").read_bytes()
        except OSError:
            continue
        text = raw.replace(b"\0", b" ").decode("utf-8", errors="replace").strip()
        observations.append((pid, text))
    return observations


def open_exclusive(path: Path) -> IO[bytes]:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    return os.fdopen(os.open(str(path), flags, 0o644), "wb")


def measure_resources(plan: Plan, pid: int, barrier_dir_present: bool,
                      probe_path: Path = Path(".")
                      ) -> tuple[dict[str, int], list[str]]:
    """Sample actual resources on the REAL output filesystem and caps.

    Disk after the first durable capture must still cover the remaining 95
    measured capture bundles: floor = disk_bound - one_capture_bytes.
    """
    mem = real_mem_available_bytes()
    rss = real_rss_bytes(pid)
    disk = real_disk_free_bytes(Path(probe_path))
    reasons: list[str] = []
    cap = int(plan["resources"]["exact_capture_peak_bytes"])
    if rss < 0 or rss > cap:
        reasons.append(f"rss_outside_cap(rss={rss},cap={cap})")
    floor_mem = int(plan["resources"]["memory_floor_bytes"])
    if mem < 0:
        reasons.append("mem_available_unreadable")
    elif mem == 0:
        reasons.append("mem_available_broken_reading(0)")
    elif mem < floor_mem:
        # A meaningful memory floor AT RELEASE TIME: the release decision
        # itself is refused when free memory no longer covers the measured
        # exact peak + 32 GiB headroom floor.
        reasons.append(f"mem_available_below_release_floor(have={mem},floor={floor_mem})")
    per_capture = int(plan["capture"]["coefficient_bytes"]) + 77824
    disk_floor_after_first = int(plan["resources"]["disk_bound_bytes"]) - per_capture
    if disk < disk_floor_after_first:
        reasons.append(f"disk_free_below_remaining_capture_floor(have={disk},"
                       f"need={disk_floor_after_first})")
    if not barrier_dir_present:
        reasons.append("barrier_dir_missing")
    return {"mem_available_bytes": mem, "solver_rss_bytes": rss, "disk_free_bytes": disk}, reasons


def _open_log_files(logs: Path, names: Iterable[str]) -> dict[str, IO[bytes]]:
    handles: dict[str, IO[bytes]] = {}
    for name in names:
        try:
            handles[name] = open_exclusive(logs / name)
        except FileExistsError:
            for stream in handles.values():
                stream.close()
            raise contract.Refusal(68, f"log_exists({name})")
    return handles


def _host_census(plan: Plan, now: int) -> tuple[list[tuple[int, str]], list[str]]:
    del plan, now  # census is host-wide; the signature keeps (plan, now) for callers
    """Host-wide process-exclusivity census (actual /proc state)."""
    try:
        observations = host_process_observations()
    except OSError as error:
        return [], [f"host_process_census_unreadable({error})"]
    return observations, []


def _preflight_reasons(plan: Plan, stage: Path, logs: Path, output: Path,
                       now: int) -> tuple[list[str], Path]:
    probe = contract.resolve_probe_path(output, stage)
    barrier_dir = stage / "barrier"
    observations, census_reasons = _host_census(plan, now)
    reasons = contract.evaluate_readiness(
        plan, stage, hostname=real_hostname(), mem_available=real_mem_available_bytes(),
        disk_free=real_disk_free_bytes(probe), env_get=os.environ.get, now=now,
        output_path=output, logs_path=logs, barrier_dir=barrier_dir,
        receipt_paths=[stage / name for name in RECEIPT_NAMES],
        process_observations=observations)
    return reasons + census_reasons, probe


def run_launch(cfg: LaunchConfig, log: LogFn) -> int:
    stage, logs, output = cfg.stage, cfg.logs, cfg.output
    try:
        plan, observed = contract.read_frozen_plan(stage / "frozen-plan.json", cfg.plan_sha256)
    except contract.Refusal as refusal:
        log(f"refused: {refusal.reason}")
        return refusal.code
    plan["_observed_plan_sha256"] = observed
    log(f"plan_frozen_sha256_observed={observed}")
    reasons, _probe = _preflight_reasons(plan, stage, logs, output, int(time.time()))
    if reasons:
        for reason in reasons:
            log(f"refused: {reason}")
        return contract.refusal_exit_code(reasons)
    if not (stage / "barrier").is_dir():
        log(f"refused: barrier_dir_missing({stage / 'barrier'})")
        return 64
    log_files: dict[str, IO[bytes]] = {}
    try:
        if contract.logs_dir_state(logs) == "absent":
            logs.mkdir()
        log_files = _open_log_files(logs, ["solver.stdout", "solver.stderr", "watchdog.log"])
    except (FileExistsError, contract.Refusal) as error:
        reason = getattr(error, "reason", f"exclusive_create_race {error}")
        log(f"refused: {reason}")
        return 68
    if not owner.enable_child_subreaper():
        # Persistent adoption is MANDATORY: without a verified
        # PR_SET_CHILD_SUBREAPER promise, an orphaned descendant could
        # reparent to init and escape every owner.  Refuse truthfully.
        log("refused: child_subreaper_unavailable (PR_SET_CHILD_SUBREAPER not verified)")
        return 70
    children: dict[str, object] = {"solver": None, "watchdog": None,
                                   "identity": None, "watchdog_identity": None,
                                   "cleaned": False}
    ledger = receipts.ReceiptLedger()
    try:
        def body() -> int:
            return flow.run_guarded(children, plan, stage, logs, output, log_files,
                                    log, ledger, measure_resources)

        def cleanup() -> None:
            doubts = flow.teardown_and_receipt(children, stage, log, ledger)
            if doubts:
                log(f"teardown_doubts: {doubts}")

        def on_abort(descriptor: str) -> None:
            # A failure receipt for EVERY caught signal/exception, even when
            # the cleanup was clean: create-only, so an earlier adverse
            # receipt is preserved byte-for-byte rather than overwritten.
            status = write_receipt(stage, "failure-receipt.json",
                                   {"stage": "abort", "reason": descriptor}, log, ledger)
            if status != receipts.WRITTEN:
                log(f"abort_receipt_status={status} (existing bytes preserved)")

        code = kids.run_with_owned_cleanup(body, cleanup, log, on_abort=on_abort)
        if code in (74, 77):
            cleanup()
            if not (stage / "failure-receipt.json").exists():
                write_receipt(stage, "failure-receipt.json",
                              {"stage": "startup", "exit_code": code}, log, ledger)
        return code
    finally:
        for stream in log_files.values():
            try:
                stream.close()
            except OSError:
                pass


def run_preflight_only(cfg: LaunchConfig, log: LogFn) -> int:
    stage, output, logs = cfg.stage, cfg.output, cfg.logs
    try:
        plan, observed = contract.read_frozen_plan(stage / "frozen-plan.json", cfg.plan_sha256)
    except contract.Refusal as refusal:
        log(f"preflight-only stage={stage} REFUSE {refusal.reason}")
        return refusal.code
    reasons, probe = _preflight_reasons(plan, stage, logs, output, int(time.time()))
    barrier_dir = stage / "barrier"
    observations, census_reasons = _host_census(plan, int(time.time()))
    matches = contract.decide_process_exclusivity(observations)
    log(f"preflight-only stage={stage} host={real_hostname()} source={plan['source_binding']}")
    log(f"  plan_frozen_sha256_expected={cfg.plan_sha256}")
    log(f"  plan_frozen_sha256_observed={observed}")
    log(f"  mem_available_bytes={real_mem_available_bytes()}")
    log(f"  disk_free_bytes={real_disk_free_bytes(probe)} probe={probe}")
    log(f"  host_process_census scanned={len(observations)} solver_matches={len(matches)}"
        f" census_unreadable={bool(census_reasons)}")
    log(f"  deadline_epoch={plan['guard']['absolute_deadline_epoch']} now={int(time.time())}")
    log(f"  barrier_dir={barrier_dir} barrier_dir_present={barrier_dir.is_dir()}")
    log(f"  output_exists={output.exists()} logs_state={contract.logs_dir_state(logs)}")
    if reasons:
        for reason in reasons:
            log(f"  REFUSE {reason}")
        log("preflight-only: NOT READY")
        return 64
    log("preflight-only: READY (no launch performed)")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["preflight-only", "launch"])
    parser.add_argument("--stage", default=DEFAULT_STAGE)
    parser.add_argument("--output", default=None)
    parser.add_argument("--logs", default=None)
    parser.add_argument("--plan-sha256", required=True,
                        help="mandatory 64-hex digest the frozen plan bytes must hash to")
    return parser


def resolve_config(cfg: _CliConfig) -> LaunchConfig:
    """Normalize the parsed CLI defaults into concrete stage-relative paths."""
    stage = Path(cfg.stage)
    return LaunchConfig(stage=stage,
                        output=Path(cfg.output or str(stage / "output")),
                        logs=Path(cfg.logs or str(stage / "logs")),
                        plan_sha256=cfg.plan_sha256)


def main(argv: Optional[list[str]] = None) -> int:
    cfg: _CliConfig = build_parser().parse_args(argv, namespace=_CliConfig())
    config = resolve_config(cfg)

    def log(message: str) -> None:
        print(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())} {message}", flush=True)

    if cfg.mode == "preflight-only":
        return run_preflight_only(config, log)
    return run_launch(config, log)


if __name__ == "__main__":
    raise SystemExit(main())
