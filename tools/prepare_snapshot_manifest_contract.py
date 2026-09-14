"""Closed scientific, record and plan contracts for N512 endpoint preparation.

Validates the reviewed comparison template, the captured observer record, the
reviewed trajectory capture plan and the v3 launch plan that binds it, and
emits the endpoint comparison-input template from the reviewed contract only.
Unknown endpoint artifact hashes stay explicit placeholders.
"""

from __future__ import annotations

from typing import Final

from collections.abc import Mapping

from tools.json_types import Json, JsonObject, object_value
from tools.prepare_snapshot_manifest_validation import (
    MAX_IDENTITY_BYTES, PreparationError, count_value, exact_keys, flag_value, grid_value,
    hex_value, identity_field, positive_value, relative_name, string_value, tolerance_list,
)

COMPARISON_SCHEMA: Final = "p10-snapshot-comparison-input-v1"
CAPTURED_SCHEMA: Final = "p10-avx-n512-observer-state-v1"
CAPTURE_PLAN_SCHEMA: Final = "p10-n512-m512-endpoint-capture-plan-v1"
LAUNCH_PLAN_SCHEMA: Final = "p10-n512-m512-v3-launch-plan-v1"
OBSERVATION_STATUS: Final = "CapturedActualState"
PENDING_SNAPSHOT: Final = "PENDING_SNAPSHOT_BINDING"
PENDING_PLAN: Final = "PENDING_PLAN_BINDING"
PENDING_COEFFICIENT: Final = "PENDING_COEFFICIENT_SHA256"
PENDING_FILE: Final = "PENDING_FILE_SHA256"
PLACEHOLDER_HASHES = {"coefficient_sha256": PENDING_COEFFICIENT, "file_sha256": PENDING_FILE}
PROFILE_KINDS = frozenset({"legacy-full-identity", "identity-profile-field"})
REQUIRED_FIELDS = (
    "schema", "snapshot", "plan", "identity", "source_commit", "plan_sha256",
    "coefficient_sha256", "file_sha256", "backend", "execution", "dimensions",
    "evolution", "elapsed", "target", "epoch", "accepted_steps",
)
OPTIONAL_FIELDS = ("comparison_kind", "profile", "admission_guard", "arithmetic_control")
COMPARISON_KINDS = frozenset({
    "MATCHED_SPATIAL", "MATCHED_M512_SPATIAL_DIAGNOSTIC", "TIME_DIAGNOSTIC",
    "FORCE_RESOLUTION_DIAGNOSTIC", "METHOD_DIAGNOSTIC", "MIXED_FORCE_SPACE_DIAGNOSTIC",
})
EVOLUTION_FIELDS = frozenset({
    "case_sha256", "quantum_exponent", "clock_target", "comparison_endpoint",
    "lengths", "viscosity", "method", "integration_force_dimensions", "schedule",
    "absolute_tolerances", "relative_tolerances",
})
RECORD_FIELDS = frozenset({
    "schema", "identity", "resumable", "clock", "epoch", "accepted_steps",
    "coefficient_bytes", "state_sha256", "observation_status",
    "offline_observer_node", "observer_execution", "qualification",
})

# Reviewed v3 launch-plan contract (evidence/p10/n512-m512-endpoint-prep-20260913).
LAUNCH_STATUS: Final = "prepared_unexecuted_future_authorization_and_absolute_deadline_required"
LAUNCH_FEATURE: Final = "n512-m512-piecewise-cadv33"
LAUNCH_EXTERNAL_STOP: Final = "pgid-watchdog-v3-confirmed-identity-absolute-deadline"
LAUNCH_ARCHIVE_SCOPE: Final = ("local_create_new_partial_copy_inventory_compare_sync_rename; "
                              "no remote durable transfer is claimed")
