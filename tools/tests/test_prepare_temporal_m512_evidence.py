"""End-to-end M512 temporal evidence: producer output meets the consumer rule.

The adapter must materialize each reviewed arithmetic evidence file create-only
at the exact path the emitted manifests resolve beside themselves, and the
consumer in `evidence/p10/snapshot-comparison-adapter/harness/src/decode.rs`
requires the adjacent bytes to hash to `evidence_sha256` and parse to JSON
equal to the embedded ArithmeticReview. These tests replay that rule on real
producer output and refuse non-JSON or mismatched evidence fail-closed.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from tools.json_types import JsonObject
from tools.prepare_snapshot_manifest_validation import loads_checked, object_value
from tools.temporal_comparison_contract import PAIRS, SHARED_CLOCKS
from tools.tests.temporal_comparison_fixtures import FamilyRoot


def _document(path: Path) -> JsonObject:
    return dict(loads_checked(path.read_bytes(), "fixture manifest"))


def _ticks(document: JsonObject) -> list[int]:
    schedule = object_value(document["evolution"])["schedule"]
    assert isinstance(schedule, list)
    ticks: list[int] = []
    for segment in schedule:
        value = object_value(segment)["step_ticks"]
        assert isinstance(value, int)
        ticks.append(value)
    return ticks


def _reseal(family: FamilyRoot, pair: str) -> None:
    binding = family.root / "arithmetic" / f"{pair}.json"
    document = _document(binding)
    control = document["arithmetic_control"]
    assert isinstance(control, dict)
    evidence = family.root / "arithmetic" / str(control["evidence"])
    control["evidence_sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
    binding.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def test_evidence_is_materialized_exactly_where_manifests_resolve_it(
    tmp_path: Path,
) -> None:
    family = FamilyRoot(tmp_path / "root")
    output = tmp_path / "out"
    assert family.run(output) == 0
    for coarse, fine in PAIRS:
        for clock in SHARED_CLOCKS:
            directory = output / "pairs" / f"{coarse}--{fine}" / f"clock{clock:04d}"
            for name in (coarse, fine):
                manifest = _document(directory / f"{name}.json")
                assert manifest["dimensions"] == [512, 512, 512]
                evolution = object_value(manifest["evolution"])
                assert evolution["integration_force_dimensions"] == [512, 512, 512]
                control = object_value(manifest["arithmetic_control"])
                assert control["evidence"] == "arithmetic-evidence.json"
                raw = (directory / str(control["evidence"])).read_bytes()
                assert hashlib.sha256(raw).hexdigest() == control["evidence_sha256"]
                assert loads_checked(raw, "materialized evidence") == control["review"]


def test_nested_schedules_and_exact_epochs_are_emitted(tmp_path: Path) -> None:
    family = FamilyRoot(tmp_path / "root")
    output = tmp_path / "out"
    assert family.run(output) == 0
    early = output / "pairs" / "h64--h32" / "clock0512"
    assert _ticks(_document(early / "h64.json")) == [64]
    assert _ticks(_document(early / "h32.json")) == [32]
    nested = output / "pairs" / "h32--h16" / "clock2560"
    h32, h16 = _document(nested / "h32.json"), _document(nested / "h16.json")
    assert _ticks(h32) == [32, 64]
    assert _ticks(h16) == [16, 32]
    assert h32["epoch"] == h32["accepted_steps"] == 72
    assert h16["epoch"] == h16["accepted_steps"] == 144
    assert h32["elapsed"] == h16["elapsed"] == 2560


def test_non_json_arithmetic_evidence_is_refused_without_publication(
    tmp_path: Path,
) -> None:
    family = FamilyRoot(tmp_path / "root")
    (family.root / "arithmetic" / "h64--h32-lineage.json").write_bytes(b"[1, not json")
    _reseal(family, "h64--h32")
    output = tmp_path / "out"
    assert family.run(output) == 1
    assert not output.exists()


def test_evidence_review_mismatch_is_refused_without_publication(
    tmp_path: Path,
) -> None:
    family = FamilyRoot(tmp_path / "root")
    (family.root / "arithmetic" / "h32--h16-lineage.json").write_text(
        json.dumps({"schema": "unrelated-json-document"}), encoding="utf-8")
    _reseal(family, "h32--h16")
    output = tmp_path / "out"
    assert family.run(output) == 1
    assert not output.exists()
