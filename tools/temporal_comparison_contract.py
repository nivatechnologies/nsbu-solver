"""Closed N512/M512 h64/h32/h16 temporal-family contract and family-plan schema.

Preparation-only: this module binds the frozen nested temporal family (three
independently evolved from-rest Cox--Matthews branches on the identical N512
retained/M512 force profile) and validates family plans. It never opens state,
accepts a numerical result or qualifies a window. Standard library only.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from pathlib import PurePosixPath
from typing import Final

from tools.json_types import Json, JsonObject, object_value
from tools.prepare_snapshot_manifest import coefficient_bytes_for
from tools.prepare_snapshot_manifest_contract import (
    CAPTURED_SCHEMA, COMPARISON_SCHEMA, ENDPOINT_CASE_SHA256, ENDPOINT_CLOCK,
    ENDPOINT_IDENTITY, ENDPOINT_N512_SOURCE, ENDPOINT_PLAN_SHA256, ENDPOINT_PROFILE,
    ENDPOINT_TARGET,
)
from tools.prepare_snapshot_manifest_validation import (
    MAX_IDENTITY_BYTES, PreparationError, count_value, exact_keys, grid_value, hex_value,
    identity_field, positive_value, relative_name, string_value, tolerance_list,
)

FAMILY_SCHEMA: Final = "p10-n512-temporal-family-plan-v1"
FAMILY_STATUS: Final = "prepared_only_no_capture_no_comparison_no_qualification"
BOOKKEEPING_SCHEMA: Final = "p10-n512-temporal-comparison-bookkeeping-v1"
PREPARATION_STATUS: Final = "pairwise_time_diagnostic_manifests_prepared_evidence_only"
INVENTORY_SCHEMA: Final = "p10-n512-temporal-file-inventory-v1"
ARITHMETIC_SCHEMA: Final = "p10-n512-temporal-arithmetic-binding-v1"
COMPARISON_KIND: Final = "TIME_DIAGNOSTIC"
QUANTUM_EXPONENT: Final = -20
SEGMENT_FIELDS = frozenset({"from_inclusive", "until_exclusive", "step_ticks"})
TRUNCATION_NOTE: Final = (
    "per-clock schedules are the branch schedule truncated at the shared clock; "
    "from-rest evolution up to that clock is unchanged and the trajectory "
    "endpoint stays bound separately in bookkeeping"
)

COARSE, MIDDLE, FINE = "h64", "h32", "h16"
BRANCH_ORDER: Final = (COARSE, MIDDLE, FINE)
BRANCH_STEPS: Final = {COARSE: (64, 128, 48), MIDDLE: (32, 64, 96), FINE: (16, 32, 192)}
SWITCH_CLOCK: Final = 2048
ENDPOINT_TICK: Final = ENDPOINT_CLOCK
CLOCK_TARGET: Final = ENDPOINT_TARGET
SHARED_CLOCKS: Final = (512, 1024, 1536, 2048, 2560, 3072, 3584, 4096)
STATE_SCHEMA: Final = CAPTURED_SCHEMA
COMPARISON_SCHEMA_NAME: Final = COMPARISON_SCHEMA
COEFFICIENT_BYTES: Final = 3233808384
assert COEFFICIENT_BYTES == coefficient_bytes_for([512, 512, 512])
SNAP_HEADER_TRAILER_BYTES: Final = 116
ADAPTER_FIXED_OVERHEAD_BYTES: Final = 1 << 20
PAIR_RESERVATION_BYTES: Final = 2 * COEFFICIENT_BYTES + ADAPTER_FIXED_OVERHEAD_BYTES
PAYLOAD_TOTAL_BYTES: Final = COEFFICIENT_BYTES * len(BRANCH_ORDER) * len(SHARED_CLOCKS)
H64_EXECUTION: Final = "offline-baccus-observer"
H64_PLAN_FILE: Final = "v3-launch-plan.json"
OBSERVER_EXECUTION: Final = "offline-baccus-required"
LENGTHS: Final = (1.0, 1.0, 1.0)
VISCOSITY: Final = 1.0
ABSOLUTE_TOLERANCES: Final = (1e-5, 1e-4)
RELATIVE_TOLERANCES: Final = (1e-5, 1e-5)
ADVECTIVE_LIMIT: Final = 3.3
BACKEND: Final = "rustfft-6.4.1-avx-avx2-fma"
MAX_ATTEMPTS = {name: steps[2] for name, steps in BRANCH_STEPS.items()}
PAIRS: Final = ((COARSE, MIDDLE), (MIDDLE, FINE))
PAIR_KEYS: Final = tuple(f"{coarse}--{fine}" for coarse, fine in PAIRS)
REVIEW_CONCLUSIONS = frozenset({"reviewed-equivalence-supported-by-controls"})
REVIEW_OUTCOMES = frozenset({"successful-exact-bit"})
LINEAGE_STATUSES = frozenset({"reviewed-unchanged-kernel-lineage"})

PENDING_BRANCH_IDENTITY: Final = "PENDING_BRANCH_IDENTITY_BINDING"
PENDING_BRANCH_PROFILE: Final = "PENDING_BRANCH_PROFILE_BINDING"
PENDING_BRANCH_SOURCE: Final = "PENDING_BRANCH_SOURCE_BINDING"
PENDING_BRANCH_EXECUTION: Final = "PENDING_BRANCH_EXECUTION_BINDING"
PENDING_BRANCH_PLAN_SHA256: Final = "PENDING_BRANCH_PLAN_SHA256_BINDING"
PENDING_BRANCH_PLAN_FILE: Final = "PENDING_BRANCH_PLAN_FILE_BINDING"
PENDING_ARITHMETIC_BINDING: Final = "PENDING_ARITHMETIC_BINDING"
PENDING_TOKENS = frozenset({
    PENDING_BRANCH_IDENTITY, PENDING_BRANCH_PROFILE, PENDING_BRANCH_SOURCE,
    PENDING_BRANCH_EXECUTION, PENDING_BRANCH_PLAN_SHA256, PENDING_BRANCH_PLAN_FILE,
    PENDING_ARITHMETIC_BINDING,
})

FAMILY_FIELDS = frozenset({
    "schema", "status", "comparison_kind", "state_schema", "method", "dimensions",
    "integration_force_dimensions", "case_sha256", "quantum_exponent", "clock_target",
    "trajectory_endpoint", "switch_clock", "shared_clocks", "lengths", "viscosity",
    "absolute_tolerances", "relative_tolerances", "advective_limit", "backend",
    "coefficient_bytes", "adapter_overhead_bytes", "pair_reservation_bytes",
    "coefficient_bytes_total", "branches", "pairs", "arithmetic_files",
})
BRANCH_FIELDS = frozenset({
    "name", "coarse_step_ticks", "late_step_ticks", "maximum_attempts", "schedule",
    "schedule_name", "profile_prefix", "identity", "profile", "source_commit",
    "execution", "plan_sha256", "plan_file", "observer_execution",
})


def schedule_name(coarse: str) -> str:
    _, late, _ = BRANCH_STEPS[coarse]
    return f"{coarse}-clocks0-through{SWITCH_CLOCK}-then-h{late}-through{ENDPOINT_TICK}"


def profile_prefix(coarse: str) -> str:
    early, late, _ = BRANCH_STEPS[coarse]
    return f"n512-m512-h{early}to{SWITCH_CLOCK}-h{late}to{ENDPOINT_TICK}-cadv33"


def full_schedule(coarse: str) -> list[JsonObject]:
    early, late, _ = BRANCH_STEPS[coarse]
    return [
        {"from_inclusive": 0, "until_exclusive": SWITCH_CLOCK, "step_ticks": early},
        {"from_inclusive": SWITCH_CLOCK, "until_exclusive": ENDPOINT_TICK, "step_ticks": late},
    ]


def schedule_through(coarse: str, clock: int) -> list[JsonObject]:
    if clock not in SHARED_CLOCKS:
        raise PreparationError(f"{clock} is not an exact shared comparison clock")
    early, late, _ = BRANCH_STEPS[coarse]
    if clock <= SWITCH_CLOCK:
        return [{"from_inclusive": 0, "until_exclusive": clock, "step_ticks": early}]
    return [
        {"from_inclusive": 0, "until_exclusive": SWITCH_CLOCK, "step_ticks": early},
        {"from_inclusive": SWITCH_CLOCK, "until_exclusive": clock, "step_ticks": late},
    ]


def schedule_matches(name: str, value: Json) -> bool:
    expected = full_schedule(name)
    if not isinstance(value, list) or len(value) != len(expected):
        return False
    for segment, want in zip(value, expected):
        if not isinstance(segment, dict) or set(segment) != set(SEGMENT_FIELDS):
            return False
        for field, tick in want.items():
            item = segment[field]
            if isinstance(item, bool) or not isinstance(item, int) or item != tick:
                return False
    return True


def attempts_at(coarse: str, clock: int) -> int:
    early, late, _ = BRANCH_STEPS[coarse]
    if clock <= SWITCH_CLOCK:
        return clock // early
    return SWITCH_CLOCK // early + (clock - SWITCH_CLOCK) // late


def bundle_name(coarse: str, clock: int) -> str:
    return f"step-{attempts_at(coarse, clock):03}-clock-{clock:04}"


def expected_state_file_bytes(identity: str) -> int:
    return SNAP_HEADER_TRAILER_BYTES + len(identity.encode("utf-8")) + COEFFICIENT_BYTES


def is_pending(value: str) -> bool:
    return value in PENDING_TOKENS


def _require_single(identity: str, key: str, expected: str, label: str) -> None:
    found = identity_field(identity, key)
    if len(found) != 1 or found[0] != expected:
        raise PreparationError(f"{label} identity must name {key}={expected} exactly once")


def validate_branch_identity(name: str, values: Mapping[str, Json]) -> None:
    identity = string_value(values["identity"], f"branch {name} identity")
    if not 0 < len(identity.encode("utf-8")) <= MAX_IDENTITY_BYTES:
        raise PreparationError(f"branch {name} identity is empty or exceeds 16 KiB")
    profile = string_value(values["profile"], f"branch {name} profile")
    prefix = string_value(values["profile_prefix"], f"branch {name} profile prefix")
    if not profile.startswith(prefix) or profile == prefix:
        raise PreparationError(f"branch {name} profile must extend its reviewed prefix exactly")
    _require_single(identity, "schema", STATE_SCHEMA, f"branch {name}")
    _require_single(identity, "case", string_value(values["case"], f"branch {name} case"),
                    f"branch {name}")
    _require_single(identity, "profile", profile, f"branch {name}")
    _require_single(identity, "source", hex_value(values["source_commit"], 40,
                                                  f"branch {name} source"), f"branch {name}")
    _require_single(identity, "backend", BACKEND, f"branch {name}")
    _require_single(identity, "method", "cox-matthews", f"branch {name}")
    _require_single(identity, "retained", "512", f"branch {name}")
    _require_single(identity, "force_samples", "512", f"branch {name}")
    _require_single(identity, "endpoint", str(ENDPOINT_TICK), f"branch {name}")
    _require_single(identity, "advective_limit", str(ADVECTIVE_LIMIT), f"branch {name}")
    _require_single(identity, "schedule", string_value(values["schedule_name"],
                                                       f"branch {name} schedule"),
                    f"branch {name}")
    _require_single(identity, "resume", "unsupported", f"branch {name}")
    _require_single(identity, "observer_execution",
                    string_value(values["observer_execution"], f"branch {name} observer"),
                    f"branch {name}")
    for key in ("host", "external_stop", "production_source", "test_source"):
        found = identity_field(identity, key)
        if len(found) != 1 or not found[0]:
            raise PreparationError(f"branch {name} identity must name {key} exactly once")
    for key in ("production_source", "test_source"):
        hex_value(identity_field(identity, key)[0], 40, f"branch {name} identity {key}")


def _validate_branch(name: str, values: Mapping[str, Json], strict: bool) -> None:
    exact_keys(values, BRANCH_FIELDS | frozenset({"case"}), frozenset(), f"branch {name}")
    if string_value(values["name"], "branch name") != name:
        raise PreparationError(f"branch entries must appear in family order at {name}")
    early, late, attempts = BRANCH_STEPS[name]
    for key, expected in (("coarse_step_ticks", early), ("late_step_ticks", late),
                         ("maximum_attempts", attempts)):
        if count_value(values[key], f"branch {name} {key}") != expected:
            raise PreparationError(f"branch {name} {key} conflicts with the closed family")
    if not schedule_matches(name, values["schedule"]):
        raise PreparationError(f"branch {name} schedule conflicts with the closed family")
    if string_value(values["schedule_name"], f"branch {name} schedule_name") != schedule_name(name):
        raise PreparationError(f"branch {name} schedule name conflicts with the closed family")
    if string_value(values["profile_prefix"], f"branch {name} profile_prefix") \
            != profile_prefix(name):
        raise PreparationError(f"branch {name} profile prefix conflicts with the closed family")
    if string_value(values["observer_execution"], f"branch {name} observer_execution") \
            != OBSERVER_EXECUTION:
        raise PreparationError(f"branch {name} observer execution conflicts with the closed family")
    case = string_value(values["case"], f"branch {name} case")
    if case != ENDPOINT_CASE_SHA256:
        raise PreparationError(f"branch {name} case conflicts with the frozen case identity")
    plan_file = string_value(values["plan_file"], f"branch {name} plan_file")
    if not is_pending(plan_file):
        relative_name(plan_file, f"branch {name} plan_file")
    pending = any(is_pending(string_value(values[key], f"branch {name} {key}"))
                  for key in ("identity", "profile", "source_commit", "execution",
                              "plan_sha256", "plan_file"))
    if strict and pending:
        raise PreparationError(f"branch {name} still carries pending capture bindings")
    if name == COARSE:
        for key, expected in (("identity", ENDPOINT_IDENTITY), ("profile", ENDPOINT_PROFILE),
                              ("source_commit", ENDPOINT_N512_SOURCE),
                              ("plan_sha256", ENDPOINT_PLAN_SHA256),
                              ("execution", H64_EXECUTION), ("plan_file", H64_PLAN_FILE)):
            if string_value(values[key], f"branch {name} {key}") != expected:
                raise PreparationError(f"branch {name} {key} conflicts with the reviewed v3 anchor")
    if not pending:
        hex_value(values["plan_sha256"], 64, f"branch {name} plan_sha256")
        hex_value(values["source_commit"], 40, f"branch {name} source_commit")
        if not string_value(values["execution"], f"branch {name} execution"):
            raise PreparationError(f"branch {name} execution must be non-empty")
        validate_branch_identity(name, values)


def _validate_family_bindings(values: Mapping[str, Json]) -> None:
    if string_value(values["schema"], "family schema") != FAMILY_SCHEMA:
        raise PreparationError(f"family plan schema is not {FAMILY_SCHEMA}")
    if string_value(values["status"], "family status") != FAMILY_STATUS:
        raise PreparationError("family plan must carry the prepared-only no-qualification status")
    if string_value(values["comparison_kind"], "family comparison_kind") != COMPARISON_KIND:
        raise PreparationError(f"family comparison_kind is not {COMPARISON_KIND}")
    if string_value(values["state_schema"], "family state_schema") != STATE_SCHEMA:
        raise PreparationError(f"family state_schema is not {STATE_SCHEMA}")
    if string_value(values["method"], "family method") != "cox-matthews":
        raise PreparationError("family method must be cox-matthews")
    if grid_value(values["dimensions"], "family dimensions") != [512, 512, 512] or \
            grid_value(values["integration_force_dimensions"], "family force dimensions") \
            != [512, 512, 512]:
        raise PreparationError("family dimensions must retain 512 with M512 force")
    if hex_value(values["case_sha256"], 64, "family case_sha256") != ENDPOINT_CASE_SHA256:
        raise PreparationError("family case conflicts with the frozen similarity-mms-v2 case")


def _validate_family_clocks(values: Mapping[str, Json]) -> None:
    exponent = values["quantum_exponent"]
    if isinstance(exponent, bool) or not isinstance(exponent, int) \
            or exponent != QUANTUM_EXPONENT:
        raise PreparationError("family quantum exponent is not the frozen -20")
    if count_value(values["clock_target"], "family clock_target") != CLOCK_TARGET:
        raise PreparationError("family clock target conflicts with the frozen target 8192")
    if count_value(values["trajectory_endpoint"], "family trajectory_endpoint") != ENDPOINT_TICK:
        raise PreparationError("family trajectory endpoint conflicts with the frozen 4096")
    if count_value(values["switch_clock"], "family switch_clock") != SWITCH_CLOCK:
        raise PreparationError("family switch clock is not the nested-family clock 2048")
    clocks = values["shared_clocks"]
    if not isinstance(clocks, list) or [count_value(c, "shared clock") for c in clocks] \
            != list(SHARED_CLOCKS):
        raise PreparationError("family clocks must be exactly 512 through 4096 by 512")


def _validate_family_physics(values: Mapping[str, Json]) -> None:
    lengths = values["lengths"]
    if not isinstance(lengths, list) or len(lengths) != 3 or \
            [positive_value(item, "family length") for item in lengths] != list(LENGTHS):
        raise PreparationError("family lengths must be the frozen unit cube")
    if positive_value(values["viscosity"], "family viscosity") != VISCOSITY:
        raise PreparationError("family viscosity conflicts with the frozen case setting")
    for key, expected in (("absolute_tolerances", ABSOLUTE_TOLERANCES),
                         ("relative_tolerances", RELATIVE_TOLERANCES)):
        if tolerance_list(values[key], f"family {key}") != [float(v) for v in expected]:
            raise PreparationError(f"family {key} conflicts with the frozen common policy")
    if positive_value(values["advective_limit"], "family advective_limit") != ADVECTIVE_LIMIT:
        raise PreparationError("family advective limit conflicts with the frozen guard 3.3")
    if string_value(values["backend"], "family backend") != BACKEND:
        raise PreparationError("family backend conflicts with the reviewed AVX backend")


def _validate_family_disk(values: Mapping[str, Json]) -> None:
    for key, expected in (("coefficient_bytes", COEFFICIENT_BYTES),
                         ("adapter_overhead_bytes", ADAPTER_FIXED_OVERHEAD_BYTES),
                         ("pair_reservation_bytes", PAIR_RESERVATION_BYTES),
                         ("coefficient_bytes_total", PAYLOAD_TOTAL_BYTES)):
        if count_value(values[key], f"family {key}") != expected:
            raise PreparationError(f"family {key} conflicts with the bound disk arithmetic")


def _validate_family_branches(values: Mapping[str, Json], strict: bool) -> None:
    branches = values["branches"]
    if not isinstance(branches, list) or len(branches) != len(BRANCH_ORDER):
        raise PreparationError("family must hold exactly the three nested branches")
    for name, entry in zip(BRANCH_ORDER, branches):
        _validate_branch(name, object_value(entry), strict)
    profiles = [string_value(object_value(entry)["profile"], "branch profile")
                for entry in branches if not is_pending(
                    string_value(object_value(entry)["profile"], "branch profile"))]
    if len(profiles) != len(set(profiles)):
        raise PreparationError("temporal branches must carry distinct unrelabelled profiles")


def _validate_family_pairs(values: Mapping[str, Json]) -> None:
    pairs = values["pairs"]
    if not isinstance(pairs, list) or [
            (string_value(object_value(p)["coarse"], "pair coarse"),
             string_value(object_value(p)["fine"], "pair fine")) for p in pairs] != list(PAIRS):
        raise PreparationError("family pairs must be exactly the nested coarse/fine order")
    for coarse, fine in PAIRS:
        for index in (0, 1):
            if BRANCH_STEPS[coarse][index] % BRANCH_STEPS[fine][index] != 0:
                raise PreparationError("temporal branches must be node-nested at every slab")


def _validate_family_arithmetic_files(values: Mapping[str, Json], strict: bool) -> None:
    bindings = {key: string_value(value, f"arithmetic binding {key}")
                for key, value in object_value(values["arithmetic_files"]).items()}
    if set(bindings) != set(PAIR_KEYS):
        raise PreparationError("family must bind an arithmetic review per nested pair")
    if not strict:
        return
    for key, value in bindings.items():
        if is_pending(value):
            raise PreparationError(f"pair {key} still carries a pending arithmetic binding")
        candidate = PurePosixPath(value)
        if not value or candidate.is_absolute() or ".." in candidate.parts or value == ".":
            raise PreparationError(f"arithmetic binding {key} must be a safe relative path")


def validate_family_plan(values: Mapping[str, Json], *, strict: bool) -> Mapping[str, Json]:
    exact_keys(values, FAMILY_FIELDS, frozenset(), "temporal family plan")
    _validate_family_bindings(values)
    _validate_family_clocks(values)
    _validate_family_physics(values)
    _validate_family_disk(values)
    _validate_family_branches(values, strict)
    _validate_family_pairs(values)
    _validate_family_arithmetic_files(values, strict)
    return values


def _pending_branch(name: str) -> JsonObject:
    return {
        "name": name, "coarse_step_ticks": BRANCH_STEPS[name][0],
        "late_step_ticks": BRANCH_STEPS[name][1], "maximum_attempts": BRANCH_STEPS[name][2],
        "schedule": full_schedule(name), "schedule_name": schedule_name(name),
        "profile_prefix": profile_prefix(name), "case": ENDPOINT_CASE_SHA256,
        "identity": PENDING_BRANCH_IDENTITY, "profile": PENDING_BRANCH_PROFILE,
        "source_commit": PENDING_BRANCH_SOURCE, "execution": PENDING_BRANCH_EXECUTION,
        "plan_sha256": PENDING_BRANCH_PLAN_SHA256, "plan_file": PENDING_BRANCH_PLAN_FILE,
        "observer_execution": OBSERVER_EXECUTION,
    }


def emitted_family_plan() -> JsonObject:
    """The reviewed h64 anchor plus explicit pending bindings for the finer branches."""
    h64 = _pending_branch(COARSE)
    h64.update({"identity": ENDPOINT_IDENTITY, "profile": ENDPOINT_PROFILE,
                "source_commit": ENDPOINT_N512_SOURCE, "execution": H64_EXECUTION,
                "plan_sha256": ENDPOINT_PLAN_SHA256, "plan_file": H64_PLAN_FILE})
    return {
        "schema": FAMILY_SCHEMA, "status": FAMILY_STATUS, "comparison_kind": COMPARISON_KIND,
        "state_schema": STATE_SCHEMA, "method": "cox-matthews",
        "dimensions": [512, 512, 512], "integration_force_dimensions": [512, 512, 512],
        "case_sha256": ENDPOINT_CASE_SHA256, "quantum_exponent": QUANTUM_EXPONENT,
        "clock_target": CLOCK_TARGET, "trajectory_endpoint": ENDPOINT_TICK,
        "switch_clock": SWITCH_CLOCK, "shared_clocks": list(SHARED_CLOCKS),
        "lengths": list(LENGTHS), "viscosity": VISCOSITY,
        "absolute_tolerances": list(ABSOLUTE_TOLERANCES),
        "relative_tolerances": list(RELATIVE_TOLERANCES),
        "advective_limit": ADVECTIVE_LIMIT, "backend": BACKEND,
        "coefficient_bytes": COEFFICIENT_BYTES,
        "adapter_overhead_bytes": ADAPTER_FIXED_OVERHEAD_BYTES,
        "pair_reservation_bytes": PAIR_RESERVATION_BYTES,
        "coefficient_bytes_total": PAYLOAD_TOTAL_BYTES,
        "branches": [h64, _pending_branch(MIDDLE), _pending_branch(FINE)],
        "pairs": [{"coarse": coarse, "fine": fine} for coarse, fine in PAIRS],
        "arithmetic_files": {key: PENDING_ARITHMETIC_BINDING for key in PAIR_KEYS},
    }


def arithmetic_review_contract(values: Mapping[str, Json], coarse: str, fine: str,
                               plan: Mapping[str, Json]) -> Mapping[str, Json]:
    """Admit one per-pair arithmetic binding and return its carried arithmetic_control."""
    exact_keys(values, frozenset({"schema", "pair", "arithmetic_control"}), frozenset(),
               f"{coarse}--{fine} arithmetic binding")
    if string_value(values["schema"], "arithmetic binding schema") != ARITHMETIC_SCHEMA:
        raise PreparationError(f"arithmetic binding schema is not {ARITHMETIC_SCHEMA}")
    if string_value(values["pair"], "arithmetic binding pair") != f"{coarse}--{fine}":
        raise PreparationError("arithmetic binding is not for the declared nested pair")
    control = object_value(values["arithmetic_control"])
    exact_keys(control, frozenset({"evidence", "evidence_sha256", "review"}), frozenset(),
               "arithmetic control")
    relative_name(string_value(control["evidence"], "arithmetic evidence"),
                  "arithmetic evidence")
    hex_value(control["evidence_sha256"], 64, "arithmetic evidence_sha256")
    review = object_value(control["review"])
    exact_keys(review, frozenset({
        "schema", "conclusion", "case_sha256", "method", "integration_force_dimensions",
        "measured_control", "reviewed_lineage",
    }), frozenset(), "arithmetic review")
    if string_value(review["schema"], "arithmetic review schema") != "p10-time-arithmetic-review-v1":
        raise PreparationError("arithmetic review schema is not p10-time-arithmetic-review-v1")
    if string_value(review["conclusion"], "arithmetic conclusion") not in REVIEW_CONCLUSIONS:
        raise PreparationError("arithmetic conclusion is not an admitted reviewed equivalence")
    if hex_value(review["case_sha256"], 64, "arithmetic review case") != ENDPOINT_CASE_SHA256:
        raise PreparationError("arithmetic review case conflicts with the frozen case")
    if string_value(review["method"], "arithmetic review method") != "cox-matthews":
        raise PreparationError("arithmetic review method conflicts with the frozen method")
    if grid_value(review["integration_force_dimensions"], "arithmetic review force grid") \
            != [512, 512, 512]:
        raise PreparationError("arithmetic review force grid conflicts with M512")
    measured = object_value(review["measured_control"])
    exact_keys(measured, frozenset({"outcome", "serial", "w3"}), frozenset(),
               "arithmetic measured control")
    if string_value(measured["outcome"], "arithmetic measured outcome") not in REVIEW_OUTCOMES:
        raise PreparationError("arithmetic measured control is not a successful exact-bit result")
    for side in ("serial", "w3"):
        exact_keys(object_value(measured[side]), frozenset(
            {"source_commit", "backend", "execution", "configuration"}), frozenset(),
            f"arithmetic measured {side}")
        hex_value(object_value(measured[side])["source_commit"], 40,
                  f"arithmetic measured {side} source")
    lineage = object_value(review["reviewed_lineage"])
    exact_keys(lineage, frozenset({"status", "left_control_role", "right_control_role",
                                   "left", "right"}), frozenset(), "arithmetic lineage")
    if string_value(lineage["status"], "arithmetic lineage status") not in LINEAGE_STATUSES:
        raise PreparationError("arithmetic lineage is not a reviewed unchanged kernel lineage")
    if string_value(lineage["left_control_role"], "left role") != "serial" or \
            string_value(lineage["right_control_role"], "right role") != "w3":
        raise PreparationError("arithmetic lineage roles are not the reviewed serial/w3 roles")
    branches = branch_entries(plan)
    for side, name in (("left", coarse), ("right", fine)):
        entry: Mapping[str, Json] = branches[name]
        bound = object_value(lineage[side])
        exact_keys(bound, frozenset({"source_commit", "backend", "execution", "profile"}),
                   frozenset(), f"arithmetic lineage {side}")
        profile = object_value(bound["profile"])
        exact_keys(profile, frozenset({"kind", "value"}), frozenset(),
                   f"arithmetic lineage {side} profile")
        for key, expected in (("source_commit", entry["source_commit"]), ("backend", BACKEND),
                              ("execution", entry["execution"])):
            if string_value(bound[key], f"arithmetic lineage {side} {key}") \
                    != string_value(expected, f"branch {name} {key}"):
                raise PreparationError(
                    f"arithmetic lineage {side} conflicts with the {name} branch binding")
        if string_value(profile["kind"], "arithmetic lineage profile kind") \
                != "identity-profile-field" or string_value(profile["value"], "value") \
                != string_value(entry["profile"], f"branch {name} profile"):
            raise PreparationError(
                f"arithmetic lineage {side} profile does not name the {name} profile exactly")
    return control


def _family_branches(plan: Mapping[str, Json]) -> Sequence[Json]:
    branches = plan["branches"]
    if not isinstance(branches, list):
        raise PreparationError("family branches must be a list")
    return branches


def branch_entries(plan: Mapping[str, Json]) -> dict[str, Mapping[str, Json]]:
    return {string_value(object_value(entry)["name"], "branch name"): object_value(entry)
            for entry in _family_branches(plan)}
