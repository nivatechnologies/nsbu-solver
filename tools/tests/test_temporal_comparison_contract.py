"""Adversarial contract matrix for the closed N512 temporal family contract.

Every case tampers one plan or binding document and requires an explicit
PreparationError; preparation-only algebra on synthetic metadata that proves
nothing about any real trajectory and qualifies no PDE window.
"""
from __future__ import annotations

import copy
from typing import Callable

import pytest

from temporal_comparison_fixtures import (
    arithmetic_binding, concrete_family_plan, synthetic_identity,
)
from tools.json_types import Json, JsonObject, array_value, object_value
from tools.prepare_snapshot_manifest_validation import PreparationError, count_value
from tools.temporal_comparison_contract import (
    ARITHMETIC_SCHEMA, BACKEND, COEFFICIENT_BYTES, OBSERVER_EXECUTION,
    PENDING_ARITHMETIC_BINDING, PENDING_BRANCH_IDENTITY, PENDING_BRANCH_PLAN_SHA256,
    PENDING_BRANCH_PROFILE, PENDING_TOKENS, QUANTUM_EXPONENT, SHARED_CLOCKS, STATE_SCHEMA,
    arithmetic_review_contract, attempts_at, branch_entries, bundle_name,
    emitted_family_plan, expected_state_file_bytes, is_pending, schedule_matches,
    schedule_through, validate_family_plan,
)

EPOCHS: dict[str, list[int]] = {
    "h64": [8, 16, 24, 32, 36, 40, 44, 48],
    "h32": [16, 32, 48, 64, 72, 80, 88, 96],
    "h16": [32, 64, 96, 128, 144, 160, 176, 192],
}

FAMILY_REFUSALS: list[tuple[str, Json, str]] = [
    ("schema", "other-family", "temporal-family-plan-v1"),
    ("status", "qualified", "prepared-only no-qualification"),
    ("comparison_kind", "MATCHED_SPATIAL", "comparison_kind is not TIME_DIAGNOSTIC"),
    ("state_schema", "other-state", "state_schema is not"),
    ("method", "rk4", "method must be cox-matthews"),
    ("dimensions", [512, 512, 256], "retain 512 with M512 force"),
    ("dimensions", [511, 512, 512], "even dimensions"),
    ("integration_force_dimensions", [512, 512, 256], "retain 512 with M512 force"),
    ("case_sha256", "f" * 64, "frozen similarity-mms-v2 case"),
    ("case_sha256", "not-hex", "not 64 hex digits"),
    ("quantum_exponent", -19, "frozen -20"),
    ("quantum_exponent", True, "frozen -20"),
    ("clock_target", 4096, "frozen target 8192"),
    ("trajectory_endpoint", 2048, "frozen 4096"),
    ("switch_clock", 1024, "nested-family clock 2048"),
    ("shared_clocks", [512, 1024, 1536, 2048, 2560, 3072, 3584], "exactly 512 through 4096"),
    ("shared_clocks", {"clock": 512}, "exactly 512 through 4096"),
    ("shared_clocks", [512, 1024, 1536, 2048, 2560, 3072, 3584, 4096.5], "non-negative integer"),
    ("lengths", [1.0, 1.0, 2.0], "unit cube"),
    ("lengths", [1.0, 1.0], "unit cube"),
    ("viscosity", 2.0, "frozen case setting"),
    ("relative_tolerances", [1e-5, 1e-4], "frozen common policy"),
    ("absolute_tolerances", [0.0, 1e-4], "positive finite"),
    ("backend", "fftw-3", "reviewed AVX backend"),
    ("adapter_overhead_bytes", (1 << 20) + 1, "bound disk arithmetic"),
    ("pair_reservation_bytes", 2 * COEFFICIENT_BYTES, "bound disk arithmetic"),
    ("coefficient_bytes_total", COEFFICIENT_BYTES, "bound disk arithmetic"),
    ("pairs", [{"coarse": "h32", "fine": "h16"}, {"coarse": "h64", "fine": "h32"}],
     "nested coarse/fine order"),
    ("pairs", {"h64--h32": True}, "nested coarse/fine order"),
    ("branches", {}, "exactly the three nested branches"),
    ("branches", [{}, {}, {}], "omits"),
]