LAUNCH_FIRST_STEP: Final = "clock64;rhs12;hits7;misses5;steady_allocations0;integration<=1200"
LAUNCH_DEADLINE_SOURCE: Final = "caller_supplied_future_absolute_epoch"
LAUNCH_OBSERVER_NODES: Final = 8
LAUNCH_FIELDS = frozenset({
    "schema", "status", "numerical_test_source_commit", "production_source_commit",
    "harness_commit_and_run_source", "feature", "profile", "external_stop", "binary_sha256",
    "preflight_sha256", "watchdog_sha256", "local_archive_helper_sha256",
    "historical_preparation", "deadline", "resources", "guards", "archive_scope",
    "endpoint_attempts_executed", "launch_authorized",
})
LAUNCH_HISTORY_FIELDS = frozenset({
    "immutable_trajectory_plan_sha256", "v2_binary_sha256", "preparation_results",
})
LAUNCH_DEADLINE_FIELDS = frozenset({
    "source", "minimum_remaining_at_launch_seconds", "minimum_remaining_after_first_step_seconds",
})
LAUNCH_RESOURCE_FIELDS = frozenset({
    "memory_floor_bytes", "source_disk_floor_bytes", "local_archive_disk_floor_bytes",
    "address_space_limit_bytes",
})
LAUNCH_GUARD_FIELDS = frozenset({
    "explicit_opt_in", "stable_process_identity_before_group_ownership",
    "term_grace_then_kill_seconds", "solver_and_local_archive_share_absolute_deadline",
    "first_step", "committed_states", "offline_observer_nodes", "qualification",
})

# Reviewed endpoint contract constants: trajectory plan (frozen-plan.json),
# v3 launch plan and the adapter closed contract in harness/src/m512_spatial.rs.
ENDPOINT_IDENTITY: Final = (
    "source=e25f3816f83c6a7c07202cac2878f58ace460511;"
    "case=e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e;"
    "profile=n512-m512-h64to2048-h128to4096-cadv33-w3-9eba11f;"
    "backend=rustfft-6.4.1-avx-avx2-fma;"
    "production_source=0843b8b18e6a096a0208e3d896e391c7b1b2f5e0;"
    "test_source=9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645;"
    "provider=parallel-reduced-v2-force-w3-attempt-cache;"
    "rhs_w3=layout768-width3-bidirectional-add21787856768;"
    "force_w3=layout512-width3-forward-add4318465792;rhs_timer=harness-timed-rhs-v1;"
    "clock=std-time-Instant;scope=evaluate-inclusive;overhead=included;retained=512;"
    "force_samples=512;observer_force_samples=1024;observer_conservative=1024;"
    "observer_execution=offline-baccus-required;sampling_workers=32;rhs_w3_workers=3;"
    "provider_w3_workers=3;method=cox-matthews;"
    "schedule=h64-clocks0-through2048-then-h128-through4096;endpoint=4096;"
    "advective_limit=3.3;execution_cap=207578085872;artifact_cap=549755813888;"
    "schema=p10-avx-n512-observer-state-v1;attempt_schema=p10-avx-scheduled-attempt-v3;"
    "resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;"
    "external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline"
)
ENDPOINT_N512_SOURCE: Final = "e25f3816f83c6a7c07202cac2878f58ace460511"
ENDPOINT_TEST_SOURCE: Final = "9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645"
ENDPOINT_PRODUCTION_SOURCE: Final = "0843b8b18e6a096a0208e3d896e391c7b1b2f5e0"
ENDPOINT_CASE_SHA256: Final = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"
ENDPOINT_PROFILE: Final = "n512-m512-h64to2048-h128to4096-cadv33-w3-9eba11f"
ENDPOINT_PLAN_SHA256: Final = "4c14ee169cfbbb2a182d972633aba1292ed041878a9c4322e64a527f87d85da8"
ENDPOINT_TRAJECTORY_PLAN_SHA256: Final = "6e8103a1937e3e31be8b147a936b843d4dc166877ef5de5540fb51429e672634"
ENDPOINT_TARGET: Final = 8192
ENDPOINT_CLOCK: Final = 4096
ENDPOINT_EPOCH: Final = 48
ENDPOINT_STEPS: Final = 48
ENDPOINT_ADVECTIVE_LIMIT: Final = 3.3
ENDPOINT_MAX_ATTEMPTS: Final = 48


