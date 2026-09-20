"""Happy-path and determinism tests for the N512 temporal comparison adapter.

These exercise metadata-only preparation algebra on synthetic captures with the
ACTUAL reviewed h64 anchor bytes. They prove nothing about any real trajectory
and qualify no PDE window.
"""
from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess
import sys

import pytest

from temporal_comparison_fixtures import REPO, FamilyRoot, coefficient_hash, file_hash
from tools.json_types import array_value, JsonObject, object_value
from tools.prepare_snapshot_manifest_contract import ENDPOINT_IDENTITY, ENDPOINT_PLAN_SHA256
from tools.prepare_temporal_comparison import BOOKKEEPING_NAME, main
from tools.temporal_comparison_contract import (
    ADAPTER_FIXED_OVERHEAD_BYTES, COEFFICIENT_BYTES, PAIR_RESERVATION_BYTES, PAIRS,
    PAYLOAD_TOTAL_BYTES, SHARED_CLOCKS, attempts_at, branch_entries, bundle_name,
    expected_state_file_bytes, validate_family_plan,
)
from tools.prepare_snapshot_manifest_validation import (
    PreparationError, count_value, loads_checked, positive_value,
)

PACKET_PLAN = (REPO / "evidence/p10/n512-temporal-comparison-adapter-20260914/"
               "family-plan.json")


def manifest(output: Path, key: str, clock: int, side: str) -> JsonObject:
    path = output / "pairs" / key / f"clock{clock:04d}" / f"{side}.json"
    return dict(loads_checked(path.read_bytes(), "pair manifest"))


def tree_digest(root: Path) -> dict[str, str]:
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(root.rglob("*")) if path.is_file()}


def bookkeeping(output: Path) -> JsonObject:
    return dict(loads_checked((output / BOOKKEEPING_NAME).read_bytes(), "bookkeeping"))


@pytest.fixture
def family(tmp_path: Path) -> FamilyRoot:
    return FamilyRoot(tmp_path / "root")


def test_emitted_family_plan_is_the_reviewed_packet_bytes(tmp_path: Path) -> None:
    output = tmp_path / "family-plan.json"
    assert main(["--emit-family-plan", "--output", str(output)]) == 0
    assert output.read_bytes() == PACKET_PLAN.read_bytes()
    emitted = loads_checked(output.read_bytes(), "emitted family plan")
    validate_family_plan(emitted, strict=False)
    with pytest.raises(PreparationError, match="pending capture bindings"):
        validate_family_plan(emitted, strict=True)


