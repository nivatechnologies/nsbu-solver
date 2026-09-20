"""Refusal matrix for the N512 temporal comparison adapter.

Every case tampers one metadata input and requires exit 1 with nothing
published; the adapter never reads a state payload and its output can never
accept a PDE window.
"""
from __future__ import annotations

import contextlib
import copy
import io
import json
import re
import shutil
from pathlib import Path
from typing import Callable

import pytest

from temporal_comparison_fixtures import FamilyRoot, coefficient_hash
from tools.json_types import Json, JsonObject, array_value
from tools.prepare_snapshot_manifest_validation import loads_checked
from tools.prepare_temporal_comparison import INVENTORY_NAME
from tools.temporal_comparison_contract import COEFFICIENT_BYTES, branch_entries, emitted_family_plan


def run_expect(family: FamilyRoot, output: Path, pattern: str,
               plan: JsonObject | None = None) -> None:
    errors = io.StringIO()
    with contextlib.redirect_stderr(errors):
        status = family.run(output, plan)
    assert status == 1
    assert re.search(pattern, errors.getvalue()), errors.getvalue()
    assert not output.exists()


def tamper_branch(family: FamilyRoot, name: str, **overrides: Json) -> JsonObject:
    plan: JsonObject = copy.deepcopy(family.plan)
    for raw in array_value(plan["branches"]):
        assert isinstance(raw, dict)
        if raw["name"] == name:
            raw.update(overrides)
    return plan


def test_missing_bundle_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    shutil.rmtree(family.capture_path("h16", 3584))
    run_expect(family, tmp_path / "out", "capture bundle step-176-clock-3584 is missing")


def test_missing_state_payload_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.capture_path("h32", 512) / "state.bin").unlink()
    run_expect(family, tmp_path / "out", "state payload is missing")


def test_missing_record_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.capture_path("h64", 1024) / "record.json").unlink()
    run_expect(family, tmp_path / "out", "record is missing")


def test_wrong_bundle_layout_name_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    source = family.capture_path("h64", 512)
    source.rename(source.parent / "step-009-clock-0512")
    run_expect(family, tmp_path / "out", "capture bundle step-008-clock-0512 is missing")


@pytest.mark.parametrize(("field", "value", "pattern"), [
    ("clock", 1024, "not the shared clock 512"),
    ("epoch", 9, "exact schedule epoch 8"),
    ("accepted_steps", 7, "exact 8 from-rest count"),
    ("coefficient_bytes", COEFFICIENT_BYTES + 1, "bound 3233808384"),
    ("resumable", True, "resumable=false from rest"),
    ("qualification", True, "qualification=false"),
    ("observation_status", "NotScheduled", "CapturedActualState observer capture"),
    ("offline_observer_node", False, "offline_observer_node=true"),
    ("observer_execution", "inline-observer", "observer execution conflicts"),
    ("schema", "other-schema", "record schema is not"),
])
def test_record_field_tampers_are_refused(family: FamilyRoot, tmp_path: Path, field: str,
                                          value: Json, pattern: str) -> None:
    family.rewrite_record("h64", 512, **{field: value})
    run_expect(family, tmp_path / "out", pattern)


def test_cross_branch_state_copy_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    family.rewrite_record("h32", 512, state_sha256=coefficient_hash("h64", 512))
    run_expect(family, tmp_path / "out", "pairwise distinct across the 24")


def test_reference_substituted_record_identity_is_refused(family: FamilyRoot,
                                                          tmp_path: Path) -> None:
    h64_identity = str(branch_entries(family.plan)["h64"]["identity"])
    family.rewrite_record("h32", 1024, identity=h64_identity)
    run_expect(family, tmp_path / "out", "immutable branch identity")


def test_duplicate_json_key_record_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    path = family.capture_path("h16", 4096) / "record.json"
    path.write_text('{"schema": "a", "schema": "b"}', encoding="utf-8")
    run_expect(family, tmp_path / "out", "duplicate JSON keys")


def test_non_finite_inventory_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.root / INVENTORY_NAME).write_text('{"schema": "a", "files": {"k": 1e999}}',
                                              encoding="utf-8")
    run_expect(family, tmp_path / "out", "non-finite")


