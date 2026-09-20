#!/usr/bin/env python3
"""Create-only freeze of the Sulaco temporal-h32 capture stage.

Refuses if anything already exists; never modifies the Baccus stage.  The
deadline is a fresh absolute proposal derived ONLY from measured Sulaco N512
evidence (one-attempt wall 1051.20 s, integration 954.894656332 s from
evidence/p10/n512-m512-sulaco-timing-20260913) and is marked proposed_for_root.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import TypedDict, cast

from h32_types import Plan

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import h32_contract as contract  # noqa: E402
import h32_json as hjson  # noqa: E402
import h32_snapshot as snap  # noqa: E402

REPO = Path("/mnt/niva-array/nsbu-solver/work/opencode-n512-temporal-prep-20260914")
PACKET = REPO / "evidence/p10/n512-h32-sulaco-launch-20260914"
STAGE = Path("/mnt/niva-array/nsbu-solver/work/n512-h32-sulaco-capture-20260914-r5")
BINARY = REPO / "evidence/p10/avx-scheduled-endpoint/harness/target/release" \
    / "p10-avx-scheduled-endpoint"
WATCHDOG = PACKET / "watchdog" / "pgid-watchdog-v3.sh"
IDENTITY_DOC = PACKET / "docs" / "identity-expected.json"
# RUN_SOURCE is the SHA-256 over the complete hashed source inventory
# (docs/source-sha256.list), never a hand-carried token: the frozen plan can
# only ever bind the exact bytes that produced the staged binary.
RUN_SOURCE_LIST = PACKET / "docs" / "source-sha256.list"

SULACO_ATTEMPT_WALL_SECONDS = 1051.20
SULACO_ATTEMPT_INTEGRATION_SECONDS = 954.894656332
ATTEMPTS = 96
GIB = 34359738368
DEADLINE_HOURS = 48  # fresh r3 proposal; re-freeze rather than extend silently


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 22), b""):
            digest.update(block)
    return digest.hexdigest()


class FreezeOutcome(TypedDict):
    plan: Plan
    plan_sha256: str


def budget() -> dict[str, int]:
    wall = math.ceil(2.0 * SULACO_ATTEMPT_WALL_SECONDS)
    integration = math.ceil(2.0 * SULACO_ATTEMPT_INTEGRATION_SECONDS)
    return {"first_step_wall_seconds": wall, "integration": integration,
            "deadline_seconds": DEADLINE_HOURS * 3600}


def profile_identity() -> dict[str, str]:
    """The exact staged production profile-identity field map, taken from the
    Rust writer's own identity document (never hand-typed)."""
    try:
        document = hjson.load_json_object(IDENTITY_DOC.read_text())
    except hjson.JsonNotObject as error:
        raise SystemExit("refused: identity document is not an object") from error
    pairs = document.get("fields")
    if not isinstance(pairs, list):
        raise SystemExit("refused: identity document lacks fields")
    entries = cast("list[object]", pairs)
    fields: dict[str, str] = {}
    for candidate in entries:
        if not isinstance(candidate, list):
            raise SystemExit("refused: identity field entry malformed")
        pair = cast("list[object]", candidate)
        if len(pair) != 2:
            raise SystemExit("refused: identity field entry malformed")
        key, value = pair[0], pair[1]
        if not isinstance(key, str) or not isinstance(value, str):
            raise SystemExit("refused: identity field entry malformed")
        fields[key] = value
    if document.get("identity_len") != len(snap.identity_text_from_map(fields)):
        raise SystemExit("refused: identity document is not internally exact")
    return fields


