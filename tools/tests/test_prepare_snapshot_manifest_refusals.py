"""Parameterized closed-contract refusals over the ACTUAL reviewed v3 plan bytes.

Each case tampers exactly one field of a real reviewed input and asserts the
exact refusal; these exercise guard branches the positive paths never reach.
"""
from __future__ import annotations

from collections.abc import Mapping
import hashlib
import json as json_module

import pytest

from snapshot_manifest_fixtures import (
    REPO, V3_PLAN, payload_bytes, record_document, template_document,
)
from tools.json_types import Json, JsonObject, array_value, object_value
from tools.prepare_snapshot_manifest_contract import (
    endpoint_comparison_template, validate_evolution, validate_template, verify_capture_plan,
    verify_launch_plan, verify_record_document,
)
from tools.prepare_snapshot_manifest_validation import PreparationError, loads_checked

TRAJECTORY = REPO / "evidence/p10/n512-m512-endpoint-prep-20260913/frozen-plan.json"


def tampered(document: Mapping[str, Json], path: tuple[str, ...],
             value: Json) -> JsonObject:
    result: JsonObject = dict(document)
    node: JsonObject = result
    for key in path[:-1]:
        child: JsonObject = dict(object_value(node[key]))
        node[key] = child
        node = child
    node[path[-1]] = value
    return result


def endpoint_template() -> Mapping[str, Json]:
    return validate_template(endpoint_comparison_template())


LAUNCH_REFUSALS: list[tuple[tuple[str, ...], Json, str]] = [
    (("status",), "executed", "prepared-unexecuted status"),
    (("launch_authorized",), True, "unauthorized launch plan"),
    (("endpoint_attempts_executed",), 1, "executed endpoint attempts"),
    (("feature",), "n192-m384", "reviewed N512 M512 feature"),
    (("archive_scope",), "remote durable transfer", "local-only scope"),
    (("profile",), "other-profile", "identity profile exactly"),
    (("external_stop",), "none", "external stop conflicts"),
    (("harness_commit_and_run_source",), "0" * 40, "harness commit conflicts"),
    (("numerical_test_source_commit",), "0" * 40, "numerical test source conflicts"),
    (("production_source_commit",), "0" * 40, "production source conflicts"),
    (("binary_sha256",), "zz", "binary_sha256 is not 64 hex"),
    (("historical_preparation", "immutable_trajectory_plan_sha256"), "zz", "immutable trajectory"),
    (("historical_preparation", "v2_binary_sha256"), "zz", "v2 binary is not 64 hex"),
    (("historical_preparation", "preparation_results"), "/abs/results.json", "relative path"),
    (("deadline", "source"), "wall_clock", "caller-supplied source"),
    (("deadline", "minimum_remaining_at_launch_seconds"), 0, "must be positive"),
    (("deadline", "minimum_remaining_after_first_step_seconds"), 0, "must be positive"),
    (("resources", "memory_floor_bytes"), 0, "memory_floor_bytes must be positive"),
    (("resources", "source_disk_floor_bytes"), 0, "source_disk_floor_bytes must be positive"),
    (("resources", "local_archive_disk_floor_bytes"), 0, "must be positive"),
    (("resources", "address_space_limit_bytes"), -1, "u128-range"),
    (("guards", "explicit_opt_in"), False, "explicit_opt_in must be true"),
    (("guards", "stable_process_identity_before_group_ownership"), False, "must be true"),
    (("guards", "solver_and_local_archive_share_absolute_deadline"), False, "must be true"),
    (("guards", "qualification"), True, "qualification=false"),
    (("guards", "term_grace_then_kill_seconds"), 30, "reviewed 60 seconds"),
    (("guards", "first_step"), "clock64", "first-step guard"),
    (("guards", "committed_states"), 24, "committed states conflict"),
    (("guards", "offline_observer_nodes"), 4, "offline observer nodes"),
]