def rewrite_inventory(family: FamilyRoot,
                      mutate: Callable[[dict[str, Json]], object]) -> None:
    path = family.root / INVENTORY_NAME
    document = dict(loads_checked(path.read_bytes(), "inventory fixture"))
    files = document["files"]
    assert isinstance(files, dict)
    mutate(files)
    path.write_text(json.dumps(document), encoding="utf-8")


def test_missing_inventory_key_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    rewrite_inventory(family, lambda files: files.pop("h64/step-008-clock-0512/state.bin"))
    run_expect(family, tmp_path / "out", "missing")


def test_extra_inventory_key_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    rewrite_inventory(family, lambda files: files.__setitem__(
        "h64/step-009-clock-0576/state.bin", "1" * 64))
    run_expect(family, tmp_path / "out", "unexpected")


def test_duplicate_inventory_hashes_are_refused(family: FamilyRoot, tmp_path: Path) -> None:
    rewrite_inventory(family, lambda files: files.__setitem__(
        "h32/step-016-clock-0512/state.bin", files["h64/step-008-clock-0512/state.bin"]))
    run_expect(family, tmp_path / "out", "pairwise distinct")


def test_hash_layer_collision_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    rewrite_inventory(family, lambda files: files.__setitem__(
        "h16/step-032-clock-0512/state.bin", coefficient_hash("h16", 512)))
    run_expect(family, tmp_path / "out", "layers must be disjoint")


def test_tampered_capture_plan_file_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.root / "h32" / "capture-plan.json").write_text('{"capture-plan": "edited"}',
                                                           encoding="utf-8")
    run_expect(family, tmp_path / "out", "capture plan hash does not match")


@pytest.mark.parametrize(("overrides", "pattern"), [
    ({"absolute_tolerances": [1e-4, 1e-4]}, "frozen common policy"),
    ({"advective_limit": 6.6}, "guard 3.3"),
    ({"case_sha256": "0" * 64}, "frozen similarity-mms-v2 case"),
    ({"coefficient_bytes": COEFFICIENT_BYTES - 1}, "bound disk arithmetic"),
    ({"shared_clocks": [512, 1024]}, "exactly 512 through 4096"),
    ({"pairs": [{"coarse": "h32", "fine": "h64"}, {"coarse": "h32", "fine": "h16"}]},
     "nested coarse/fine order"),
])
def test_family_plan_tampers_are_refused(family: FamilyRoot, tmp_path: Path,
                                         overrides: dict[str, Json], pattern: str) -> None:
    run_expect(family, tmp_path / "out", pattern, family.tampered(**overrides))


def test_h64_anchor_relabel_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    run_expect(family, tmp_path / "out", "reviewed v3 anchor",
               tamper_branch(family, "h64", profile="relabelled-h64"))


def test_pending_family_plan_run_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    run_expect(family, tmp_path / "out", "pending capture bindings", emitted_family_plan())


def tamper_binding(family: FamilyRoot, key: str, mutate: Callable[[JsonObject], None]) -> None:
    path = family.root / "arithmetic" / f"{key}.json"
    document = dict(loads_checked(path.read_bytes(), "binding fixture"))
    mutate(document)
    path.write_text(json.dumps(document), encoding="utf-8")


def binding_review(document: JsonObject) -> JsonObject:
    control = document["arithmetic_control"]
    assert isinstance(control, dict)
    review = control["review"]
    assert isinstance(review, dict)
    return review


def binding_lineage(document: JsonObject) -> JsonObject:
    lineage = binding_review(document)["reviewed_lineage"]
    assert isinstance(lineage, dict)
    return lineage


def test_missing_arithmetic_binding_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.root / "arithmetic" / "h32--h16.json").unlink()
    run_expect(family, tmp_path / "out", "arithmetic binding is missing")


def test_tampered_arithmetic_evidence_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.root / "arithmetic" / "h64--h32-lineage.json").write_text("edited",
                                                                      encoding="utf-8")
    run_expect(family, tmp_path / "out", "evidence hash does not match")


def test_wrong_pair_label_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    tamper_binding(family, "h64--h32", lambda document: document.update({"pair": "h32--h16"}))
    run_expect(family, tmp_path / "out", "not for the declared nested pair")


def test_lineage_execution_conflict_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    def mutate(document: JsonObject) -> None:
        left = binding_lineage(document)["left"]
        assert isinstance(left, dict)
        left.update({"execution": "some-other-execution"})

    tamper_binding(family, "h64--h32", mutate)
    run_expect(family, tmp_path / "out", "conflicts with the h64 branch binding")