BRANCH_REFUSALS: list[tuple[str, Json, str]] = [
    ("name", "h8", "appear in family order"),
    ("coarse_step_ticks", 63, "conflicts with the closed family"),
    ("late_step_ticks", 129, "conflicts with the closed family"),
    ("maximum_attempts", 49, "conflicts with the closed family"),
    ("schedule_name", "h32-whatever", "schedule name conflicts"),
    ("profile_prefix", "wrong-prefix", "profile prefix conflicts"),
    ("observer_execution", "inline-observer", "observer execution conflicts"),
    ("case", "0" * 64, "frozen case identity"),
    ("execution", "", "execution must be non-empty"),
    ("plan_sha256", "0" * 63, "not 64 hex digits"),
    ("source_commit", "z" * 40, "is not 40 hex digits"),
    ("plan_file", "../escape.json", "plain relative file name"),
    ("invented", 1, "unknown fields"),
]


def replace_identity_field(identity: str, key: str, value: str) -> str:
    fields: list[str] = []
    for field in identity.split(";"):
        name, separator, _ = field.partition("=")
        fields.append(f"{name}={value}" if separator and name == key else field)
    return ";".join(fields)


def drop_identity_field(identity: str, key: str) -> str:
    return ";".join(field for field in identity.split(";")
                    if not field.startswith(f"{key}="))


IDENTITY_REFUSALS: list[tuple[Callable[[str], str], str]] = [
    (lambda text: drop_identity_field(text, "backend"), "identity must name backend="),
    (lambda text: text + f";schema={STATE_SCHEMA}", "schema=.*exactly once"),
    (lambda text: text + ";host=", "identity must name host exactly once"),
    (lambda text: text.replace("external_stop=pgid-watchdog-v3-confirmed-identity-absolute"
                              "-deadline", "external_stop=", 1),
     "identity must name external_stop exactly once"),
    (lambda text: replace_identity_field(text, "production_source", "not-a-hash"),
     "identity production_source is not 40 hex digits"),
    (lambda text: drop_identity_field(text, "profile"), "identity must name profile="),
    (lambda text: replace_identity_field(text, "observer_execution", "inline-observer"),
     "observer_execution=offline-baccus-required exactly once"),
]

ARITHMETIC_PATH_REFUSALS: list[tuple[str, str]] = [
    (PENDING_ARITHMETIC_BINDING, "pending arithmetic binding"),
    ("../escape.json", "safe relative path"),
    ("", "safe relative path"),
    ("/etc/passwd", "safe relative path"),
]

