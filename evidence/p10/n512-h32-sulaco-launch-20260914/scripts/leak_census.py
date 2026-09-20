#!/usr/bin/env python3
"""External VERIFIED-subreaper /proc leak census for the capture packet.

The census process enables and VERIFIES PR_SET_CHILD_SUBREAPER before any
job starts, so every descendant an escaping test leaves behind is adopted
directly by the census — the ppid walk below is decisive, not heuristic.

For each job (every unit test method, the FULL suite, and the selftest,
repeated ``--repeats`` times) it:

  1. snapshots every /proc identity BEFORE the job,
  2. runs the job in its own session with output to FILES (never a pipe an
     escaped descendant could hold open) under a hard timeout,
  3. snapshots AFTER and reports EVERY new process still descended from the
     census — ZOMBIES INCLUDED (a zombie is an unreaped leak; the previous
     census wrongly excluded state=Z),
  4. only THEN sweeps (kill group + SIGKILL) and reaps the census's own
     zombie children, never before the snapshot, and
  5. appends a JSONL record and exits non-zero on ANY leak, rc != 0, or
     subreaper failure.

Foreign system processes are never touched: only pids absent from the
pre-snapshot AND reparented under the census are considered ours.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
import unittest
from pathlib import Path
from typing import cast, Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))
from h32_owner import child_pids, enable_child_subreaper  # noqa: E402

PROC = Path("/proc")


class _CensusConfig(argparse.Namespace):
    """Typed argparse namespace mirroring the parser's destinations exactly,
    so parsed values keep their real ``Path``/``int`` types."""

    tests: Path
    out: Path
    timeout: int
    repeats: int


class ProcInfo:
    __slots__ = ("pid", "ppid", "pgid", "starttime", "state", "cmdline")

    def __init__(self, pid: int, ppid: int, pgid: int, starttime: int,
                 state: str, cmdline: bytes) -> None:
        self.pid = pid
        self.ppid = ppid
        self.pgid = pgid
        self.starttime = starttime
        self.state = state
        self.cmdline = cmdline

    def record(self) -> dict[str, object]:
        return {"pid": self.pid, "ppid": self.ppid, "pgid": self.pgid,
                "starttime": self.starttime, "state": self.state,
                "cmdline": _show(self.cmdline)}


def _show(cmdline: bytes) -> str:
    return " ".join(x.decode(errors="replace")
                    for x in cmdline.split(b"\0") if x) or "[none]"


def read_all(proc_root: Path = PROC) -> dict[int, ProcInfo]:
    snap: dict[int, ProcInfo] = {}
    for entry in proc_root.iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / "stat").read_text(
                errors="replace").rsplit(") ", 1)[1].split()
            raw = (entry / "cmdline").read_bytes()
        except (OSError, IndexError):
            continue
        try:
            snap[int(entry.name)] = ProcInfo(
                int(entry.name), int(fields[1]), int(fields[2]),
                int(fields[19]), fields[0], raw)
        except (IndexError, ValueError):
            continue
    return snap


def census_leaks(pre: dict[int, ProcInfo], self_pid: int) -> list[ProcInfo]:
    """New processes still parented (transitively) under the census or its
    runner job — ZOMBIES COUNT.  The reachability set is computed from the
    CURRENT /proc (not the pre-snapshot) so a descendant orphaned by the
    runner's death and reparented to the census subreaper is caught even
    though its ppid changed after the pre-snapshot."""
    reach = _descendant_set(self_pid)
    post = read_all()
    leaks: list[ProcInfo] = []
    for info in post.values():
        if info.pid == self_pid or info.pid in pre or info.pid not in reach:
            continue
        leaks.append(info)
    return leaks


def _descendant_set(root_pid: int) -> set[int]:
    """All pids transitively parented under root_pid in the CURRENT /proc."""
    out: set[int] = set()
    frontier = [root_pid]
    snapshot = read_all()
    children_of: dict[int, list[int]] = {}
    for info in snapshot.values():
        children_of.setdefault(info.ppid, []).append(info.pid)
    while frontier:
        pid = frontier.pop()
        for kid in children_of.get(pid, []):
            if kid not in out:
                out.add(kid)
                frontier.append(kid)
    return out


def sweep(pid: int) -> None:
    try:
        os.killpg(os.getpgid(pid), signal.SIGKILL)
    except OSError:
        pass
    try:
        os.kill(pid, signal.SIGKILL)
    except OSError:
        pass


def reap_own_zombies(self_pid: int) -> list[int]:
    reaped: list[int] = []
    for pid in child_pids(PROC, self_pid):
        try:
            state = (PROC / str(pid) / "stat").read_text(
                errors="replace").rsplit(") ", 1)[1].split()[0]
        except (OSError, IndexError):
            continue
        if state != "Z":
            continue
        try:
            done, _status = os.waitpid(pid, os.WNOHANG)
        except (ChildProcessError, OSError):
            continue
        if done == pid:
            reaped.append(pid)
    return reaped


def run_job(label: str, argv: list[str], tests_dir: Path, out_dir: Path,
            timeout_s: int) -> tuple[int, list[ProcInfo]]:
    out_path = out_dir / (label.replace("/", "_").replace(".", "_")
                         .replace(":", "_") + ".out")
    pre = read_all()
    with open(out_path, "w") as out:
        try:
            done = subprocess.run(argv, cwd=tests_dir, stdout=out,
                                  stderr=subprocess.STDOUT,
                                  timeout=timeout_s,
                                  preexec_fn=os.setsid)
            rc: int = done.returncode
        except subprocess.TimeoutExpired as exc:
            rc = -signal.SIGKILL
            timed_out_pid = cast("int | None", getattr(exc, "pid", None))
            if timed_out_pid:
                sweep(timed_out_pid)
    time.sleep(0.25)  # let adoption/reparenting settle BEFORE the snapshot
    leaks = census_leaks(pre, os.getpid())
    return rc, leaks


def cleanup_after(leaks: list[ProcInfo], self_pid: int) -> list[int]:
    """Post-snapshot cleanup only: kill survivors and reap census zombies.
    Returns pids still live after the sweep (never silent)."""
    for info in leaks:
        sweep(info.pid)
    deadline = time.time() + 8
    live: list[int] = []
    while time.time() < deadline:
        reap_own_zombies(self_pid)
        live = []
        for info in leaks:
            try:
                state = (PROC / str(info.pid) / "stat").read_text(
                    errors="replace").rsplit(") ", 1)[1].split()[0]
            except (OSError, IndexError):
                continue
            if state != "Z":
                live.append(info.pid)
        if not live:
            return []
        time.sleep(0.1)
    return live


def enumerate_methods(tests_dir: Path) -> list[str]:
    loader = unittest.TestLoader()
    suite = loader.discover(str(tests_dir), pattern="test_*.py")
    ids: list[str] = []

    def walk(s: unittest.TestSuite) -> None:
        for item in s:
            if isinstance(item, unittest.TestSuite):
                walk(item)
            else:
                ids.append(item.id())
    walk(suite)
    return ids


def build_jobs(tests_dir: Path) -> list[tuple[str, list[str]]]:
    jobs: list[tuple[str, list[str]]] = []
    for tid in enumerate_methods(tests_dir):
        jobs.append((tid, [sys.executable, "-m", "unittest", "-v", tid]))
    jobs.append(("FULL-suite",
                 [sys.executable, "-m", "unittest", "discover",
                  "-p", "test_*.py"]))
    jobs.append(("SELFTEST", [sys.executable, "selftest_guarded_launch.py"]))
    return jobs


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tests", type=Path,
                        default=Path(__file__).resolve().parents[1] / "tests")
    parser.add_argument("--out", type=Path,
                        default=Path("/tmp/h32-leak-census"))
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--repeats", type=int, default=2)
    args: _CensusConfig = parser.parse_args(argv, namespace=_CensusConfig())
    if not enable_child_subreaper():
        print("CENSUS INVALID: PR_SET_CHILD_SUBREAPER unavailable/verifies-off")
        return 2
    args.out.mkdir(parents=True, exist_ok=True)
    jsonl = args.out / "census.jsonl"
    jsonl.unlink(missing_ok=True)
    leaky = 0
    jobs = build_jobs(args.tests)
    with open(jsonl, "a") as sink:
        for repeat in range(args.repeats):
            for label, argv_job in jobs:
                full = f"r{repeat}:{label}"
                started = time.monotonic()
                rc, leaks = run_job(full, argv_job, args.tests, args.out,
                                    args.timeout)
                unconfirmed = cleanup_after(leaks, os.getpid())
                record = {"job": full, "rc": rc,
                          "elapsed_s": round(time.monotonic() - started, 1),
                          "leak_count": len(leaks),
                          "leaks": [info.record() for info in leaks],
                          "unconfirmed_after_sweep": unconfirmed}
                sink.write(json.dumps(record) + "\n")
                sink.flush()
                bad = bool(leaks) or rc != 0 or bool(unconfirmed)
                leaky += int(bad)
                print(f"{'LEAK/BAD' if bad else 'clean'}\trc={rc}\t{full}"
                      f"\tleaks={len(leaks)}\tunconfirmed={unconfirmed}",
                      flush=True)
                for info in leaks:
                    print(f"\tLEAK {info.record()}", flush=True)
    total = args.repeats * len(jobs)
    print(f"CENSUS DONE jobs={total} bad={leaky} "
          f"subreaper=VERIFIED zombie_states_counted=True")
    return 1 if leaky else 0


if __name__ == "__main__":
    raise SystemExit(main())
