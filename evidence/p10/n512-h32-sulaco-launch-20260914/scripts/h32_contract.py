"""Frozen Sulaco temporal-h32 contract: plan binding and readiness gates.

Pure logic only: no process is spawned, no signal sent and no file written.
Side effects live in the thin driver.  Every constant here is Sulaco-specific
and measured; nothing is inherited from the Baccus candidate's stage bytes.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Callable, Mapping, Optional, cast

sys.path.insert(0, str(Path(__file__).resolve().parent))
import h32_json as hjson  # noqa: E402
import h32_snapshot as snap  # noqa: E402
from h32_types import EnvGet, Plan  # noqa: E402, F401

PLAN_SCHEMA = "p10-n512-m512-sulaco-temporal-h32-capture-plan-v1"
MAX_PLAN_BYTES = 65536

# Fixed PIDs that belong to unrelated jobs; never bind to or signal them.
PROTECTED_PIDS = frozenset({303603, 112351})

STAGE_FILENAMES = {
    "solver": "binary_sha256",
    "pgid-watchdog-v3.sh": "watchdog_sha256",
    "preflight.stdout": "preflight_sha256",
}

WATCHDOG_POLL_ENV = "WATCHDOG_POLL_SECONDS"
WATCHDOG_GRACE_ENV = "WATCHDOG_GRACE_SECONDS"
PINNED_WATCHDOG_POLL_SECONDS = 5
PINNED_WATCHDOG_GRACE_SECONDS = 60

OPTIN_ENV = "NSBU_RUN_N512_TEMPORAL_CAPTURE"
REVIEWED_HOST_ENV = "NSBU_N512_TEMPORAL_REVIEWED_HOST"

BARRIER_DIR_ENV = "NSBU_N512_TEMPORAL_BARRIER_DIR"
BARRIER_NONCE_ENV = "NSBU_N512_TEMPORAL_BARRIER_NONCE"
BARRIER_SECRET_ENV = "NSBU_N512_TEMPORAL_BARRIER_SECRET"
BARRIER_CLOCK_ENV = "NSBU_N512_TEMPORAL_BARRIER_CLOCK"
BARRIER_DEADLINE_ENV = "NSBU_N512_TEMPORAL_BARRIER_DEADLINE"


class Refusal(Exception):
    """Raised to abort a launch with a fixed diagnostic exit code."""

    def __init__(self, code: int, reason: str) -> None:
        super().__init__(reason)
        self.code = code
        self.reason = reason


def sha256_file(path: Path, chunk: int = 1 << 20) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(chunk), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def read_frozen_plan(path: Path, expected_sha256: str) -> tuple[Plan, str]:
    """Hash and parse the SAME bounded bytes; the expectation is mandatory."""
    if not re.fullmatch(r"[0-9a-f]{64}", str(expected_sha256)):
        raise Refusal(64, f"plan_hash_expectation_malformed({expected_sha256!r})")
    try:
        with Path(path).open("rb") as stream:
            raw = stream.read(MAX_PLAN_BYTES + 1)
    except OSError as error:
        raise Refusal(67, f"plan_unreadable({error})") from error
    if len(raw) > MAX_PLAN_BYTES:
        raise Refusal(67, f"plan_oversized(limit={MAX_PLAN_BYTES})")
    observed = sha256_bytes(raw)
    if observed != expected_sha256:
        raise Refusal(
            67,
            f"plan_frozen_sha256_mismatch(expected={expected_sha256},observed={observed})",
        )
    try:
        parsed = hjson.load_json_object(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Refusal(67, f"plan_not_json({error})") from error
    except hjson.JsonNotObject:
        raise Refusal(67, "plan_not_object")
    validate_plan_shape(parsed)
    return cast(Plan, parsed), observed


def validate_plan_shape(raw: object) -> None:
    """Refuse any plan whose Sulaco identity is not internally exact.

    This IS the runtime shape validation; only after it passes may callers
    treat the parsed JSON as the Plan shape (the TypedDict documents the
    validated structure; nothing is trusted that this check has not run).
    """
    if not isinstance(raw, dict):
        raise Refusal(67, "plan_not_object")
    probe = cast("dict[object, object]", raw)
    checks: tuple[tuple[str, bool], ...] = (
        ("schema", probe.get("schema") == PLAN_SCHEMA),
        ("host", probe.get("host") == "sulaco"),
        ("qualification", probe.get("qualification") is False),
        ("resume", probe.get("resume") == "unsupported"),
        ("from_rest", probe.get("from_rest") is True),
    )
    failed = [f"{name}={probe.get(name)!r}" for name, ok in checks if not ok]
    if failed:
        raise Refusal(67, f"plan_shape_invalid({';'.join(failed)})")
    plan = cast(Plan, probe)
    guard = plan["guard"]
    if int(guard["first_step_clock"]) != 32 or int(plan["capture"]["all_committed_states"]) != 96:
        raise Refusal(67, "plan_shape_not_h32_96")
    resources = plan["resources"]
    if int(resources["memory_floor_bytes"]) < 241987189520:
        raise Refusal(67, "plan_memory_floor_below_measured_exact")
    deadline: object = guard["absolute_deadline_epoch"]
    if type(deadline) is not int:
        raise Refusal(67, f"plan_deadline_not_integer({deadline!r})")
    _validate_profile_identity(plan)


def _validate_profile_identity(plan: Plan) -> None:
    """The plan must freeze the COMPLETE production profile-identity map, and
    any snapshot_bytes claim must be DERIVED from that exact frozen identity
    (writer header + identity text + coefficient payload)."""
    _fields, reasons = snap.expected_identity_map(plan)
    if reasons:
        raise Refusal(67, "plan_profile_identity_invalid(" + ";".join(reasons) + ")")
    capture = plan["capture"]
    if "snapshot_bytes" in capture:
        identity = snap.identity_text_from_map(_fields)
        want = snap.expected_snapshot_size(identity, int(capture["coefficient_bytes"]))
        if int(capture["snapshot_bytes"]) != want:
            raise Refusal(67, f"plan_snapshot_bytes_not_derived(claim={capture['snapshot_bytes']},"
                              f"derived={want})")


def _stage_hash_field(plan: Plan, key: str) -> str:
    if key == "binary_sha256":
        return plan.get("binary_sha256", "")
    if key == "watchdog_sha256":
        return plan.get("watchdog_sha256", "")
    if key == "preflight_sha256":
        return plan.get("preflight_sha256", "")
    raise Refusal(67, f"unknown_stage_hash_field({key})")


def verify_stage(plan: Plan, stage_dir: Path) -> list[tuple[str, str, str]]:
    stage_dir = Path(stage_dir)
    mismatches: list[tuple[str, str, str]] = []
    for name, key in STAGE_FILENAMES.items():
        expected = _stage_hash_field(plan, key)
        path = stage_dir / name
        if not path.is_file():
            mismatches.append((name, expected, "missing"))
        else:
            observed = sha256_file(path)
            if observed != expected:
                mismatches.append((name, expected, observed))
    return mismatches


# --- Readiness decisions (pure) -------------------------------------------------


def decide_host(actual_hostname: str, plan: Plan) -> list[str]:
    want = str(plan["host"]).strip().lower()
    got = str(actual_hostname).strip().lower()
    return [] if want and got == want else [f"host_mismatch(expected={want},got={got})"]


def decide_optin(env_get: EnvGet, plan: Plan) -> list[str]:
    reasons: list[str] = []
    if env_get(OPTIN_ENV) != "1":
        reasons.append(f"opt_in_env_missing({OPTIN_ENV})")
    reviewed = env_get(REVIEWED_HOST_ENV)
    if reviewed != str(plan["host"]).strip().lower():
        reasons.append(f"reviewed_host_env_missing({REVIEWED_HOST_ENV})")
    return reasons


def decide_deadline(plan: Plan, now_epoch: int) -> list[str]:
    deadline = plan["guard"]["absolute_deadline_epoch"]
    minimum_margin = int(plan["guard"].get("minimum_launch_margin_seconds", 3600))
    if now_epoch >= deadline:
        return [f"deadline_not_future(deadline={deadline},now={now_epoch})"]
    if deadline - now_epoch < minimum_margin:
        return [f"deadline_margin_too_small(remaining={deadline - now_epoch},"
                f"minimum={minimum_margin})"]
    return []


def decide_resources(mem_available_bytes: int, disk_free_bytes: int, plan: Plan) -> list[str]:
    reasons: list[str] = []
    floor_mem = int(plan["resources"]["memory_floor_bytes"])
    floor_disk = int(plan["resources"]["disk_floor_bytes"])
    if mem_available_bytes == 0:
        # A zero MemAvailable reading is a broken kernel read, not free memory.
        reasons.append("mem_available_broken_reading(0)")
    elif mem_available_bytes < floor_mem:
        reasons.append(f"mem_available_below_floor(need={floor_mem},have={mem_available_bytes})")
    if disk_free_bytes < floor_disk:
        reasons.append(f"disk_below_floor(need={floor_disk},have={disk_free_bytes})")
    return reasons


def resolve_probe_path(output_path: Path, fallback: Path) -> Path:
    """Filesystem probe target = the real (nearest existing) ancestor of the
    ACTUAL output directory, never silently the cwd or the stage."""
    candidate = Path(output_path)
    for parent in (candidate, *candidate.parents):
        if parent.exists():
            return parent
    return Path(fallback)


def decide_exclusive(*paths: Optional[Path]) -> list[str]:
    reasons: list[str] = []
    for path in paths:
        if path is None:
            continue
        if Path(path).exists():
            reasons.append(f"target_exists({path})")
    return reasons


# Substring fingerprints of this packet's solver/watchdog/supervisor
# processes.  The preflight runs a HOST-WIDE /proc census (every live PID)
# and refuses when ANY foreign process running one of them is still alive:
# one solver per host, exclusively.
PROCESS_EXCLUSIVITY_PATTERNS = (
    "pgid-watchdog-v3.sh", "h32_launch_supervisor.py",
    "p10-avx-scheduled-endpoint", "/solver run",
)


def decide_process_exclusivity(observations: list[tuple[int, str]]) -> list[str]:
    """Pure decision over host process census observations.

    ``observations`` is ``[(pid, cmdline_text), ...]`` for processes that are
    NOT this supervisor or its own descendants (the census side excludes
    self/ancestors).  Any live process matching a solver/watchdog/supervisor
    fingerprint means another launch is active on this host: refuse.
    """
    reasons: list[str] = []
    for pid, cmdline in observations:
        matched = [pattern for pattern in PROCESS_EXCLUSIVITY_PATTERNS
                   if pattern in cmdline]
        if matched:
            reasons.append(f"host_exclusivity_violation(pid={pid},"
                           f"matched={matched[0]},cmdline={cmdline[:80]!r})")
    return reasons


def decide_watchdog_env(env_get: EnvGet) -> list[str]:
    reasons: list[str] = []
    for key, pinned in (
        (WATCHDOG_POLL_ENV, PINNED_WATCHDOG_POLL_SECONDS),
        (WATCHDOG_GRACE_ENV, PINNED_WATCHDOG_GRACE_SECONDS),
    ):
        value = env_get(key)
        if value is None or value == str(pinned):
            continue
        reasons.append(f"watchdog_env_conflict({key}={value},pinned={pinned})")
    return reasons


def pinned_watchdog_env(base: Mapping[str, str]) -> dict[str, str]:
    env = dict(base)
    env[WATCHDOG_POLL_ENV] = str(PINNED_WATCHDOG_POLL_SECONDS)
    env[WATCHDOG_GRACE_ENV] = str(PINNED_WATCHDOG_GRACE_SECONDS)
    return env


def decide_barrier_precondition(barrier_dir: Path) -> list[str]:
    """A prior armed receipt means a barrier handshake already ran: a fresh
    from-rest launch must never reuse a barrier directory."""
    armed = Path(barrier_dir) / "barrier-armed.json"
    return [f"barrier_armed_receipt_preexists({armed})"] if armed.exists() else []


def logs_dir_state(logs_dir: Path) -> str:
    logs_dir = Path(logs_dir)
    if not logs_dir.exists():
        return "absent"
    if not logs_dir.is_dir():
        return "not_dir"
    if any(logs_dir.iterdir()):
        return "nonempty"
    return "empty"


def evaluate_readiness(
    plan: Plan,
    stage_dir: Path,
    *,
    hostname: str,
    mem_available: int,
    disk_free: int,
    env_get: Callable[[str], Optional[str]],
    now: int,
    output_path: Path,
    logs_path: Path,
    barrier_dir: Path,
    receipt_paths: Optional[list[Path]] = None,
    process_observations: Optional[list[tuple[int, str]]] = None,
) -> list[str]:
    reasons: list[str] = []
    mismatches = verify_stage(plan, stage_dir)
    if mismatches:
        names = ",".join(name for name, _, _ in mismatches)
        reasons.append(f"stage_hash_mismatch({names})")
    reasons += decide_host(hostname, plan)
    reasons += decide_optin(env_get, plan)
    reasons += decide_deadline(plan, now)
    reasons += decide_resources(mem_available, disk_free, plan)
    reasons += decide_watchdog_env(env_get)
    reasons += decide_exclusive(output_path, *(receipt_paths or []))
    reasons += decide_process_exclusivity(process_observations or [])
    reasons += decide_barrier_precondition(barrier_dir)
    state = logs_dir_state(logs_path)
    if state in ("not_dir", "nonempty"):
        reasons.append(f"logs_dir_{state}")
    return reasons


def refusal_exit_code(reasons: list[str]) -> int:
    if any("host_mismatch" in r for r in reasons):
        return 65
    if any(r.startswith("stage_hash_mismatch") for r in reasons):
        return 67
    return 64