BINDING_REFUSALS: list[tuple[tuple[str, ...], Json, str]] = [
    (("schema",), "other", "schema is not p10-n512-temporal-arithmetic-binding-v1"),
    (("pair",), "h32--h16", "not for the declared nested pair"),
    (("invented",), 1, "unknown fields"),
    (("arithmetic_control", "evidence"), "/abs.json", "plain relative file name"),
    (("arithmetic_control", "evidence"), "", "plain relative file name"),
    (("arithmetic_control", "evidence_sha256"), "0" * 63, "not 64 hex digits"),
    (("arithmetic_control", "review", "schema"), "other", "p10-time-arithmetic-review-v1"),
    (("arithmetic_control", "review", "conclusion"), "inconclusive",
     "admitted reviewed equivalence"),
    (("arithmetic_control", "review", "case_sha256"), "0" * 64, "conflicts with the frozen case"),
    (("arithmetic_control", "review", "method"), "rk4", "frozen method"),
    (("arithmetic_control", "review", "integration_force_dimensions"), [512, 512, 256],
     "conflicts with M512"),
    (("arithmetic_control", "review", "measured_control", "outcome"), "partial",
     "successful exact-bit"),
    (("arithmetic_control", "review", "measured_control", "serial", "source_commit"), "z" * 40,
     "not 40 hex digits"),
    (("arithmetic_control", "review", "measured_control", "w3", "extra"), 1, "unknown fields"),
    (("arithmetic_control", "review", "reviewed_lineage", "status"), "unreviewed",
     "reviewed unchanged kernel lineage"),
    (("arithmetic_control", "review", "reviewed_lineage", "left_control_role"), "w3",
     "serial/w3 roles"),
    (("arithmetic_control", "review", "reviewed_lineage", "left", "source_commit"), "a" * 40,
     "conflicts with the h64 branch binding"),
    (("arithmetic_control", "review", "reviewed_lineage", "right", "backend"), "fftw",
     "conflicts with the h32 branch binding"),
    (("arithmetic_control", "review", "reviewed_lineage", "right", "profile", "kind"),
     "legacy-full-identity", "does not name the h32 profile exactly"),
]


@pytest.fixture
def plan() -> JsonObject:
    return concrete_family_plan()


def refuse(values: JsonObject, pattern: str, *, strict: bool = True) -> None:
    with pytest.raises(PreparationError, match=pattern):
        validate_family_plan(values, strict=strict)


def branch_list(plan: JsonObject) -> list[JsonObject]:
    return [dict(object_value(entry)) for entry in array_value(plan["branches"])]


def edit_branch(plan: JsonObject, target: str, **overrides: Json) -> JsonObject:
    document = copy.deepcopy(plan)
    branches: list[Json] = [dict(object_value(entry))
                            for entry in array_value(document["branches"])]
    document["branches"] = branches
    for entry in branches:
        assert isinstance(entry, dict)
        if str(entry["name"]) == target:
            entry.update(overrides)
    return document


def tamper(document: JsonObject, path: tuple[str, ...], value: Json) -> JsonObject:
    result: JsonObject = dict(document)
    node: JsonObject = result
    for key in path[:-1]:
        child: JsonObject = dict(object_value(node[key]))
        node[key] = child
        node = child
    node[path[-1]] = value
    return result


@pytest.mark.parametrize("name", ("h64", "h32", "h16"))
@pytest.mark.parametrize(("index", "clock"), list(enumerate(SHARED_CLOCKS)))
def test_epoch_ladders_are_exact(name: str, index: int, clock: int) -> None:
    assert attempts_at(name, clock) == EPOCHS[name][index]


def test_bundle_names_are_zero_padded_and_epoch_bound() -> None:
    assert bundle_name("h64", 512) == "step-008-clock-0512"
    assert bundle_name("h16", 2048) == "step-128-clock-2048"
    assert bundle_name("h16", 4096) == "step-192-clock-4096"


def test_schedule_through_covers_every_clock_without_gaps() -> None:
    for name, ladder in EPOCHS.items():
        for clock, expected in zip(SHARED_CLOCKS, ladder):
            segments = schedule_through(name, clock)
            assert count_value(segments[0]["from_inclusive"], "segment start") == 0
            steps = 0
            for segment in segments:
                start = count_value(segment["from_inclusive"], "segment start")
                until = count_value(segment["until_exclusive"], "segment end")
                ticks = count_value(segment["step_ticks"], "segment ticks")
                assert (until - start) % ticks == 0
                steps += (until - start) // ticks
            assert count_value(segments[-1]["until_exclusive"], "segment end") == clock
            assert steps == expected


def test_schedule_through_refuses_non_shared_clocks() -> None:
    for clock in (0, 300, 2049, 4097, 8192):
        with pytest.raises(PreparationError, match="exact shared comparison clock"):
            schedule_through("h32", clock)