def validate_evolution(evolution: Mapping[str, Json]) -> None:
    exact_keys(evolution, EVOLUTION_FIELDS, frozenset(), "template evolution")
    hex_value(evolution["case_sha256"], 64, "evolution case_sha256")
    endpoint = count_value(evolution["comparison_endpoint"], "evolution comparison_endpoint")
    lengths = evolution["lengths"]
    if not isinstance(lengths, list) or len(lengths) != 3:
        raise PreparationError("evolution lengths must hold three positive values")
    [positive_value(item, "evolution length") for item in lengths]
    positive_value(evolution["viscosity"], "evolution viscosity")
    if not string_value(evolution["method"], "evolution method"):
        raise PreparationError("evolution method must be non-empty")
    exponent = evolution["quantum_exponent"]
    if isinstance(exponent, bool) or not isinstance(exponent, int):
        raise PreparationError("evolution quantum_exponent must be an integer")
    grid_value(evolution["integration_force_dimensions"], "evolution integration_force_dimensions")
    schedule = evolution["schedule"]
    if not isinstance(schedule, list) or not schedule:
        raise PreparationError("evolution schedule must be a non-empty list")
    next_clock = 0
    for segment_value in schedule:
        segment = object_value(segment_value)
        exact_keys(segment, frozenset({"from_inclusive", "until_exclusive", "step_ticks"}),
                   frozenset(), "schedule segment")
        from_inclusive = count_value(segment["from_inclusive"], "schedule from_inclusive")
        until_exclusive = count_value(segment["until_exclusive"], "schedule until_exclusive")
        step_ticks = count_value(segment["step_ticks"], "schedule step_ticks")
        if from_inclusive != next_clock or until_exclusive <= from_inclusive or step_ticks == 0:
            raise PreparationError("piecewise schedule is not contiguous and increasing")
        if (until_exclusive - from_inclusive) % step_ticks != 0:
            raise PreparationError("schedule segment is not an exact multiple of its step")
        next_clock = until_exclusive
    if next_clock != endpoint:
        raise PreparationError("piecewise schedule does not reach the comparison endpoint")
    for name in ("absolute_tolerances", "relative_tolerances"):
        tolerance_list(evolution[name], f"evolution {name}")


def _validate_profile_binding(values: Mapping[str, Json], identity: str) -> None:
    if "profile" not in values:
        return
    binding = object_value(values["profile"])
    exact_keys(binding, frozenset({"kind", "value"}), frozenset(), "template profile binding")
    kind = string_value(binding["kind"], "template profile kind")
    if kind not in PROFILE_KINDS:
        raise PreparationError("template profile binding kind is not admitted")
    profile = string_value(binding["value"], "template profile value")
    if not profile:
        raise PreparationError("template profile value must be non-empty")
    if kind == "identity-profile-field":
        if identity_field(identity, "profile") != [profile]:
            raise PreparationError("template profile must name the identity profile field exactly")
    elif profile != identity:
        raise PreparationError("legacy-full-identity profile must equal the whole identity")


def _validate_admission_guard(values: Mapping[str, Json], identity: str) -> None:
    if "admission_guard" not in values:
        return
    guard = object_value(values["admission_guard"])
    exact_keys(guard, frozenset({"advective_limit", "maximum_attempts"}), frozenset(),
               "template admission guard")
    positive_value(guard["advective_limit"], "guard advective limit")
    if count_value(guard["maximum_attempts"], "guard maximum_attempts") == 0:
        raise PreparationError("guard maximum_attempts must be positive")
    limits = identity_field(identity, "advective_limit")
    if limits and float(limits[0]) != positive_value(guard["advective_limit"],
                                                     "guard advective limit"):
        raise PreparationError("template guard advective limit conflicts with the identity field")


