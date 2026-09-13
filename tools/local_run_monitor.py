"""Bounded read-only polling for one explicitly identified remote run."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import time
from pathlib import Path
from typing import TypedDict


class Observation(TypedDict):
    utc_epoch: int
    state: str
    exit_status: str | None
    last_lines: list[str]
    error: str | None


def classify(returncode: int, stdout: str, stderr: str) -> Observation:
    now = int(time.time())
    if returncode != 0:
        return {
            "utc_epoch": now,
            "state": "unreachable",
            "exit_status": None,
            "last_lines": [],
            "error": stderr.strip() or f"ssh exited {returncode}",
        }
    lines = stdout.splitlines()
    marker = next((line for line in lines if line.startswith("EXIT_STATUS=")), None)
    exit_status = marker.removeprefix("EXIT_STATUS=") if marker else None
    state = "nonterminal" if exit_status is None else "terminal_success" if exit_status == "0" else "terminal_failure"
    return {
        "utc_epoch": now,
        "state": state,
        "exit_status": exit_status,
        "last_lines": [line for line in lines if not line.startswith("EXIT_STATUS=")][-8:],
        "error": None,
    }


def poll(host: str, start: str, exit_path: str, stdout: str) -> Observation:
    start_q, exit_q, stdout_q = map(shlex.quote, (start, exit_path, stdout))
    command = (
        f"cat {start_q}; if test -e {exit_q}; then printf '\\nEXIT_STATUS='; "
        f"cat {exit_q}; fi; tail -n 8 {stdout_q}"
    )
    completed = subprocess.run(
        ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", host, command],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    return classify(completed.returncode, completed.stdout, completed.stderr)


def atomic_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def run(args: argparse.Namespace) -> int:
    status_path = Path(args.status)
    history_path = Path(args.history)
    history: list[Observation] = []
    stop_epoch = int(args.deadline) + int(args.grace)
    while int(time.time()) <= stop_epoch:
        try:
            observation = poll(args.host, args.start, args.exit_path, args.stdout)
        except (OSError, subprocess.SubprocessError) as exc:
            observation = classify(255, "", f"{type(exc).__name__}: {exc}")
        history.append(observation)
        atomic_json(status_path, observation)
        atomic_json(history_path, history)
        if observation["state"].startswith("terminal_"):
            return 0
        remaining = stop_epoch - int(time.time())
        if remaining <= 0:
            break
        time.sleep(min(int(args.interval), remaining))
    final: Observation = {
        "utc_epoch": int(time.time()),
        "state": "monitor_deadline_reached",
        "exit_status": None,
        "last_lines": [],
        "error": None,
    }
    history.append(final)
    atomic_json(status_path, final)
    atomic_json(history_path, history)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", required=True)
    parser.add_argument("--start", required=True)
    parser.add_argument("--exit-path", required=True)
    parser.add_argument("--stdout", required=True)
    parser.add_argument("--deadline", type=int, required=True)
    parser.add_argument("--grace", type=int, default=900)
    parser.add_argument("--interval", type=int, default=300)
    parser.add_argument("--status", required=True)
    parser.add_argument("--history", required=True)
    return run(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