def test_full_preparation_publishes_thirty_three_create_only_outputs(family: FamilyRoot,
                                                                    tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    files = [path for path in output.rglob("*") if path.is_file()]
    assert len(files) == 3 * 8 * len(PAIRS) + 1
    assert (output / BOOKKEEPING_NAME).is_file()
    second = family.run(output)
    assert second == 1


def test_pair_manifest_carries_exact_side_semantics(family: FamilyRoot, tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    coarse = manifest(output, "h64--h32", 2560, "h64")
    fine = manifest(output, "h64--h32", 2560, "h32")
    for side, name, steps, attempt_cap in ((coarse, "h64", 36, 48), (fine, "h32", 72, 96)):
        evolution = object_value(side["evolution"])
        assert side["schema"] == "p10-snapshot-comparison-input-v1"
        assert side["comparison_kind"] == "TIME_DIAGNOSTIC"
        assert side["elapsed"] == 2560 and side["target"] == 8192
        assert side["epoch"] == steps and side["accepted_steps"] == steps
        assert evolution["comparison_endpoint"] == 2560
        assert object_value(side["admission_guard"])["maximum_attempts"] == attempt_cap
        directory = output / "pairs" / "h64--h32" / "clock2560"
        assert (directory / str(side["snapshot"])).resolve() == (
            family.capture_path(name, 2560) / "state.bin").resolve()
        assert (directory / str(side["plan"])).resolve().is_file()
    assert coarse["coefficient_sha256"] == coefficient_hash("h64", 2560)
    assert coarse["file_sha256"] == file_hash("h64", 2560)
    assert fine["profile"] != coarse["profile"]
    assert coarse["arithmetic_control"] == fine["arithmetic_control"]
    assert coarse["identity"] != fine["identity"]


def test_truncated_schedules_end_at_every_shared_clock(family: FamilyRoot,
                                                       tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    for coarse, fine in PAIRS:
        for clock in SHARED_CLOCKS:
            for side in (coarse, fine):
                evolution = object_value(manifest(output, f"{coarse}--{fine}", clock,
                                                   side)["evolution"])
                schedule = [object_value(segment) for segment in
                            array_value(evolution["schedule"])]
                next_clock, steps = 0, 0
                for segment in schedule:
                    start = count_value(segment["from_inclusive"], "segment start")
                    until = count_value(segment["until_exclusive"], "segment end")
                    ticks = count_value(segment["step_ticks"], "segment ticks")
                    assert start == next_clock
                    assert (until - start) % ticks == 0
                    steps += (until - start) // ticks
                    next_clock = until
                assert next_clock == clock
                assert steps == attempts_at(side, clock)


def test_bookkeeping_binds_exact_names_epochs_and_order(family: FamilyRoot,
                                                        tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    book = bookkeeping(output)
    assert book["branch_order"] == ["h64", "h32", "h16"]
    assert book["pair_step_ratio"] == 2
    ledger = array_value(book["ledger"])
    assert [count_value(object_value(row)["clock"], "ledger clock") for row in ledger] \
        == list(SHARED_CLOCKS)
    for raw_row in ledger:
        sides = [object_value(side) for side in array_value(object_value(raw_row)["sides"])]
        assert [str(side["branch"]) for side in sides] == ["h64", "h32", "h16"]


def test_bookkeeping_binds_exact_side_bundles_epochs_and_hashes(family: FamilyRoot,
                                                                 tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    for raw_row in array_value(bookkeeping(output)["ledger"]):
        row = object_value(raw_row)
        clock = count_value(row["clock"], "ledger clock")
        for raw_side in array_value(row["sides"]):
            side = object_value(raw_side)
            name = str(side["branch"])
            assert side["bundle"] == bundle_name(name, clock)
            assert side["epoch"] == attempts_at(name, clock)
            assert side["accepted_steps"] == attempts_at(name, clock)
            assert side["coefficient_sha256"] == coefficient_hash(name, clock)
            assert side["file_sha256"] == file_hash(name, clock)


def test_bookkeeping_binds_pair_rows_and_epoch_ratios(family: FamilyRoot,
                                                      tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    pairs = [object_value(row) for row in array_value(bookkeeping(output)["pairs"])]
    assert [(str(row["pair"]), count_value(row["clock"], "pair clock")) for row in pairs] == [
        (f"{coarse}--{fine}", clock) for coarse, fine in PAIRS for clock in SHARED_CLOCKS]
    for row in pairs:
        assert count_value(row["coarse_epoch"], "coarse epoch") * 2 \
            == count_value(row["fine_epoch"], "fine epoch")



def test_bookkeeping_binds_state_payload_and_disk_arithmetic(family: FamilyRoot,
                                                             tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    book = dict(loads_checked((output / BOOKKEEPING_NAME).read_bytes(), "bookkeeping"))
    disk = object_value(book["disk"])
    assert disk["coefficient_bytes_per_state"] == 3233808384 == COEFFICIENT_BYTES
    assert disk["state_count"] == 24
    assert disk["coefficient_bytes_total"] == 77611401216 == PAYLOAD_TOTAL_BYTES
    assert disk["adapter_overhead_bytes_per_pair"] == 1048576 == ADAPTER_FIXED_OVERHEAD_BYTES
    assert disk["pair_reservation_bytes"] == 6468665344 == PAIR_RESERVATION_BYTES
    assert disk["pair_invocations_total"] == 6468665344 * 16
    by_branch = object_value(disk["state_file_bytes_by_branch"])
    entries = branch_entries(family.plan)
    for name in ("h64", "h32", "h16"):
        identity = str(entries[name]["identity"])
        assert count_value(by_branch[name], "state file bytes") \
            == 116 + len(identity.encode("utf-8")) + 3233808384
        assert count_value(by_branch[name], "state file bytes") \
            == expected_state_file_bytes(identity)
    assert count_value(by_branch["h64"], "state file bytes") \
        == expected_state_file_bytes(ENDPOINT_IDENTITY)


def test_h64_side_carries_the_actual_reviewed_anchor(family: FamilyRoot,
                                                     tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    side = manifest(output, "h64--h32", 4096, "h64")
    assert side["identity"] == ENDPOINT_IDENTITY
    assert side["plan_sha256"] == ENDPOINT_PLAN_SHA256
    guard = object_value(side["admission_guard"])
    assert guard["advective_limit"] == 3.3 and guard["maximum_attempts"] == 48
    directory = output / "pairs" / "h64--h32" / "clock4096"
    plan_bytes = (directory / str(side["plan"])).resolve().read_bytes()
    assert hashlib.sha256(plan_bytes).hexdigest() == ENDPOINT_PLAN_SHA256


def test_preparation_is_deterministic_and_never_touches_the_capture_root(
        family: FamilyRoot, tmp_path: Path) -> None:
    first, second = tmp_path / "first", tmp_path / "second"
    before = tree_digest(family.root)
    assert family.run(first) == 0
    assert family.run(second) == 0
    first_digest, second_digest = tree_digest(first), tree_digest(second)
    assert first_digest == second_digest
    assert set(first_digest) == {str(path.relative_to(first))
                                 for path in first.rglob("*") if path.is_file()}
    after = {name: digest for name, digest in tree_digest(family.root).items()}
    assert after == before


def test_state_payloads_are_never_opened(family: FamilyRoot, tmp_path: Path) -> None:
    locked: list[Path] = []
    for name in ("h64", "h32", "h16"):
        for clock in SHARED_CLOCKS:
            path = family.capture_path(name, clock) / "state.bin"
            path.chmod(0o000)
            locked.append(path)
    try:
        assert family.run(tmp_path / "out") == 0
    finally:
        for path in locked:
            path.chmod(0o644)


def test_bookkeeping_refuses_any_qualification_claim(family: FamilyRoot,
                                                     tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    book = dict(loads_checked((output / BOOKKEEPING_NAME).read_bytes(), "bookkeeping"))
    assert book["acceptance"] == {"status": "not_assessed", "accepted_windows": 0}
    assert "cannot accept a PDE window" in str(book["claim"])
    independence = object_value(book["independence"])
    assert independence["state_bytes_read_by_adapter"] == 0
    assert independence["state_reset_or_injection"] == "none"
    assert independence["reference_substitution"] == "none"
    assert "never re-streamed" in str(independence["hash_provenance"])


def test_cli_module_runs_as_a_standalone_script(tmp_path: Path) -> None:
    emitted = tmp_path / "emitted.json"
    finished = subprocess.run([sys.executable, str(REPO / "tools" /
                                                   "prepare_temporal_comparison.py"),
                               "--emit-family-plan", "--output", str(emitted)],
                              cwd=REPO, capture_output=True, text=True, check=False)
    assert finished.returncode == 0, finished.stderr
    assert emitted.read_bytes() == PACKET_PLAN.read_bytes()
    missing = subprocess.run([sys.executable, str(REPO / "tools" /
                                                  "prepare_temporal_comparison.py"),
                              "--output", str(tmp_path / "none")],
                             cwd=REPO, capture_output=True, text=True, check=False)
    assert missing.returncode == 1
    assert "missing required argument" in missing.stderr


def test_pair_manifest_evolution_binds_the_contract_constants(family: FamilyRoot,
                                                               tmp_path: Path) -> None:
    output = tmp_path / "out"
    assert family.run(output) == 0
    side = manifest(output, "h32--h16", 1536, "h16")
    evolution = object_value(side["evolution"])
    assert evolution["quantum_exponent"] == -20
    assert [positive_value(v, "length") for v in array_value(evolution["lengths"])] \
        == [1.0, 1.0, 1.0]
    assert evolution["viscosity"] == 1.0
    assert [positive_value(v, "absolute tolerance")
            for v in array_value(evolution["absolute_tolerances"])] == [1e-5, 1e-4]
    assert [positive_value(v, "relative tolerance")
            for v in array_value(evolution["relative_tolerances"])] == [1e-5, 1e-5]
    assert side["schema"] == "p10-snapshot-comparison-input-v1"
    assert evolution["clock_target"] == 8192