def validate_template(values: Mapping[str, Json]) -> Mapping[str, Json]:
    exact_keys(values, frozenset(REQUIRED_FIELDS), frozenset(OPTIONAL_FIELDS),
               "comparison template")
    if string_value(values["schema"], "template schema") != COMPARISON_SCHEMA:
        raise PreparationError(f"template schema is not {COMPARISON_SCHEMA}")
    if "comparison_kind" in values:
        if string_value(values["comparison_kind"], "template comparison_kind") \
                not in COMPARISON_KINDS:
            raise PreparationError("template comparison_kind is not admitted")
    for name in ("backend", "execution"):
        if not string_value(values[name], f"template {name}"):
            raise PreparationError(f"template {name} must be non-empty")
    identity = string_value(values["identity"], "template identity")
    if not 0 < len(identity.encode("utf-8")) <= MAX_IDENTITY_BYTES:
        raise PreparationError("template identity is empty or exceeds 16 KiB")
    if identity_field(identity, "schema") != [CAPTURED_SCHEMA]:
        raise PreparationError(f"template identity must name schema={CAPTURED_SCHEMA} exactly once")
    source = hex_value(values["source_commit"], 40, "template source_commit")
    if identity_field(identity, "source") != [source]:
        raise PreparationError("template identity source must match source_commit exactly once")
    hex_value(values["plan_sha256"], 64, "template plan_sha256")
    _validate_profile_binding(values, identity)
    _validate_admission_guard(values, identity)
    for name, pending in PLACEHOLDER_HASHES.items():
        binding = string_value(values[name], f"template {name}")
        if binding != pending:
            hex_value(binding, 64, f"template {name}")
    for name, pending in (("snapshot", PENDING_SNAPSHOT), ("plan", PENDING_PLAN)):
        binding = string_value(values[name], f"template {name}")
        if binding != pending:
            relative_name(binding, f"template {name}")
    grid_value(values["dimensions"], "template dimensions")
    validate_evolution(object_value(values["evolution"]))
    for name in ("elapsed", "target", "epoch", "accepted_steps"):
        count_value(values[name], f"template {name}")
    evolution = object_value(values["evolution"])
    if count_value(values["elapsed"], "template elapsed") != count_value(
            evolution["comparison_endpoint"], "evolution comparison_endpoint"):
        raise PreparationError("template elapsed must equal the frozen comparison endpoint")
    if count_value(values["target"], "template target") != count_value(
            evolution["clock_target"], "evolution clock_target"):
        raise PreparationError("template target must equal the frozen clock target")
    return values


def verify_record_document(record: Mapping[str, Json], template: Mapping[str, Json],
                           coefficient_bytes: int,
                           coefficient_hash: str, observer_execution: str) -> None:
    exact_keys(record, RECORD_FIELDS, frozenset(), "captured record")
    if string_value(record["schema"], "record schema") != CAPTURED_SCHEMA:
        raise PreparationError(f"record schema is not {CAPTURED_SCHEMA}")
    if string_value(record["identity"], "record identity") != string_value(
            template["identity"], "identity"):
        raise PreparationError("record identity does not match the reviewed template")
    if flag_value(record["resumable"], "record resumable"):
        raise PreparationError("captured record must be resumable=false")
    if flag_value(record["qualification"], "record qualification"):
        raise PreparationError("captured record must be qualification=false")
    if string_value(record["observation_status"], "record observation_status") != OBSERVATION_STATUS:
        raise PreparationError(f"record observation_status is not {OBSERVATION_STATUS}")
    if not flag_value(record["offline_observer_node"], "record offline_observer_node"):
        raise PreparationError("captured record must mark offline_observer_node=true")
    if string_value(record["observer_execution"], "record observer_execution") != observer_execution:
        raise PreparationError(
            f"record observer_execution must be the reviewed {observer_execution} metadata")
    for record_key, template_key, message in (
        ("clock", "elapsed", "record clock is not the frozen comparison endpoint"),
        ("epoch", "epoch", "record epoch does not match the reviewed template"),
        ("accepted_steps", "accepted_steps", "record accepted_steps does not match the template"),
    ):
        if count_value(record[record_key], f"record {record_key}") != count_value(
                template[template_key], f"template {template_key}"):
            raise PreparationError(message)
    if count_value(record["coefficient_bytes"], "record coefficient_bytes") != coefficient_bytes:
        raise PreparationError("record coefficient_bytes does not match the streamed payload")
    if hex_value(record["state_sha256"], 64, "record state_sha256") != coefficient_hash:
        raise PreparationError("record state_sha256 does not match the streamed coefficient hash")