def create_exclusive(path: Path, data: bytes, mode: int = 0o644) -> None:
    fd = os.open(str(path), os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    with os.fdopen(fd, "wb") as stream:
        stream.write(data)


def freeze(now: int | None = None) -> FreezeOutcome:
    if STAGE.exists():
        raise SystemExit(f"refused: stage already exists ({STAGE})")
    if not BINARY.is_file():
        raise SystemExit("refused: release binary not built")
    freeze_epoch = int(time.time()) if now is None else now
    run_source = sha256_file(RUN_SOURCE_LIST)
    completed = subprocess.run([str(BINARY), "preflight", "."],
                               capture_output=True, text=True, timeout=600)
    preflight = completed.stdout
    if "preflight" not in preflight.lower():
        raise SystemExit("refused: preflight stdout not produced")
    STAGE.mkdir()
    (STAGE / "logs").mkdir()
    (STAGE / "barrier").mkdir()
    shutil.copy(BINARY, STAGE / "solver")
    os.chmod(STAGE / "solver", 0o755)
    shutil.copy(WATCHDOG, STAGE / "pgid-watchdog-v3.sh")
    os.chmod(STAGE / "pgid-watchdog-v3.sh", 0o755)
    create_exclusive(STAGE / "preflight.stdout", preflight.encode())
    exact_peak = _exact_peak(preflight)
    bounds = budget()
    identity_map = profile_identity()
    coefficient_bytes = 512 * 512 * 257 * 3 * 16
    snapshot_bytes = snap.expected_snapshot_size(
        snap.identity_text_from_map(identity_map), coefficient_bytes)
    plan = {
        "schema": contract.PLAN_SCHEMA,
        "host": "sulaco",
        "from_rest": True,
        "resume": "unsupported",
        "qualification": False,
        "accepted_windows": 0,
        "prepared_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(freeze_epoch)),
        "source_binding": "binary built from the frozen source inventory; see "
                          "docs/source-binding.json (RUN_SOURCE identity token)",
        "identity_source": run_source,
        "profile": "n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995",
        "schedule_identity": "h32-clocks0-through2048-then-h64-through4096",
        "attempt_schema": "p10-avx-scheduled-attempt-v3",
        "observer_state_schema": "p10-avx-n512-m512-h32-observer-state-v1",
        "capture_terminal_schema": "p10-avx-n512-m512-h32-endpoint-capture-v1",
        "binary_sha256": sha256_file(STAGE / "solver"),
        "watchdog_sha256": sha256_file(STAGE / "pgid-watchdog-v3.sh"),
        "preflight_sha256": sha256_file(STAGE / "preflight.stdout"),
        "guard": {
            "first_step_clock": 32,
            "first_step_bundle": "step-001-clock-0032",
            "rhs_calls": 12, "cache_hits": 7, "cache_misses": 5,
            "steady_allocations": 0, "maximum_local_error_ratio": 1.0,
            "maximum_first_step_integration_seconds": bounds["integration"],
            "first_step_wall_seconds": bounds["first_step_wall_seconds"],
            "absolute_deadline_epoch": freeze_epoch + bounds["deadline_seconds"],
            "minimum_launch_margin_seconds": 3600,
        },
        "profile_identity": identity_map,
        "capture": {
            "all_committed_states": ATTEMPTS, "epoch": 1,
            "coefficient_bytes": coefficient_bytes,
            "snapshot_bytes": snapshot_bytes,
        },
        "resources": {
            "exact_capture_peak_bytes": exact_peak,
            "memory_floor_bytes": exact_peak + GIB,
            "address_space_limit_bytes": 274877906944,
            "disk_bound_bytes": 310453075968,
            "disk_floor_bytes": 310453075968 + GIB,
            "probe_path": str(STAGE),
        },
        "barrier": {"decision_margin_seconds": 3600, "armed_receipt_seconds": 120},
        "deadline_status": "proposed_for_root",
        "budget_evidence": {
            "sulaco_one_attempt_wall_seconds": SULACO_ATTEMPT_WALL_SECONDS,
            "sulaco_one_attempt_integration_seconds": SULACO_ATTEMPT_INTEGRATION_SECONDS,
            "rule": "ceil(2x measured first-step cost); fresh 48 h absolute deadline "
                    "proposed_for_root (r5 re-freeze after final re-inventory); "
                    "re-freeze rather than extend",
            "identity_document": "docs/identity-expected.json (writer-derived)",
        },
    }
    if identity_map.get("source") != run_source:
        raise SystemExit("refused: staged production identity source != RUN_SOURCE")
    contract.validate_plan_shape(plan)
    validated = cast(Plan, plan)
    raw = (json.dumps(validated, indent=2) + "\n").encode()
    create_exclusive(STAGE / "frozen-plan.json", raw)
    return {"plan": validated, "plan_sha256": hashlib.sha256(raw).hexdigest()}


def _exact_peak(preflight: str) -> int:
    import re

    match = re.search(r"\btotal=(\d+) cap=(\d+)\b", preflight)
    if match is None or match.group(1) != match.group(2):
        raise SystemExit("refused: exact capture peak not stated as total=cap")
    return int(match.group(1))


if __name__ == "__main__":
    result = freeze()
    print(json.dumps({"stage": str(STAGE), "plan_sha256": result["plan_sha256"],
                      "deadline_epoch": result["plan"]["guard"]["absolute_deadline_epoch"]}))
