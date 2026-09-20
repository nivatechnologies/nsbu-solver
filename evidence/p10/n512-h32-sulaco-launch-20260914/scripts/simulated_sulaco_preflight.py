#!/usr/bin/env python3
"""Simulated-Sulaco preflight-only readiness probe (LOCAL ONLY).

Runs the REAL preflight-only path (plan digest, stage hashes, resources,
deadline, barrier, create-only constraints) with ONLY the hostname gate
simulated as `sulaco`; every reading (MemAvailable, statvfs on the actual
output filesystem, deadline clock) is the real one.  preflight-only never
launches, never writes stage artifacts, and the launch path itself keeps the
un-spoofable `socket.gethostname()` gate (proven by the paired refusal
evidence run off-sulaco).  This script is evidence tooling, not a launcher.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import h32_launch_supervisor as driver  # noqa: E402


def main(argv: list[str]) -> int:
    driver.real_hostname = lambda: "sulaco"
    summary = (__doc__ or "").splitlines()[0].strip()
    print(f"{summary} (simulated hostname; all measurements real)", flush=True)
    return driver.main(["preflight-only", *argv])


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