def _capture_numerics(values: Mapping[str, Json], template: Mapping[str, Json]) -> None:
    if string_value(values["schema"], "trajectory plan schema") != CAPTURE_PLAN_SCHEMA:
        raise PreparationError(f"trajectory plan schema is not {CAPTURE_PLAN_SCHEMA}")
    if flag_value(values["qualification"], "trajectory plan qualification"):
        raise PreparationError("trajectory plan must be qualification=false")
    case = hex_value(values["case_sha256"], 64, "trajectory plan case_sha256")
    evolution = object_value(template["evolution"])
    if case != hex_value(evolution["case_sha256"], 64, "evolution case_sha256"):
        raise PreparationError("trajectory plan case identity conflicts with the reviewed template")
    profile = object_value(values["profile"])
    if count_value(profile["clock_target"], "plan clock_target") != count_value(
            template["target"], "template target"):
        raise PreparationError("trajectory plan clock target conflicts with the reviewed template")
    if count_value(profile["endpoint"], "plan endpoint") != count_value(
            template["elapsed"], "template elapsed"):
        raise PreparationError("trajectory plan endpoint conflicts with the reviewed template")
    if string_value(profile["method"], "plan method") != string_value(
            evolution["method"], "evolution method"):
        raise PreparationError("trajectory plan method conflicts with the reviewed template")
    if flag_value(profile["analytical_reset"], "plan analytical_reset"):
        raise PreparationError("trajectory plan must record analytical_reset=false")
    for name in ("absolute_tolerances", "relative_tolerances"):
        if tolerance_list(profile[name], f"trajectory plan {name}") != [
                float(item) for item in tolerance_list(evolution[name], f"evolution {name}")]:
            raise PreparationError(f"trajectory plan {name} conflicts with the reviewed template")
    guard = template.get("admission_guard")
    if guard is not None:
        _capture_guard_binding(object_value(guard), profile)


def _capture_guard_binding(guard_values: Mapping[str, Json], profile: Mapping[str, Json]) -> None:
    if count_value(guard_values["maximum_attempts"], "guard maximum_attempts") != count_value(
            profile["maximum_attempts"], "plan maximum_attempts"):
        raise PreparationError("trajectory plan attempt bound conflicts with the reviewed guard")
    if positive_value(guard_values["advective_limit"], "guard advective limit") != \
            positive_value(profile["advective_limit"], "plan advective limit"):
        raise PreparationError("trajectory plan advective limit conflicts with the reviewed guard")


def _capture_operations(values: Mapping[str, Json], template: Mapping[str, Json]) -> str:
    captures = object_value(values["captures"])
    if string_value(captures["resume"], "plan resume") != "unsupported":
        raise PreparationError("trajectory plan captures must record resume=unsupported")
    if not flag_value(captures["create_new"], "plan create_new"):
        raise PreparationError("trajectory plan captures must record create_new=true")
    clocks = captures["positive_offline_observer_clocks"]
    if not isinstance(clocks, list) or count_value(template["elapsed"], "template elapsed") not in clocks:
        raise PreparationError("record clock is not a planned offline observer clock")
    resources = object_value(values["resource_plan"])
    host = string_value(resources["offline_observer_host"], "plan offline observer host")
    if not host:
        raise PreparationError("trajectory plan offline observer host must be non-empty")
    return f"offline-{host}-required"


def verify_capture_plan(values: Mapping[str, Json], template: Mapping[str, Json]) -> str:
    _capture_numerics(values, template)
    return _capture_operations(values, template)


