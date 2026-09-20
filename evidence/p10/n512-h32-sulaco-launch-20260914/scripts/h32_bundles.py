"""First-step stdout gates and durable bundle validation for temporal h32.

Pure parse/verify logic over bounded text and small JSON reads.  Every
refusal is fail-closed; malformed input never defaults to "pass".
"""

from __future__ import annotations

import math
import re
import sys
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))
import h32_json as hjson  # noqa: E402
import h32_snapshot as snap  # noqa: E402
from h32_types import Plan, PlanCapture, PlanGuard  # noqa: E402

_INTEGER = re.compile(r"\d+")
_DECIMAL = re.compile(r"\d+(?:\.\d+)?")
_FLOAT = re.compile(r"[+-]?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?")
_STATE_HASH = re.compile(r'state_sha256=Some\("([0-9a-f]{64})"\)')
_STATE_KEY = re.compile(r"(?:^|\s)state_sha256=(\S+)")
_HITMISS = re.compile(r"(?:^|\s)cache_hit_miss=(\[[^\]]*\])")
_HITMISS_PARTS = re.compile(r"\[\s*(\d+)\s*,\s*(\d+)\s*\]")
_PUBLICATION = re.compile(r"(?:^|\s)publication=(\S+)")
_RATIOS = re.compile(r"(?:^|\s)ratios=\[([^\]]*)\]")
_HEX64 = re.compile(r"[0-9a-f]{64}")

REST_SCHEMA = "p10-avx-n384-rest-v1"


def validate_state_payload(state_bin: Path, identity: str, plan: Plan,
                           state_sha: str) -> list[str]:
    """Structured exact validation of state.bin against the Rust writer layout."""
    try:
        endpoint = snap.identity_endpoint(identity)
    except ValueError as error:
        return [str(error)]
    clocks = (int(plan["guard"]["first_step_clock"]), endpoint,
              int(plan["capture"]["epoch"]), 1)
    return snap.validate_snapshot_file(
        state_bin, identity=identity, clocks=clocks,
        coefficient_bytes=int(plan["capture"]["coefficient_bytes"]),
        state_sha256=state_sha)


def _tokens(line: str, key: str) -> list[str]:
    return re.findall(rf"(?:^|\s){re.escape(key)}=(\S+)", line)


def _unique_token(line: str, key: str) -> tuple[Optional[str], Optional[str]]:
    found = _tokens(line, key)
    if not found:
        return None, f"{key}_token_absent"
    if len(found) > 1:
        return None, f"{key}_token_duplicated(count={len(found)})"
    return found[0], None


def _unique_int_token(line: str, key: str) -> tuple[Optional[int], Optional[str]]:
    token, error = _unique_token(line, key)
    if error is not None:
        return None, error
    assert token is not None
    if not _INTEGER.fullmatch(token):
        return None, f"{key}_token_not_integer({token})"
    return int(token), None


def finite_ratio_list(text: str) -> Optional[list[float]]:
    """Parse a bracketed float list; every entry must be a finite value."""
    raw_parts = text.split(",")
    values: list[float] = []
    for part in raw_parts:
        token = part.strip()
        if not token or not _FLOAT.fullmatch(token):
            return None
        value = float(token)
        if not math.isfinite(value):
            return None
        values.append(value)
    return values or None


def _gate_counters(line: str, guard: PlanGuard) -> Optional[str]:
    """attempt/clock/integration/RHS/cache-counter field group."""
    attempt, error = _unique_int_token(line, "attempt")
    if error is not None:
        return error
    if attempt != 1:
        return "attempt_not_1"
    clock, error = _unique_int_token(line, "clock")
    if error is not None:
        return error
    if clock != int(guard["first_step_clock"]):
        return "clock_mismatch"
    integration, error = _unique_token(line, "integration_seconds")
    if error is not None:
        return error
    assert integration is not None
    if not _DECIMAL.fullmatch(integration):
        return "integration_not_finite"
    if float(integration) > int(guard["maximum_first_step_integration_seconds"]):
        return "integration_exceeds_limit"
    rhs, error = _unique_int_token(line, "rhs_timed_calls")
    if error is not None or rhs != int(guard["rhs_calls"]):
        return error or "rhs_calls_mismatch"
    hits = list(_HITMISS.finditer(line))
    if len(hits) != 1:
        return "cache_hit_miss_token_absent_or_duplicated"
    parts = _HITMISS_PARTS.fullmatch(hits[0].group(1))
    if parts is None or (int(parts.group(1)), int(parts.group(2))) != (
        int(guard["cache_hits"]), int(guard["cache_misses"]),
    ):
        return "cache_hit_miss_mismatch"
    return None


