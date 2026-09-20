"""Synthetic metadata-only fixtures for the N512 temporal comparison adapter.

Every state payload is an empty placeholder: the adapter is metadata-only and
the fixtures declare the bound 3,233,808,384-byte coefficient payload in the
records without ever storing or reading a multi-GiB array.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Final

from tools.json_types import Json, JsonObject
from tools.prepare_snapshot_manifest_validation import loads_checked
from tools.prepare_temporal_comparison import main
from tools.temporal_comparison_contract import (
    ARITHMETIC_SCHEMA, BACKEND, BRANCH_ORDER, COEFFICIENT_BYTES, INVENTORY_SCHEMA,
    OBSERVER_EXECUTION, PAIRS, SHARED_CLOCKS, STATE_SCHEMA, attempts_at, branch_entries,
    bundle_name, emitted_family_plan, profile_prefix, schedule_name,
)

REPO = Path(__file__).resolve().parents[2]
V3_PLAN_BYTES: Final = (REPO / "evidence/p10/n512-m512-endpoint-prep-20260913/"
                        "proposed-launch/v3-launch-plan.json").read_bytes()
INVENTORY_NAME: Final = "state-inventory.json"
CASE: Final = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"
EXTERNAL_STOP: Final = "pgid-watchdog-v3-confirmed-identity-absolute-deadline"


def _hex(seed: str, width: int) -> str:
    return hashlib.sha256(seed.encode("utf-8")).hexdigest()[:width]


def synthetic_profile(name: str) -> str:
    return f"{profile_prefix(name)}-w3-{_hex('profile' + name, 8)}"


def synthetic_identity(name: str) -> str:
    return ";".join((
        f"source={_hex('source' + name, 40)}", f"case={CASE}",
        f"profile={synthetic_profile(name)}", f"backend={BACKEND}",
        f"production_source={_hex('production' + name, 40)}",
        f"test_source={_hex('test' + name, 40)}", "method=cox-matthews", "retained=512",
        "force_samples=512", "endpoint=4096", "advective_limit=3.3",
        f"schedule={schedule_name(name)}", "resume=unsupported",
        f"observer_execution={OBSERVER_EXECUTION}", "host=baccus",
        f"external_stop={EXTERNAL_STOP}", f"schema={STATE_SCHEMA}",
    ))


def coefficient_hash(name: str, clock: int) -> str:
    return _hex(f"coefficient|{name}|{clock}", 64)


def file_hash(name: str, clock: int) -> str:
    return _hex(f"file|{name}|{clock}", 64)


def record_identity(name: str, plan: JsonObject) -> str:
    return str(branch_entries(plan)[name]["identity"])


def record_document(name: str, clock: int, identity: str) -> JsonObject:
    epoch = attempts_at(name, clock)
    return {
        "schema": STATE_SCHEMA, "identity": identity, "resumable": False,
        "clock": clock, "epoch": epoch, "accepted_steps": epoch,
        "coefficient_bytes": COEFFICIENT_BYTES,
        "state_sha256": coefficient_hash(name, clock),
        "observation_status": "CapturedActualState", "offline_observer_node": True,
        "observer_execution": OBSERVER_EXECUTION, "qualification": False,
    }


def concrete_family_plan() -> JsonObject:
    plan = emitted_family_plan()
    branches = plan["branches"]
    assert isinstance(branches, list)
    for raw in branches:
        assert isinstance(raw, dict)
        name = str(raw["name"])
        if name == "h64":
            continue
        raw.update({"identity": synthetic_identity(name), "profile": synthetic_profile(name),
                    "source_commit": _hex("source" + name, 40),
                    "execution": f"offline-baccus-observer-{name}",
                    "plan_sha256": _hex(f"plan|{name}", 64),
                    "plan_file": "capture-plan.json"})
    plan["arithmetic_files"] = {f"{coarse}--{fine}": f"arithmetic/{coarse}--{fine}.json"
                                for coarse, fine in PAIRS}
    return plan


def arithmetic_binding(coarse: str, fine: str, plan: JsonObject) -> JsonObject:
    def side(name: str) -> JsonObject:
        entry = branch_entries(plan)[name]
        return {"source_commit": entry["source_commit"], "backend": BACKEND,
                "execution": entry["execution"],
                "profile": {"kind": "identity-profile-field", "value": entry["profile"]}}

    evidence = f"{coarse}--{fine}-lineage.json"
    return {
        "schema": ARITHMETIC_SCHEMA, "pair": f"{coarse}--{fine}",
        "arithmetic_control": {
            "evidence": evidence, "evidence_sha256": "0" * 64,
            "review": {
                "schema": "p10-time-arithmetic-review-v1",
                "conclusion": "reviewed-equivalence-supported-by-controls",
                "case_sha256": CASE, "method": "cox-matthews",
                "integration_force_dimensions": [512, 512, 512],
                "measured_control": {
                    "outcome": "successful-exact-bit",
                    "serial": {"source_commit": _hex("serial", 40), "backend": BACKEND,
                               "execution": "serial-owner", "configuration": "serial-w1"},
                    "w3": {"source_commit": _hex("w3", 40), "backend": BACKEND,
                           "execution": "w3-owner", "configuration": "w3-w3"},
                },
                "reviewed_lineage": {"status": "reviewed-unchanged-kernel-lineage",
                                     "left_control_role": "serial",
                                     "right_control_role": "w3",
                                     "left": side(coarse), "right": side(fine)},
            },
        },
    }


class FamilyRoot:
    """A complete synthetic capture root plus a strictly valid family plan."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.plan = concrete_family_plan()
        self._write_plans()
        self._write_captures()
        self._write_inventory()
        self._write_arithmetic()

    def branch_list(self) -> list[dict[str, Json]]:
        branches = self.plan["branches"]
        assert isinstance(branches, list)
        return [branch for branch in branches if isinstance(branch, dict)]

    def _write_plans(self) -> None:
        for entry in self.branch_list():
            name = str(entry["name"])
            path = self.root / name / ("v3-launch-plan.json" if name == "h64"
                                       else "capture-plan.json")
            path.parent.mkdir(parents=True, exist_ok=True)
            raw = (V3_PLAN_BYTES if name == "h64"
                   else json.dumps({"capture-plan": name}).encode("utf-8"))
            path.write_bytes(raw)
            if name != "h64":
                entry["plan_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()

    def _write_captures(self) -> None:
        for name in BRANCH_ORDER:
            for clock in SHARED_CLOCKS:
                bundle = self.capture_path(name, clock)
                bundle.mkdir(parents=True, exist_ok=True)
                (bundle / "state.bin").write_bytes(b"")
                (bundle / "record.json").write_text(
                    json.dumps(record_document(name, clock,
                                               record_identity(name, self.plan)),
                               indent=2) + "\n", encoding="utf-8")

    def _write_inventory(self) -> None:
        files = {f"{name}/{bundle_name(name, clock)}/state.bin": file_hash(name, clock)
                 for name in BRANCH_ORDER for clock in SHARED_CLOCKS}
        (self.root / INVENTORY_NAME).write_text(
            json.dumps({"schema": INVENTORY_SCHEMA, "files": files}, indent=2) + "\n",
            encoding="utf-8")

    def _write_arithmetic(self) -> None:
        arithmetic = self.root / "arithmetic"
        arithmetic.mkdir(parents=True, exist_ok=True)
        for coarse, fine in PAIRS:
            key = f"{coarse}--{fine}"
            binding = arithmetic_binding(coarse, fine, self.plan)
            control = binding["arithmetic_control"]
            assert isinstance(control, dict)
            evidence = arithmetic / str(control["evidence"])
            evidence.write_text(json.dumps(control["review"], indent=2) + "\n", encoding="utf-8")
            control["evidence_sha256"] = hashlib.sha256(evidence.read_bytes()).hexdigest()
            (arithmetic / f"{key}.json").write_text(
                json.dumps(binding, indent=2) + "\n", encoding="utf-8")

    def capture_path(self, name: str, clock: int) -> Path:
        return self.root / name / bundle_name(name, clock)

    def rewrite_record(self, name: str, clock: int, /, **overrides: Json) -> None:
        path = self.capture_path(name, clock) / "record.json"
        record: JsonObject = dict(loads_checked(path.read_bytes(), "fixture record"))
        record.update(overrides)
        path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")

    def tampered(self, **overrides: Json) -> JsonObject:
        plan: JsonObject = dict(loads_checked(json.dumps(self.plan).encode("utf-8"),
                                             "fixture plan"))
        plan.update(overrides)
        return plan

    def run(self, output: Path, plan: JsonObject | None = None) -> int:
        path = self.root.parent / "plan-under-test.json"
        document = self.plan if plan is None else plan
        path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
        return main(["--family-plan", str(path), "--root", str(self.root),
                     "--output", str(output)])