@pytest.mark.parametrize("token", sorted(PENDING_TOKENS))
def test_pending_tokens_are_recognized(token: str) -> None:
    assert is_pending(token)
    assert not is_pending(token + "-tampered")


def test_constants_are_the_frozen_family_values() -> None:
    assert QUANTUM_EXPONENT == -20
    assert ARITHMETIC_SCHEMA == "p10-n512-temporal-arithmetic-binding-v1"


def test_expected_state_file_bytes_binds_identity_width() -> None:
    assert expected_state_file_bytes("ab") == 116 + 2 + COEFFICIENT_BYTES


def test_valid_concrete_plan_passes_strict_validation(plan: JsonObject) -> None:
    assert validate_family_plan(plan, strict=True) is plan


def test_float_tick_schedule_is_refused(plan: JsonObject) -> None:
    refuse(edit_branch(plan, "h64", schedule=[
        {"from_inclusive": 0.0, "until_exclusive": 2048, "step_ticks": 64},
        {"from_inclusive": 2048, "until_exclusive": 4096, "step_ticks": 128}]),
        "schedule conflicts with the closed family")


def test_boolean_tick_and_extra_segment_fields_are_refused(plan: JsonObject) -> None:
    refuse(edit_branch(plan, "h32", schedule=[
        {"from_inclusive": False, "until_exclusive": 2048, "step_ticks": 32},
        {"from_inclusive": 2048, "until_exclusive": 4096, "step_ticks": 64}]),
        "schedule conflicts with the closed family")
    refuse(edit_branch(plan, "h32", schedule=[
        {"from_inclusive": 0, "until_exclusive": 2048, "step_ticks": 32, "extra": 1},
        {"from_inclusive": 2048, "until_exclusive": 4096, "step_ticks": 64}]),
        "schedule conflicts with the closed family")
    assert not schedule_matches("h32", [])
    assert not schedule_matches("h32", "not-a-list")


@pytest.mark.parametrize(("field", "value", "pattern"), FAMILY_REFUSALS)
def test_family_field_tampers_are_refused(plan: JsonObject, field: str, value: Json,
                                          pattern: str) -> None:
    tampered_plan = tamper(plan, (field,), value)
    refuse(tampered_plan, pattern)


def test_missing_and_unknown_family_fields_are_refused(plan: JsonObject) -> None:
    without_switch = dict(plan)
    del without_switch["switch_clock"]
    refuse(without_switch, "omits")
    refuse(tamper(plan, ("invented",), 1), "unknown fields")


@pytest.mark.parametrize(("field", "value", "pattern"), BRANCH_REFUSALS)
def test_branch_field_tampers_are_refused(plan: JsonObject, field: str, value: Json,
                                          pattern: str) -> None:
    refuse(edit_branch(plan, "h32", **{field: value}), pattern)


def test_h64_anchor_field_tamper_is_refused(plan: JsonObject) -> None:
    refuse(edit_branch(plan, "h64", plan_sha256="0" * 64), "reviewed v3 anchor")


def test_branch_missing_field_and_order_are_refused(plan: JsonObject) -> None:
    without_case = edit_branch(plan, "h32")
    entries = branch_list(without_case)
    del entries[1]["case"]
    without_case["branches"] = list(entries)
    refuse(without_case, "omits")
    swapped = copy.deepcopy(plan)
    order = branch_list(swapped)
    order[0], order[1] = order[1], order[0]
    swapped["branches"] = list(order)
    refuse(swapped, "appear in family order at h64")


@pytest.mark.parametrize(("mutate", "pattern"), IDENTITY_REFUSALS)
def test_identity_tampers_are_refused(plan: JsonObject, mutate: Callable[[str], str],
                                      pattern: str) -> None:
    entry = branch_entries(plan)["h32"]
    refuse(edit_branch(plan, "h32", identity=mutate(str(entry["identity"]))), pattern)