@pytest.fixture(scope="module")
def actual_launch() -> JsonObject:
    return dict(loads_checked(V3_PLAN.read_bytes(), "v3 launch plan"))


@pytest.mark.parametrize(("path", "value", "message"), LAUNCH_REFUSALS,
                         ids=[("+".join(path)) for path, _, _ in LAUNCH_REFUSALS])
def test_actual_v3_launch_plan_field_refusals(
        actual_launch: JsonObject, path: tuple[str, ...], value: Json, message: str) -> None:
    template = endpoint_template()
    launch = tampered(actual_launch, path, value)
    with pytest.raises(PreparationError, match=message):
        verify_launch_plan(launch, template)


def test_actual_v3_launch_plan_unknown_field_is_refused(actual_launch: JsonObject) -> None:
    launch = tampered(actual_launch, ("extra_field",), True)
    with pytest.raises(PreparationError, match="unknown fields"):
        verify_launch_plan(launch, endpoint_template())


def test_actual_v3_launch_plan_missing_field_is_refused(actual_launch: JsonObject) -> None:
    launch = dict(actual_launch)
    del launch["guards"]
    with pytest.raises(PreparationError, match="omits"):
        verify_launch_plan(launch, endpoint_template())


def test_actual_v3_launch_plan_non_object_history_is_refused(actual_launch: JsonObject) -> None:
    launch = tampered(actual_launch, ("historical_preparation",), "flat")
    with pytest.raises(ValueError, match="Expected a JSON object"):
        verify_launch_plan(launch, endpoint_template())


CAPTURE_REFUSALS: list[tuple[tuple[str, ...], Json, str]] = [
    (("schema",), "other", "trajectory plan schema is not"),
    (("qualification",), True, "qualification=false"),
    (("case_sha256",), "0" * 64, "case identity conflicts"),
    (("profile", "clock_target"), 4096, "clock target conflicts"),
    (("profile", "endpoint"), 2048, "endpoint conflicts"),
    (("profile", "method"), "hochbruck-ostermann", "method conflicts"),
    (("profile", "analytical_reset"), True, "analytical_reset=false"),
    (("profile", "absolute_tolerances"), [1e-6, 1e-4], "absolute_tolerances conflicts"),
    (("profile", "relative_tolerances"), [1e-5, 1e-6], "relative_tolerances conflicts"),
    (("profile", "maximum_attempts"), 99, "attempt bound conflicts"),
    (("profile", "advective_limit"), 2.0, "advective limit conflicts"),
    (("captures", "resume"), "checkpoint", "resume=unsupported"),
    (("captures", "create_new"), False, "create_new=true"),
    (("captures", "positive_offline_observer_clocks"), [512, 1024], "planned offline observer"),
    (("resource_plan", "offline_observer_host"), "", "host must be non-empty"),
]


@pytest.fixture(scope="module")
def actual_capture() -> JsonObject:
    return dict(loads_checked(TRAJECTORY.read_bytes(), "trajectory plan"))


@pytest.mark.parametrize(("path", "value", "message"), CAPTURE_REFUSALS,
                         ids=[("+".join(path)) for path, _, _ in CAPTURE_REFUSALS])
def test_actual_capture_plan_field_refusals(
        actual_capture: JsonObject, path: tuple[str, ...], value: Json, message: str) -> None:
    capture = tampered(actual_capture, path, value)
    with pytest.raises(PreparationError, match=message):
        verify_capture_plan(capture, endpoint_template())


def test_capture_plan_without_template_guard_skips_guard_binding(
        actual_capture: JsonObject) -> None:
    template = endpoint_comparison_template()
    del template["admission_guard"]
    assert verify_capture_plan(actual_capture, validate_template(template)) \
        == "offline-baccus-required"