def _gate_ratios_and_publication(line: str,
                                guard: PlanGuard) -> Optional[str]:
    """ratios/steady-allocations/publication/state-hash field group."""
    ratios = list(_RATIOS.finditer(line))
    if len(ratios) != 1:
        return "ratios_token_absent_or_duplicated"
    values = finite_ratio_list(ratios[0].group(1))
    if values is None or len(values) != 2:
        return "ratios_malformed"
    if any(value > float(guard["maximum_local_error_ratio"]) for value in values):
        return "ratios_exceed_local_limit"
    steady, error = _unique_int_token(line, "steady_allocations")
    if error is not None or steady != 0:
        return error or "steady_allocations_nonzero"
    publications = _PUBLICATION.findall(line)
    if len(publications) != 1 or publications[0] != "Step":
        return "publication_not_step"
    state_keys = _STATE_KEY.findall(line)
    state_hashes = _STATE_HASH.findall(line)
    if len(state_keys) != 1 or len(state_hashes) != 1:
        return "state_hash_missing_or_malformed"
    return None


def gate_first_step_line(line: str,
                        plan: Plan) -> tuple[bool, str, Optional[str]]:
    """Validate the first committed stdout report for the armed clock.

    Returns (ok, reason, state_sha256).  Requires exact 12 RHS / 7 hits /
    5 misses / 0 steady allocations and finite local ratios <= plan limit.
    Every token must appear EXACTLY once and be parsed as a whole field
    (substrings of longer tokens are impossible by the anchored regexes).
    """
    guard = plan["guard"]
    failure = _gate_counters(line, guard)
    if failure is None:
        failure = _gate_ratios_and_publication(line, guard)
    if failure is not None:
        return False, failure, None
    state_hashes = _STATE_HASH.findall(line)
    return True, "first_step_pass", state_hashes[0]


def is_committed_first_step(line: str, plan: Plan) -> bool:
    marker = f"attempt=1 clock={plan['guard']['first_step_clock']} "
    return marker in line


# --- Durable JSON reads with duplicate-key refusal ------------------------------


def load_unique_json(path: Path,
                     maximum_bytes: int = 65536) -> dict[str, object]:
    raw = path.read_bytes()
    if len(raw) > maximum_bytes:
        raise ValueError(f"json_oversized({path.name},{len(raw)})")
    return hjson.load_unique_json_object(raw.decode("utf-8"), path.name)


def identity_has_required_fields(identity: str, plan: Plan) -> list[str]:
    """Exact whole-field checks; e.g. ``endpoint=40960`` is NOT accepted by
    the ``endpoint=4096`` contract (a substring check would have accepted)."""
    return snap.validate_identity_fields(identity, plan)


def validate_attempt(attempt: dict[str, object], plan: Plan) -> list[str]:
    guard = plan["guard"]
    expected: tuple[tuple[str, object], ...] = (
        ("schema", plan["attempt_schema"]),
        ("attempt", 1),
        ("attempted_from", 0),
        ("attempted_to", int(guard["first_step_clock"])),
        ("ticks", int(guard["first_step_clock"])),
        ("outcome", "committed"),
        ("rhs_calls", int(guard["rhs_calls"])),
        ("cache_hits", int(guard["cache_hits"])),
        ("cache_misses", int(guard["cache_misses"])),
        ("rhs_timed_calls", int(guard["rhs_calls"])),
        ("steady_allocations", 0),
        ("observer_seconds", None),
    )
    reasons = [f"attempt_{key}_mismatch({attempt.get(key)!r})"
               for key, value in expected if attempt.get(key) != value]
    integration = attempt.get("integration_seconds")
    if not isinstance(integration, (int, float)) or isinstance(integration, bool) \
            or not math.isfinite(float(integration)) \
            or float(integration) > int(guard["maximum_first_step_integration_seconds"]):
        reasons.append("attempt_integration_unbounded")
    for key in ("error_ratio_l2", "error_ratio_h1"):
        ratio = attempt.get(key)
        if not isinstance(ratio, (int, float)) or isinstance(ratio, bool) \
                or not math.isfinite(float(ratio)) \
                or float(ratio) > float(guard["maximum_local_error_ratio"]):
            reasons.append(f"attempt_{key}_not_finite_within_local_limit")
    identity = attempt.get("identity")
    if not isinstance(identity, str):
        reasons.append("attempt_identity_not_text")
    else:
        reasons += [f"attempt_{item}" for item in identity_has_required_fields(identity, plan)]
    return reasons