def test_lineage_profile_relabel_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    def mutate(document: JsonObject) -> None:
        right = binding_lineage(document)["right"]
        assert isinstance(right, dict)
        right.update({"profile": {"kind": "identity-profile-field", "value": "relabelled"}})

    tamper_binding(family, "h64--h32", mutate)
    run_expect(family, tmp_path / "out", "does not name the h32 profile exactly")


def test_unadmitted_arithmetic_conclusion_is_refused(family: FamilyRoot,
                                                     tmp_path: Path) -> None:
    def mutate(document: JsonObject) -> None:
        binding_review(document).update({"conclusion": "reviewed-inconclusive"})

    tamper_binding(family, "h64--h32", mutate)
    run_expect(family, tmp_path / "out", "admitted reviewed equivalence")


def test_record_unknown_field_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    family.rewrite_record("h64", 1536, injected_field=1)
    run_expect(family, tmp_path / "out", "unknown fields")


def test_oversized_record_is_refused_at_its_byte_bound(family: FamilyRoot,
                                                       tmp_path: Path) -> None:
    path = family.capture_path("h64", 512) / "record.json"
    record = dict(loads_checked(path.read_bytes(), "record fixture"))
    record["pad"] = "x" * 9000
    path.write_text(json.dumps(record), encoding="utf-8")
    run_expect(family, tmp_path / "out", "exceeds its 8192 byte bound")


def test_oversized_inventory_is_refused_at_its_byte_bound(family: FamilyRoot,
                                                          tmp_path: Path) -> None:
    (family.root / INVENTORY_NAME).write_text(
        json.dumps({"schema": "p10-n512-temporal-file-inventory-v1",
                    "files": {"k": "y" * 70000}}), encoding="utf-8")
    run_expect(family, tmp_path / "out", "exceeds 65536 byte bound")


def test_wrong_inventory_schema_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    document = dict(loads_checked((family.root / INVENTORY_NAME).read_bytes(), "inventory"))
    document["schema"] = "p10-inventory-v0"
    (family.root / INVENTORY_NAME).write_text(json.dumps(document), encoding="utf-8")
    run_expect(family, tmp_path / "out", "inventory schema is not")


def test_non_object_inventory_files_is_refused(family: FamilyRoot, tmp_path: Path) -> None:
    (family.root / INVENTORY_NAME).write_text(
        json.dumps({"schema": "p10-n512-temporal-file-inventory-v1", "files": []}),
        encoding="utf-8")
    run_expect(family, tmp_path / "out", "inventory files must be an object")


def test_unreadable_capture_plan_is_refused_without_partial_output(family: FamilyRoot,
                                                                   tmp_path: Path) -> None:
    path = family.root / "h32" / "capture-plan.json"
    path.chmod(0o000)
    try:
        run_expect(family, tmp_path / "out", "capture plan could not be read")
    finally:
        path.chmod(0o644)


def test_publication_uncertainty_exits_two_and_stays_incomplete(family: FamilyRoot,
                                                                tmp_path: Path,
                                                                monkeypatch: pytest.MonkeyPatch,
                                                                ) -> None:
    import os

    output = tmp_path / "out"
    real_link = os.link
    calls = [0]

    def racing_link(source: str, destination: str, *args: object, **kwargs: object) -> None:
        calls[0] += 1
        if calls[0] == 3:
            Path(destination).write_bytes(b"RACE")
            raise OSError(28, "No space left on device")
        real_link(source, destination, *args, **kwargs)

    monkeypatch.setattr(os, "link", racing_link)
    errors = io.StringIO()
    with contextlib.redirect_stderr(errors):
        assert family.run(output) == 2
    assert re.search("publication uncertain", errors.getvalue())
    book = output / "bookkeeping.json"
    assert not book.exists()
    raced = [path for path in output.rglob("*.json") if path.read_bytes() == b"RACE"]
    assert len(raced) == 1
    staged = [path for path in output.rglob(".*partial*")]
    assert staged
    monkeypatch.undo()
    errors = io.StringIO()
    with contextlib.redirect_stderr(errors):
        assert family.run(output) == 1
    assert re.search("refusing to overwrite", errors.getvalue())
    assert not book.exists()


@pytest.fixture
def family(tmp_path: Path) -> FamilyRoot:
    return FamilyRoot(tmp_path / "root")