def _launch_identity(values: Mapping[str, Json], template: Mapping[str, Json]) -> None:
    if string_value(values["status"], "launch plan status") != LAUNCH_STATUS:
        raise PreparationError("launch plan status is not the reviewed prepared-unexecuted status")
    if flag_value(values["launch_authorized"], "launch plan launch_authorized"):
        raise PreparationError("reviewed preparation binds an unauthorized launch plan")
    if count_value(values["endpoint_attempts_executed"], "launch plan endpoint_attempts_executed") != 0:
        raise PreparationError("launch plan reports executed endpoint attempts; preparation bytes changed")
    if string_value(values["feature"], "launch plan feature") != LAUNCH_FEATURE:
        raise PreparationError("launch plan feature is not the reviewed N512 M512 feature")
    if string_value(values["archive_scope"], "launch plan archive_scope") != LAUNCH_ARCHIVE_SCOPE:
        raise PreparationError("launch plan archive scope is not the reviewed local-only scope")
    identity = string_value(template["identity"], "template identity")
    profile = string_value(values["profile"], "launch plan profile")
    if identity_field(identity, "profile") != [profile]:
        raise PreparationError("launch plan profile string does not name the identity profile exactly")
    if identity_field(identity, "external_stop") != [
            string_value(values["external_stop"], "launch plan external_stop")]:
        raise PreparationError("launch plan external stop conflicts with the identity field")
    harness = hex_value(values["harness_commit_and_run_source"], 40, "launch plan harness commit")
    if harness != hex_value(template["source_commit"], 40, "template source_commit"):
        raise PreparationError("launch plan harness commit conflicts with the template source")
    if hex_value(values["numerical_test_source_commit"], 40, "launch plan test source") != \
            (identity_field(identity, "test_source") or [""])[0]:
        raise PreparationError("launch plan numerical test source conflicts with the identity field")
    if hex_value(values["production_source_commit"], 40, "launch plan production source") != \
            (identity_field(identity, "production_source") or [""])[0]:
        raise PreparationError("launch plan production source conflicts with the identity field")


def _launch_history(values: Mapping[str, Json]) -> str:
    for name in ("binary_sha256", "preflight_sha256", "watchdog_sha256", "local_archive_helper_sha256"):
        hex_value(values[name], 64, f"launch plan {name}")
    history = object_value(values["historical_preparation"])
    exact_keys(history, LAUNCH_HISTORY_FIELDS, frozenset(), "launch plan historical preparation")
    trajectory_hash = hex_value(history["immutable_trajectory_plan_sha256"], 64,
                                "launch plan immutable trajectory plan")
    hex_value(history["v2_binary_sha256"], 64, "launch plan v2 binary")
    results = string_value(history["preparation_results"], "launch plan preparation results")
    if not results or results.startswith("/"):
        raise PreparationError("launch plan preparation results must be a relative path")
    return trajectory_hash


def _launch_deadline_and_resources(values: Mapping[str, Json]) -> None:
    deadline = object_value(values["deadline"])
    exact_keys(deadline, LAUNCH_DEADLINE_FIELDS, frozenset(), "launch plan deadline")
    if string_value(deadline["source"], "launch plan deadline source") != LAUNCH_DEADLINE_SOURCE:
        raise PreparationError("launch plan deadline source is not the reviewed caller-supplied source")
    for name in ("minimum_remaining_at_launch_seconds", "minimum_remaining_after_first_step_seconds"):
        if count_value(deadline[name], f"launch plan {name}") <= 0:
            raise PreparationError(f"launch plan {name} must be positive")
    resources = object_value(values["resources"])
    exact_keys(resources, LAUNCH_RESOURCE_FIELDS, frozenset(), "launch plan resources")
    for name in LAUNCH_RESOURCE_FIELDS:
        if count_value(resources[name], f"launch plan {name}") <= 0:
            raise PreparationError(f"launch plan {name} must be positive")