EVOLUTION_REFUSALS: list[tuple[str, Json, str]] = [
    ("case_sha256", "0" * 63, "case_sha256 is not 64 hex"),
    ("quantum_exponent", "x", "quantum_exponent must be an integer"),
    ("lengths", [1.0, 1.0], "three positive values"),
    ("lengths", [1.0, 1.0, 0.0], "positive finite"),
    ("viscosity", 0.0, "positive finite"),
    ("method", "", "non-empty"),
    ("integration_force_dimensions", [3, 3, 3], "even dimensions"),
    ("schedule", [], "non-empty list"),
    ("schedule", [{"from_inclusive": 1, "until_exclusive": 2048, "step_ticks": 64}],
     "contiguous and increasing"),
    ("schedule", [{"from_inclusive": 0, "until_exclusive": 4096, "step_ticks": 100}],
     "exact multiple"),
    ("schedule", [{"from_inclusive": 0, "until_exclusive": 2048, "step_ticks": 64}],
     "does not reach the comparison endpoint"),
    ("absolute_tolerances", [1e-5, 1e-5, 1e-5], "exactly 2 positive"),
    ("absolute_tolerances", [0.0, 1e-4], "positive finite"),
]


@pytest.mark.parametrize(("field", "value", "message"), EVOLUTION_REFUSALS,
                         ids=[field for field, _, _ in EVOLUTION_REFUSALS])
def test_evolution_field_refusals(field: str, value: Json, message: str) -> None:
    evolution = dict(object_value(endpoint_comparison_template()["evolution"]))
    evolution[field] = value
    with pytest.raises(PreparationError, match=message):
        validate_evolution(evolution)


def test_evolution_unknown_field_is_refused() -> None:
    evolution = dict(object_value(endpoint_comparison_template()["evolution"]))
    evolution["extra"] = 1
    with pytest.raises(PreparationError, match="unknown fields"):
        validate_evolution(evolution)


def test_schedule_segment_unknown_key_is_refused() -> None:
    evolution = dict(object_value(endpoint_comparison_template()["evolution"]))
    segment = dict(object_value(array_value(evolution["schedule"])[0]))
    segment["extra"] = 1
    evolution["schedule"] = [segment]
    with pytest.raises(PreparationError, match="unknown fields"):
        validate_evolution(evolution)


TEMPLATE_REFUSALS: list[tuple[tuple[str, ...], Json, str]] = [
    (("schema",), "other", "template schema is not"),
    (("comparison_kind",), "NOPE", "comparison_kind is not admitted"),
    (("comparison_kind",), [], "comparison_kind must be a string"),
    (("comparison_kind",), 5, "comparison_kind must be a string"),
    (("comparison_kind",), {}, "comparison_kind must be a string"),
    (("backend",), "", "must be non-empty"),
    (("execution",), "", "must be non-empty"),
    (("identity",), "", "empty or exceeds"),
    (("identity",), "source=none;schema=other", "identity must name schema"),
    (("source_commit",), "0" * 40, "identity source must match"),
    (("plan_sha256",), "zz", "is not 64 hex"),
    (("coefficient_sha256",), "zz", "is not 64 hex"),
    (("snapshot",), "/abs/state.bin", "plain relative file name"),
    (("profile", "kind"), "unknown-kind", "profile binding kind is not admitted"),
    (("profile", "kind"), "legacy-full-identity", "legacy-full-identity profile"),
    (("profile", "value"), "", "must be non-empty"),
    (("profile", "value"), "other-profile", "identity profile field exactly"),
    (("admission_guard", "advective_limit"), 1.5, "identity field"),
    (("admission_guard", "maximum_attempts"), 0, "must be positive"),
    (("dimensions",), [3, 3, 3], "even dimensions"),
    (("elapsed",), 2048, "frozen comparison endpoint"),
    (("target",), 4096, "frozen clock target"),
]


@pytest.mark.parametrize(("path", "value", "message"), TEMPLATE_REFUSALS,
                         ids=["+".join(path) for path, _, _ in TEMPLATE_REFUSALS])
