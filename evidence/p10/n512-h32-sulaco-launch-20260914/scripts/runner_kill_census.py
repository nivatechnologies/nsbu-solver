#!/usr/bin/env python3
"""Runner-violence census: SIGKILL each spawn-capable test runner MID-METHOD
(simulating timeout / violent leader death), then census what survives.

Attribution is decisive, not heuristic: this process VERIFIES
PR_SET_CHILD_SUBREAPER first, so every orphan of a killed runner is adopted
directly here and the descendant-reachability census from leak_census (which
COUNTS zombies) attributes every survivor.  A method is leak-capable when
killing its runner strands processes the external sweep has to clear.

Output: one JSONL record per (method, kill-delay) plus a summary; exit 1 if
any survivor ever appeared.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))
from h32_owner import enable_child_subreaper  # noqa: E402
from leak_census import (
    ProcInfo, cleanup_after, census_leaks, read_all)  # noqa: E402

PACKET_ROOT = Path(__file__).resolve().parents[1]


class _ProbeConfig(argparse.Namespace):
    """Typed argparse namespace mirroring the parser's destinations exactly,
    so parsed values keep their real ``Path``/``list[float]``/``str`` types."""

    tests: Path
    out: Path
    delays: list[float]
    methods: str

# Methods that spawn solver/watchdog/driver descendants.  Override with
# --methods to probe a different set (comma-separated unittest ids).
SPAWN_METHODS = [
    "test_h32_children.TeardownPathTests.test_kill_escalation_reaches_a_term_ignoring_grandchild",
    "test_h32_children.TeardownPathTests.test_reap_escalates_a_term_ignoring_direct_child",
    "test_h32_children.TeardownPathTests.test_success_path_stops_watchdog_and_reaps_both",
    "test_h32_children.TeardownPathTests.test_changed_identity_gets_no_signal_from_a_real_drain",
    "test_h32_children.TeardownPathTests.test_error_path_reaps_both_and_writes_failure_receipt",
    "test_h32_children.TeardownPathTests.test_sigint_path_reaps_both_and_writes_failure_receipt",
    "test_h32_owner.SubreaperTests.test_escapee_reparented_to_subreaper_when_driver_dies",
    "test_h32_owner.TerminateOwnedTests.test_kill_ignoring_nested_tree_is_swept_and_reaped",
    "test_h32_hardening.CensusToolTests.test_setsid_escapee_is_counted_and_swept",
    "test_h32_hardening.CensusToolTests.test_zombie_under_live_orphaned_intermediate_is_counted",
    "test_h32_children.RealProcessCleanupTests.test_leader_exit_still_drains_group_and_reaps_direct_child",
    "test_h32_children.InterruptionTests.test_watchdog_is_stopped_even_without_solver",
    "test_h32_children.TeardownPathTests.test_repaired_watchdog_keeps_escalating_after_leader_exit",
    "test_h32_launch_flow.ReleaseDeadlineTests.test_token_gate_never_releases_past_the_deadline",
    "test_h32_launch_flow.TimerBeforeSpawnTests.test_first_step_timer_starts_before_popen_so_spawn_latency_counts",
    "test_h32_orphans.OrphanSweepTests.test_detached_orphans_cannot_stall_a_pipe_drain_and_always_die",
]


def probe(label: str, kill_after: float, tests_dir: Path) -> list[ProcInfo]:
    pre = read_all()
    proc = subprocess.Popen([sys.executable, "-m", "unittest", "-v", label],
                            cwd=tests_dir, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL, preexec_fn=os.setsid)
    time.sleep(kill_after)
    if proc.poll() is None:
        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        pass
    time.sleep(0.25)
    return census_leaks(pre, os.getpid())


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tests", type=Path, default=PACKET_ROOT / "tests")
    parser.add_argument("--out", type=Path, default=Path("/tmp/h32-runner-kill"))
    parser.add_argument("--delays", type=float, nargs="+",
                        default=[0.8, 1.6, 3.0])
    parser.add_argument("--methods", type=str, default="")
    args: _ProbeConfig = parser.parse_args(argv, namespace=_ProbeConfig())
    if not enable_child_subreaper():
        print("CENSUS INVALID: PR_SET_CHILD_SUBREAPER unavailable")
        return 2
    args.out.mkdir(parents=True, exist_ok=True)
    methods = ([m for m in args.methods.split(",") if m]
               if args.methods else SPAWN_METHODS)
    jsonl = args.out / "runner-kill.jsonl"
    jsonl.unlink(missing_ok=True)
    bad = 0
    with open(jsonl, "a") as sink:
        for label in methods:
            for delay in args.delays:
                survivors = probe(label, delay, args.tests)
                stragglers = cleanup_after(survivors, os.getpid())
                record = {"method": label, "kill_after_s": delay,
                          "survivors": [i.record() for i in survivors]}
                sink.write(json.dumps(record) + "\n")
                sink.flush()
                if survivors or stragglers:
                    bad += 1
                    print(f"STRANDED@{delay}s\t{label}\t"
                          + json.dumps([i.record() for i in survivors])
                          + f"\tunconfirmed={stragglers}", flush=True)
    print(f"RUNNER-KILL CENSUS DONE probes={len(methods) * len(args.delays)} "
          f"stranded={bad} subreaper=VERIFIED")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
