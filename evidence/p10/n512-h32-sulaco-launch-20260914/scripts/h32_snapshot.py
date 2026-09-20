"""Exact snapshot/bundle payload validation derived from the Rust writer.

The byte layout is DERIVED from ``harness/src/artifact.rs::write_snapshot``
(never guessed): magic ``P10AVXSNAP1\\0`` (12 B), identity length u64 LE,
identity bytes, then four u128 LE words in writer order ``elapsed, target,
epoch, accepted_steps``, then the little-endian coefficient words, and a
trailing SHA-256 digest taken over EXACTLY the coefficient bytes.  Identity
fields are parsed as ``key=value`` pairs joined by ``;`` and compared by
whole-field equality, so ``endpoint=40960`` can never satisfy the
``endpoint=4096`` contract the way a substring check would.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path
from typing import cast

sys.path.insert(0, str(Path(__file__).resolve().parent))
from h32_types import Plan  # noqa: E402

MAGIC = b"P10AVXSNAP1\0"
LENGTH_BYTES = 8
CLOCK_WORDS = 4
CLOCK_BYTES = 16
DIGEST_BYTES = 32
IDENTITY_OFFSET = len(MAGIC) + LENGTH_BYTES
CLOCK_BLOCK_BYTES = CLOCK_WORDS * CLOCK_BYTES
FIXED_BYTES = IDENTITY_OFFSET + CLOCK_BLOCK_BYTES + DIGEST_BYTES
SPARSE_SLACK_BYTES = 4096  # one filesystem block of rounding tolerance

REQUIRED_IDENTITY_KEYS = ("source", "case", "profile", "backend", "provider",
                          "observer_execution", "host_provenance", "schedule",
                          "endpoint", "maximum_attempts", "execution_cap",
                          "artifact_cap", "method", "schema", "attempt_schema",
                          "resume", "numa", "external_stop")
# The complete field set of the staged production writer's identity format
# (config::identity() under n512-m512-parallel-capture).  A frozen map with a
# different key set is drift in the plan itself, never "legitimate extras".
WRITER_IDENTITY_KEYS = (
    "source", "case", "profile", "backend", "library_source", "prototype_source",
    # timed_rhs::IDENTITY expands as "harness-timed-rhs-v1;clock=…;scope=…;
    # overhead=…" so the writer's identity carries these three provenance
    # fields between rhs_timer and retained (writer truth: config/identity.rs
    # + timed_rhs.rs IDENTITY; see docs/identity-expected.json).
    "provider", "rhs_w3", "force_w3", "rhs_timer", "clock", "scope", "overhead",
    "retained", "force_samples",
    "observer_force_samples", "observer_conservative", "observer_execution",
    "host_provenance", "sampling_workers", "rhs_w3_persistent_callers",
    "rhs_fft_helpers", "rhs_fft_total_workers", "provider_w3_persistent_callers",
    "provider_fft_helpers", "provider_fft_total_workers", "method", "schedule",
    "endpoint", "maximum_attempts", "advective_limit", "execution_cap",
    "artifact_cap", "schema", "attempt_schema", "resume", "numa", "external_stop",
)
IDENTITY_SEPARATORS = "=;\n\r"


def identity_fields(text: str) -> tuple[dict[str, str], list[str]]:
    """Parse ``key=value`` pairs; duplicated, malformed or empty keys fail."""
    reasons: list[str] = []
    fields: dict[str, str] = {}
    if not text:
        return {}, ["identity_empty"]
    for part in text.split(";"):
        if part.count("=") != 1:
            reasons.append(f"identity_field_malformed({part!r})")
            continue
        key, value = part.split("=", 1)
        if not key or not value:
            reasons.append(f"identity_field_incomplete({part!r})")
            continue
        if key in fields:
            reasons.append(f"identity_field_duplicated({key})")
            continue
        fields[key] = value
    return fields, reasons


def _parse_profile_map(raw: "dict[object, object]") -> tuple[dict[str, str], list[str]]:
    """Whole-field parse of the frozen profile map; malformed/duplicate parts
    are refusals, never silent skips."""
    reasons: list[str] = []
    fields: dict[str, str] = {}
    for key, value in raw.items():
        if not isinstance(key, str) or not isinstance(value, str) \
                or not key or not value:
            reasons.append(f"plan_profile_identity_field_malformed({key!r})")
            continue
        if any(ch in key or ch in value for ch in IDENTITY_SEPARATORS):
            reasons.append(f"plan_profile_identity_field_malformed({key!r})")
            continue
        if key in fields:
            reasons.append(f"plan_profile_identity_field_duplicated({key})")
            continue
        fields[key] = value
    return fields, reasons


def _check_profile_keyset(fields: dict[str, str], raw: "dict[object, object]",
                          ) -> list[str]:
    reasons: list[str] = []
    for key in REQUIRED_IDENTITY_KEYS:
        if key not in fields:
            reasons.append(f"plan_profile_identity_incomplete({key})")
    if list(fields) != list(WRITER_IDENTITY_KEYS):
        extra = sorted(set(fields) - set(WRITER_IDENTITY_KEYS))
        missing = sorted(set(WRITER_IDENTITY_KEYS) - set(fields))
        if extra or missing:
            reasons.append(f"plan_profile_identity_keyset_drift(extra={extra},"
                           f"missing={missing})")
        elif list(raw.keys()) != list(WRITER_IDENTITY_KEYS):
            reasons.append("plan_profile_identity_key_order_drift")
    return reasons


def _check_profile_plan_links(pview: dict[str, object],
                              fields: dict[str, str]) -> list[str]:
    reasons: list[str] = []
    plan_links: tuple[tuple[str, str], ...] = (
        ("source", "identity_source"), ("profile", "profile"),
        ("schedule", "schedule_identity"),
        ("attempt_schema", "attempt_schema"),
        ("schema", "observer_state_schema"))
    for key, plan_key in plan_links:
        if plan_key not in pview:
            continue
        if fields.get(key) != str(pview[plan_key]):
            reasons.append(f"plan_profile_identity_inconsistent({key})")
    for key, want in (("endpoint", "4096"), ("resume", "unsupported"),
                      ("method", "cox-matthews")):
        if fields.get(key) != want:
            reasons.append(f"plan_profile_identity_inconsistent({key})")
    return reasons


def expected_identity_map(plan: Plan) -> tuple[dict[str, str], list[str]]:
    """The COMPLETE frozen profile-identity map: every field the production
    writer emits, each with its exact expected value, in writer order.

    The map comes from ``plan["profile_identity"]`` — frozen from the real
    staged binary's identity document (docs/identity-expected.json) — so the
    legitimate writer fields beyond the seven core ones (backend, provider,
    w3/timer/sampling provenance, caps, schemas, numa, external_stop, ...)
    validate by whole-field equality instead of being rejected as "extra".
    A field absent from the map is unknown drift; a duplicated or malformed
    part is malformed; the plan-linked fields must agree with the plan.
    """
    # A possibly-UNVALIDATED plan view: TypedDict claims cannot be trusted
    # before validate_plan_shape has run, so the profile map is re-probed.
    pview: dict[str, object] = dict(plan)
    probe = pview.get("profile_identity")
    if not isinstance(probe, dict) or not probe:
        return {}, ["plan_profile_identity_missing"]
    raw = cast("dict[object, object]", probe)
    fields, reasons = _parse_profile_map(raw)
    reasons += _check_profile_keyset(fields, raw)
    reasons += _check_profile_plan_links(pview, fields)
    return fields, reasons


def identity_text_from_map(fields: dict[str, str]) -> str:
    return ";".join(f"{key}={value}" for key, value in fields.items())


def validate_identity_fields(text: str, plan: Plan) -> list[str]:
    """Whole-field equality against the frozen profile-identity map; unknown
    drift is refused while the legitimate writer fields are validated."""
    fields, reasons = identity_fields(text)
    expected, plan_reasons = expected_identity_map(plan)
    reasons += plan_reasons
    for key, want in expected.items():
        if key not in fields:
            reasons.append(f"identity_field_missing({key})")
        elif fields[key] != want:
            reasons.append(f"identity_field_mismatch({key}={fields[key]!r},"
                           f"want={want!r})")
    for key in fields:
        if key not in expected:
            reasons.append(f"identity_field_unexpected({key})")
    return reasons


def identity_endpoint(text: str) -> int:
    fields, reasons = identity_fields(text)
    if reasons or "endpoint" not in fields:
        raise ValueError("identity_endpoint_unparseable")
    if not fields["endpoint"].isdigit():
        raise ValueError("identity_endpoint_not_integer")
    return int(fields["endpoint"])


def expected_snapshot_size(identity: str, coefficient_bytes: int) -> int:
    return FIXED_BYTES + len(identity.encode("utf-8")) + coefficient_bytes


def _nul_run(raw: bytes) -> int:
    run = longest = 0
    for byte in raw:
        run = run + 1 if byte == 0 else 0
        longest = max(longest, run)
    return longest


def validate_snapshot_bytes(raw: bytes, *, identity: str, clocks: tuple[int, int, int, int],
                            coefficient_bytes: int, state_sha256: str,
                            maximum_nul_run: int | None = None) -> list[str]:
    """Verify every structural field of one writer-produced snapshot blob."""
    reasons: list[str] = []
    identity_bytes = identity.encode("utf-8")
    if raw[:len(MAGIC)] != MAGIC:
        return ["snapshot_magic_mismatch"]
    if len(raw) < IDENTITY_OFFSET:
        return ["snapshot_truncated_before_identity_length"]
    declared = int.from_bytes(raw[len(MAGIC):IDENTITY_OFFSET], "little")
    if declared != len(identity_bytes):
        return [f"snapshot_identity_length_mismatch({declared}!={len(identity_bytes)})"]
    total = expected_snapshot_size(identity, coefficient_bytes)
    if len(raw) != total:
        return [f"snapshot_size_mismatch({len(raw)}!={total})"]
    start = IDENTITY_OFFSET
    if raw[start:start + len(identity_bytes)] != identity_bytes:
        reasons.append("snapshot_identity_mismatch")
    clock_start = start + len(identity_bytes)
    words = [int.from_bytes(raw[clock_start + index * CLOCK_BYTES:
                                clock_start + (index + 1) * CLOCK_BYTES], "little")
             for index in range(CLOCK_WORDS)]
    for name, observed, want in zip(("elapsed", "target", "epoch", "accepted_steps"),
                                    words, clocks):
        if observed != want:
            reasons.append(f"snapshot_clock_{name}_mismatch({observed}!={want})")
    if words[0] == words[1]:
        reasons.append("snapshot_clock_elapsed_equals_target")
    coeff_start = clock_start + CLOCK_BLOCK_BYTES
    coeff_end = coeff_start + coefficient_bytes
    coefficients = raw[coeff_start:coeff_end]
    if len(coefficients) != coefficient_bytes:
        return reasons + ["snapshot_coefficient_region_truncated"]
    digest = raw[coeff_end:]
    computed = hashlib.sha256(coefficients).hexdigest()
    if digest != bytes.fromhex(computed):
        reasons.append("snapshot_trailing_digest_mismatch")
    if computed != state_sha256:
        reasons.append(f"snapshot_payload_sha_mismatch(got={computed[:12]}…,"
                       f"want={state_sha256[:12]}…)")
    from_rest = words[0] == 0 and words[2] == 0 and words[3] == 0
    if coefficient_bytes > 0 and not any(coefficients) and not from_rest:
        reasons.append("snapshot_all_zero_coefficients")
    if maximum_nul_run is not None and _nul_run(coefficients) > maximum_nul_run:
        reasons.append("snapshot_sparse_nul_run")
    return reasons


def validate_snapshot_file(path: Path, *, identity: str,
                           clocks: tuple[int, int, int, int],
                           coefficient_bytes: int, state_sha256: str,
                           maximum_nul_run: int | None = None) -> list[str]:
    """File-level guards (exact size + sparse-hole detection) then bytes."""
    path = Path(path)
    try:
        stat = path.stat()
    except OSError as error:
        return [f"snapshot_unreadable({error})"]
    total = expected_snapshot_size(identity, coefficient_bytes)
    if stat.st_size != total:
        return [f"snapshot_size_mismatch({stat.st_size}!={total})"]
    if stat.st_blocks * 512 + SPARSE_SLACK_BYTES < stat.st_size:
        return [f"snapshot_sparse_hole(allocated={stat.st_blocks * 512},"
                f"size={stat.st_size})"]
    try:
        raw = path.read_bytes()
    except OSError as error:
        return [f"snapshot_unreadable({error})"]
    return validate_snapshot_bytes(raw, identity=identity, clocks=clocks,
                                   coefficient_bytes=coefficient_bytes,
                                   state_sha256=state_sha256,
                                   maximum_nul_run=maximum_nul_run)