def test_template_field_refusals(path: tuple[str, ...], value: Json, message: str) -> None:
    template = tampered(endpoint_comparison_template(), path, value)
    with pytest.raises(PreparationError, match=message):
        validate_template(template)


def test_template_unknown_field_is_refused() -> None:
    template = tampered(endpoint_comparison_template(), ("extra",), 1)
    with pytest.raises(PreparationError, match="unknown fields"):
        validate_template(template)


def test_template_legacy_identity_profile_binding_is_admitted() -> None:
    template = endpoint_comparison_template()
    template["profile"] = {"kind": "legacy-full-identity", "value": template["identity"]}
    assert validate_template(template)["profile"] == template["profile"]


RECORD_REFUSALS: list[tuple[tuple[str, ...], Json, str]] = [
    (("schema",), "other", "record schema is not"),
    (("identity",), "different", "identity does not match"),
    (("resumable",), True, "resumable=false"),
    (("qualification",), True, "qualification=false"),
    (("observation_status",), "Projected", "observation_status is not"),
    (("offline_observer_node",), False, "offline_observer_node=true"),
    (("observer_execution",), "local", "observer_execution must be the reviewed"),
    (("clock",), 1536, "frozen comparison endpoint"),
    (("epoch",), 24, "epoch does not match"),
    (("accepted_steps",), 24, "accepted_steps does not match"),
    (("coefficient_bytes",), 383, "streamed payload"),
    (("state_sha256",), "zz", "is not 64 hex"),
    (("extra",), 1, "unknown fields"),
]


@pytest.mark.parametrize(("path", "value", "message"), RECORD_REFUSALS,
                         ids=["+".join(path) for path, _, _ in RECORD_REFUSALS])
def test_endpoint_record_field_refusals(path: tuple[str, ...], value: Json,
                                        message: str) -> None:
    # One small independent endpoint-consistent state fixture so each guard
    # branch is reached in isolation; the actual clock1536 refusal has its
    # own dedicated test in the contract suite.
    payload = payload_bytes()
    record = tampered(record_document(payload), path, value)
    template = validate_template(template_document("1" * 64))
    with pytest.raises(PreparationError, match=message):
        verify_record_document(record, template, len(payload),
                               hashlib.sha256(payload).hexdigest(), "offline-baccus-required")


def nested_json(levels: int) -> bytes:
    return b'{"a": ' * levels + b"1" + b"}" * levels


def test_json_nesting_at_the_bound_is_admitted() -> None:
    assert loads_checked(nested_json(32), "nested") == nested_python(32)


def nested_python(levels: int) -> JsonObject:
    document: JsonObject = {"a": 1}
    for _ in range(levels - 1):
        document = {"a": document}
    return document


def test_json_nesting_past_the_bound_is_refused() -> None:
    with pytest.raises(PreparationError, match="nests deeper than 32 levels"):
        loads_checked(nested_json(33), "nested")


def test_parser_recursion_error_is_converted_to_refusal(monkeypatch: pytest.MonkeyPatch) -> None:
    def exploding_loads(*args: object, **kwargs: object) -> object:
        raise RecursionError("mocked parser stack exhaustion")

    monkeypatch.setattr(json_module, "loads", exploding_loads)
    with pytest.raises(PreparationError, match="nests deeper"):
        loads_checked(b"{}", "nested")


def test_malformed_json_is_a_labelled_refusal() -> None:
    with pytest.raises(PreparationError, match="is invalid JSON"):
        loads_checked(b"{oops", "nested")


def test_multi_kibibyte_deep_nesting_refuses_without_recursion_error() -> None:
    deep = b'{"nest": ' + b"[" * 1100 + b"]" * 1100 + b"}"
    assert len(deep) >= 2 * 1024
    with pytest.raises(PreparationError, match="nests deeper"):
        loads_checked(deep, "nested")


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