def validate_record(record: dict[str, object], plan: Plan,
                    state_sha: str) -> list[str]:
    capture: PlanCapture = plan["capture"]
    expected: tuple[tuple[str, object], ...] = (
        ("schema", plan["observer_state_schema"]),
        ("resumable", False),
        ("clock", int(plan["guard"]["first_step_clock"])),
        ("epoch", int(capture["epoch"])),
        ("accepted_steps", 1),
        ("coefficient_bytes", int(capture["coefficient_bytes"])),
        ("observation_status", "CapturedActualState"),
        ("offline_observer_node", False),
        ("qualification", False),
    )
    reasons = [f"record_{key}_mismatch({record.get(key)!r})"
               for key, value in expected if record.get(key) != value]
    observed_sha = record.get("state_sha256")
    if not isinstance(observed_sha, str) or not _HEX64.fullmatch(observed_sha):
        reasons.append("record_state_sha_malformed")
    elif state_sha and observed_sha != state_sha:
        reasons.append("record_state_sha_mismatch")
    identity = record.get("identity")
    if not isinstance(identity, str):
        reasons.append("record_identity_not_text")
    else:
        reasons += [f"record_{item}" for item in identity_has_required_fields(identity, plan)]
    return reasons


def crosscheck_durable_bundle(output_dir: Path, state_sha: str,
                              plan: Plan) -> tuple[bool, str]:
    """Verify the whole durable first-step bundle before any release."""
    clock = int(plan["guard"]["first_step_clock"])
    root = Path(output_dir)
    step = root / f"step-001-clock-{clock:04d}"
    if (root / f"step-001-clock-{clock:04d}.partial").exists():
        return False, "partial_bundle_present"
    if not step.is_dir():
        return False, f"missing_step_bundle({step.name})"
    try:
        attempt = load_unique_json(step / "attempt.json")
        record = load_unique_json(step / "record.json")
    except (OSError, ValueError, UnicodeDecodeError) as error:
        return False, f"unreadable_bundle_json({error})"
    reasons = validate_attempt(attempt, plan) + validate_record(record, plan, state_sha)
    identity = attempt.get("identity")
    if identity != record.get("identity"):
        reasons.append("bundle_identity_divergence(attempt!=record)")
    if reasons:
        return False, "bundle_invalid(" + ";".join(reasons[:6]) + ")"
    state_bin = step / "state.bin"
    if not state_bin.is_file():
        return False, "state_payload_missing"
    payload_reasons = validate_state_payload(state_bin, str(identity), plan, state_sha)
    if payload_reasons:
        return False, "state_payload_invalid(" + ";".join(payload_reasons[:6]) + ")"
    try:
        rest = load_unique_json(root / "rest.json")
    except (OSError, ValueError, UnicodeDecodeError) as error:
        return False, f"unreadable_rest_json({error})"
    if rest.get("schema") != REST_SCHEMA or rest.get("clock") != 0 \
            or rest.get("observation_status") != "RestExact":
        return False, "rest_json_mismatch"
    return True, "durable_first_step_pass"


# --- Completion bookkeeping ------------------------------------------------------


def expected_capture_names(plan: Plan) -> list[str]:
    clock = 0
    names: list[str] = []
    for index in range(1, int(plan["capture"]["all_committed_states"]) + 1):
        clock += 32 if clock < 2048 else 64
        names.append(f"step-{index:03}-clock-{clock:04d}")
    return names


def count_committed_capture_dirs(output_dir: Path) -> int:
    return sum(1 for entry in Path(output_dir).iterdir()
               if entry.is_dir() and re.fullmatch(r"step-\d{3}-clock-\d{4}", entry.name))


def complete_capture_set(output_dir: Path, plan: Plan) -> bool:
    root = Path(output_dir)
    if not root.is_dir():
        return False
    if any(entry.name.endswith(".partial") for entry in root.iterdir()):
        return False
    names = sorted(entry.name for entry in root.iterdir() if entry.is_dir())
    if names != sorted(expected_capture_names(plan)):
        return False
    return all((root / name / "state.bin").is_file()
               and (root / name / "record.json").is_file()
               and (root / name / "attempt.json").is_file()
               for name in names)


def terminal_reached(stdout_text: str) -> bool:
    return "terminal endpoint_capture_complete_offline_observer_required clock=4096" in stdout_text