def test_oversized_identity_and_prefix_equal_profile_are_refused(plan: JsonObject) -> None:
    identity = synthetic_identity("h32") + ";pad=" + "p" * 20000
    refuse(edit_branch(plan, "h32", identity=identity), "empty or exceeds 16 KiB")
    entry = branch_entries(plan)["h16"]
    prefix = str(entry["profile_prefix"])
    identity = replace_identity_field(str(entry["identity"]), "profile", prefix)
    refuse(edit_branch(plan, "h16", identity=identity, profile=prefix),
           "must extend its reviewed prefix exactly")


def test_pending_branches_are_fail_closed_under_strict_only(plan: JsonObject) -> None:
    pending = edit_branch(plan, "h32", identity=PENDING_BRANCH_IDENTITY)
    validate_family_plan(pending, strict=False)
    refuse(pending, "pending capture bindings")
    refuse(edit_branch(plan, "h16", profile=PENDING_BRANCH_PROFILE),
           "pending capture bindings")


@pytest.mark.parametrize(("value", "pattern"), ARITHMETIC_PATH_REFUSALS)
def test_arithmetic_binding_path_tampers_are_refused(plan: JsonObject, value: str,
                                                     pattern: str) -> None:
    tampered_plan = tamper(plan, ("arithmetic_files", "h32--h16"), value)
    refuse(tampered_plan, pattern)


def test_missing_arithmetic_pair_key_is_refused(plan: JsonObject) -> None:
    refuse(tamper(plan, ("arithmetic_files",), {"h64--h32": "arithmetic/h64--h32.json"}),
           "bind an arithmetic review per nested pair")


@pytest.fixture
def binding() -> tuple[JsonObject, JsonObject]:
    concrete = concrete_family_plan()
    return arithmetic_binding("h64", "h32", concrete), concrete


def test_valid_arithmetic_binding_returns_its_control(binding: tuple[JsonObject,
                                                                    JsonObject]) -> None:
    document, concrete = binding
    control = arithmetic_review_contract(document, "h64", "h32", concrete)
    assert control == document["arithmetic_control"]


@pytest.mark.parametrize(("path", "value", "pattern"), BINDING_REFUSALS)
def test_arithmetic_binding_tampers_are_refused(binding: tuple[JsonObject, JsonObject],
                                                path: tuple[str, ...], value: Json,
                                                pattern: str) -> None:
    document, concrete = binding
    with pytest.raises(PreparationError, match=pattern):
        arithmetic_review_contract(tamper(document, path, value), "h64", "h32", concrete)


def test_arithmetic_lineage_against_unlisted_branches_is_refused(binding: tuple[
        JsonObject, JsonObject]) -> None:
    document, concrete = binding
    broken = copy.deepcopy(concrete)
    broken["branches"] = {}
    with pytest.raises(PreparationError, match="branches must be a list"):
        arithmetic_review_contract(document, "h64", "h32", broken)


def test_emitted_packet_arithmetic_is_pending_until_reviewed() -> None:
    emitted = emitted_family_plan()
    validate_family_plan(emitted, strict=False)
    with pytest.raises(PreparationError, match="pending capture bindings"):
        validate_family_plan(emitted, strict=True)
    assert branch_entries(emitted)["h16"]["plan_sha256"] == PENDING_BRANCH_PLAN_SHA256


def test_independent_identities_never_reuse_the_h64_anchor(plan: JsonObject) -> None:
    entries = branch_entries(validate_family_plan(plan, strict=True))
    assert entries["h64"]["identity"] != entries["h32"]["identity"]
    assert entries["h64"]["identity"] != entries["h16"]["identity"]
    assert entries["h32"]["profile"] != entries["h16"]["profile"]
    for name in ("h32", "h16"):
        identity = str(entries[name]["identity"])
        assert identity.count(f"profile={entries[name]['profile']}") == 1
        assert f"observer_execution={OBSERVER_EXECUTION}" in identity.split(";")
        assert identity.count(f"backend={BACKEND}") == 1