def _launch_guards(values: Mapping[str, Json], template: Mapping[str, Json]) -> None:
    guards = object_value(values["guards"])
    exact_keys(guards, LAUNCH_GUARD_FIELDS, frozenset(), "launch plan guards")
    for name in ("explicit_opt_in", "stable_process_identity_before_group_ownership",
                 "solver_and_local_archive_share_absolute_deadline"):
        if not flag_value(guards[name], f"launch plan guards.{name}"):
            raise PreparationError(f"launch plan guard {name} must be true")
    if flag_value(guards["qualification"], "launch plan guards.qualification"):
        raise PreparationError("launch plan must be qualification=false")
    if count_value(guards["term_grace_then_kill_seconds"], "launch plan term grace") != 60:
        raise PreparationError("launch plan term grace is not the reviewed 60 seconds")
    if string_value(guards["first_step"], "launch plan first step") != LAUNCH_FIRST_STEP:
        raise PreparationError("launch plan first-step guard is not the reviewed exact check list")
    committed = count_value(guards["committed_states"], "launch plan committed_states")
    if committed != count_value(template["accepted_steps"], "template accepted_steps"):
        raise PreparationError("launch plan committed states conflict with the reviewed template")
    if count_value(guards["offline_observer_nodes"], "launch plan observer nodes") != LAUNCH_OBSERVER_NODES:
        raise PreparationError(f"launch plan must bind {LAUNCH_OBSERVER_NODES} offline observer nodes")


def verify_launch_plan(values: Mapping[str, Json], template: Mapping[str, Json]) -> str:
    """Admit the reviewed v3 launch plan and return its immutable trajectory-plan hash."""
    exact_keys(values, LAUNCH_FIELDS, frozenset(), "v3 launch plan")
    if string_value(values["schema"], "launch plan schema") != LAUNCH_PLAN_SCHEMA:
        raise PreparationError(f"launch plan schema is not {LAUNCH_PLAN_SCHEMA}")
    _launch_identity(values, template)
    trajectory_hash = _launch_history(values)
    _launch_deadline_and_resources(values)
    _launch_guards(values, template)
    return trajectory_hash


def endpoint_comparison_template() -> JsonObject:
    """Endpoint comparison-input template from the reviewed contract only.

    target 8192, endpoint 4096, epoch and accepted steps 48, exact sources and
    profile; the bound plan is the reviewed v3 launch plan (adapter contract
    plan_sha256). Endpoint coefficient and whole-file hashes are unknown before
    a capture and stay explicit placeholders.
    """
    return {
        "schema": COMPARISON_SCHEMA,
        "comparison_kind": "MATCHED_M512_SPATIAL_DIAGNOSTIC",
        "snapshot": PENDING_SNAPSHOT,
        "plan": PENDING_PLAN,
        "identity": ENDPOINT_IDENTITY,
        "source_commit": ENDPOINT_N512_SOURCE,
        "plan_sha256": ENDPOINT_PLAN_SHA256,
        "coefficient_sha256": PENDING_COEFFICIENT,
        "file_sha256": PENDING_FILE,
        "backend": "rustfft-6.4.1-avx-avx2-fma",
        "execution": "offline-baccus-observer",
        "dimensions": [512, 512, 512],
        "evolution": {
            "case_sha256": ENDPOINT_CASE_SHA256,
            "quantum_exponent": -20,
            "clock_target": ENDPOINT_TARGET,
            "comparison_endpoint": ENDPOINT_CLOCK,
            "lengths": [1.0, 1.0, 1.0],
            "viscosity": 1.0,
            "method": "cox-matthews",
            "integration_force_dimensions": [512, 512, 512],
            "schedule": [
                {"from_inclusive": 0, "until_exclusive": 2048, "step_ticks": 64},
                {"from_inclusive": 2048, "until_exclusive": 4096, "step_ticks": 128},
            ],
            "absolute_tolerances": [1e-5, 1e-4],
            "relative_tolerances": [1e-5, 1e-5],
        },
        "elapsed": ENDPOINT_CLOCK,
        "target": ENDPOINT_TARGET,
        "epoch": ENDPOINT_EPOCH,
        "accepted_steps": ENDPOINT_STEPS,
        "profile": {"kind": "identity-profile-field", "value": ENDPOINT_PROFILE},
        "admission_guard": {"advective_limit": ENDPOINT_ADVECTIVE_LIMIT,
                            "maximum_attempts": ENDPOINT_MAX_ATTEMPTS},
    }
