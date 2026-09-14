"""Shared synthetic fixtures for the snapshot manifest preparation test suite."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import struct
from typing import Final

from tools.json_types import Json, JsonObject
from tools.prepare_snapshot_manifest import main
from tools.prepare_snapshot_manifest_contract import ENDPOINT_IDENTITY, LAUNCH_EXTERNAL_STOP

REPO = Path(__file__).resolve().parents[2]
V3_PLAN = REPO / "evidence/p10/n512-m512-endpoint-prep-20260913/proposed-launch/v3-launch-plan.json"
TRAJECTORY_PLAN = REPO / "evidence/p10/n512-m512-endpoint-prep-20260913/frozen-plan.json"
CLOCK1536_SHA256: Final = "4fbb8c213a41a5e88eca4599953e3fa4cd5d4a2af8975abe6062b51a066d6376"


def clock1536_record() -> JsonObject:
    """The actual clock1536 early-capture record in its reviewed field order.

    Embedded as a self-contained literal (the ``work/`` tree is gitignored) so a
    fresh checkout reproduces the incomplete-record refusal; ``clock1536_payload``
    is byte-identical to the reviewed record and pinned by ``CLOCK1536_SHA256``.
    """
    return {
        "schema": "p10-avx-n512-observer-state-v1",
        "identity": ENDPOINT_IDENTITY,
        "resumable": False,
        "clock": 1536,
        "epoch": 24,
        "accepted_steps": 24,
        "coefficient_bytes": 3233808384,
        "state_sha256": "6501282b8224d19baf2cd68e201e5c6db7ffbf906c277d1a2ebb0495fbc3d509",
        "observation_status": "CapturedActualState",
        "offline_observer_node": True,
        "observer_execution": "offline-baccus-required",
        "qualification": False,
    }


def clock1536_payload() -> bytes:
    return (json.dumps(clock1536_record(), indent=2) + "\n").encode("utf-8")

MAGIC = b"P10AVXSNAP1\0"
SOURCE = "a" * 40
TEST_SOURCE = "b" * 40
PROD_SOURCE = "c" * 40
IDENTITY = (f"source={SOURCE};schema=p10-avx-n512-observer-state-v1;profile=test-profile;"
            f"test_source={TEST_SOURCE};production_source={PROD_SOURCE};"
            f"external_stop={LAUNCH_EXTERNAL_STOP};advective_limit=3.3")
CLOCK, TARGET, EPOCH, STEPS = 4096, 8192, 48, 48
CASE = "e" * 64
HEX64 = "1" * 64


def payload_bytes(seed: int = 0) -> bytes:
    return b"".join(struct.pack("<dd", (i + seed) / 4.0, -(i + seed) / 8.0) for i in range(24))


def build_state(identity: str = IDENTITY, clock: int = CLOCK, payload: bytes | None = None) -> bytes:
    body = MAGIC + len(identity).to_bytes(8, "little") + identity.encode("utf-8")
    for word in (clock, TARGET, EPOCH, STEPS):
        body += word.to_bytes(16, "little")
    coefficients = payload_bytes() if payload is None else payload
    return body + coefficients + hashlib.sha256(coefficients).digest()


def record_document(payload: bytes, **overrides: Json) -> JsonObject:
    record: JsonObject = {
        "schema": "p10-avx-n512-observer-state-v1", "identity": IDENTITY, "resumable": False,
        "clock": CLOCK, "epoch": EPOCH, "accepted_steps": STEPS, "coefficient_bytes": 384,
        "state_sha256": hashlib.sha256(payload).hexdigest(),
        "observation_status": "CapturedActualState", "offline_observer_node": True,
        "observer_execution": "offline-baccus-required", "qualification": False,
    }
    record.update(overrides)
    return record


def trajectory_document(**profile_overrides: Json) -> JsonObject:
    profile: JsonObject = {
        "clock_target": TARGET, "endpoint": CLOCK, "method": "cox-matthews",
        "analytical_reset": False, "absolute_tolerances": [1e-5, 1e-4],
        "relative_tolerances": [1e-5, 1e-5], "maximum_attempts": 48, "advective_limit": 3.3,
    }
    profile.update(profile_overrides)
    return {
        "schema": "p10-n512-m512-endpoint-capture-plan-v1", "qualification": False,
        "case_sha256": CASE, "profile": profile,
        "captures": {"resume": "unsupported", "create_new": True,
                     "positive_offline_observer_clocks": [512, 1024, 4096]},
        "resource_plan": {"offline_observer_host": "baccus"},
    }


def launch_document(trajectory_hash: str, **overrides: Json) -> JsonObject:
    plan: JsonObject = {
        "schema": "p10-n512-m512-v3-launch-plan-v1",
        "status": "prepared_unexecuted_future_authorization_and_absolute_deadline_required",
        "numerical_test_source_commit": TEST_SOURCE, "production_source_commit": PROD_SOURCE,
        "harness_commit_and_run_source": SOURCE, "feature": "n512-m512-piecewise-cadv33",
        "profile": "test-profile", "external_stop": LAUNCH_EXTERNAL_STOP,
        "binary_sha256": HEX64, "preflight_sha256": HEX64, "watchdog_sha256": HEX64,
        "local_archive_helper_sha256": HEX64,
        "historical_preparation": {"immutable_trajectory_plan_sha256": trajectory_hash,
                                   "v2_binary_sha256": HEX64,
                                   "preparation_results": "../preparation-results.json"},
        "deadline": {"source": "caller_supplied_future_absolute_epoch",
                     "minimum_remaining_at_launch_seconds": 66672,
                     "minimum_remaining_after_first_step_seconds": 65410},
        "resources": {"memory_floor_bytes": 241937824240, "source_disk_floor_bytes": 189586276352,
                      "local_archive_disk_floor_bytes": 189586276352,
                      "address_space_limit_bytes": 274877906944},
        "guards": {"explicit_opt_in": True,
                   "stable_process_identity_before_group_ownership": True,
                   "term_grace_then_kill_seconds": 60,
                   "solver_and_local_archive_share_absolute_deadline": True,
                   "first_step": "clock64;rhs12;hits7;misses5;steady_allocations0;integration<=1200",
                   "committed_states": STEPS, "offline_observer_nodes": 8, "qualification": False},
        "archive_scope": ("local_create_new_partial_copy_inventory_compare_sync_rename; "
                          "no remote durable transfer is claimed"),
        "endpoint_attempts_executed": 0, "launch_authorized": False,
    }
    plan.update(overrides)
    return plan


def template_document(plan_hash: str, **overrides: Json) -> JsonObject:
    template: JsonObject = {
        "schema": "p10-snapshot-comparison-input-v1",
        "comparison_kind": "MATCHED_M512_SPATIAL_DIAGNOSTIC",
        "snapshot": "PENDING_SNAPSHOT_BINDING", "plan": "PENDING_PLAN_BINDING",
        "identity": IDENTITY, "source_commit": SOURCE, "plan_sha256": plan_hash,
        "coefficient_sha256": "PENDING_COEFFICIENT_SHA256", "file_sha256": "PENDING_FILE_SHA256",
        "backend": "rustfft-6.4.1-avx-avx2-fma", "execution": "offline-baccus-observer",
        "dimensions": [2, 2, 2],
        "evolution": {
            "case_sha256": CASE, "quantum_exponent": -20, "clock_target": TARGET,
            "comparison_endpoint": CLOCK, "lengths": [1.0, 1.0, 1.0], "viscosity": 0.01,
            "method": "cox-matthews", "integration_force_dimensions": [2, 2, 2],
            "schedule": [{"from_inclusive": 0, "until_exclusive": 2048, "step_ticks": 64},
                         {"from_inclusive": 2048, "until_exclusive": 4096, "step_ticks": 128}],
            "absolute_tolerances": [1e-5, 1e-4], "relative_tolerances": [1e-5, 1e-5],
        },
        "elapsed": CLOCK, "target": TARGET, "epoch": EPOCH, "accepted_steps": STEPS,
        "admission_guard": {"advective_limit": 3.3, "maximum_attempts": 48},
    }
    template.update(overrides)
    return template


class Inputs:
    def __init__(self, directory: Path) -> None:
        self.directory = directory
        self.bundle = directory
        self.output = directory / "comparison-input.json"
        self.template_path = directory / "template.json"
        self.plan_path = directory / "v3-launch-plan.json"
        self.trajectory_path = directory / "frozen-plan.json"
        self.state_path = directory / "state.bin"
        self.record_path = directory / "record.json"

    def write_trajectory(self, plan: JsonObject | None = None) -> str:
        raw = json.dumps(plan if plan is not None else trajectory_document()).encode("utf-8")
        self.trajectory_path.write_bytes(raw)
        return hashlib.sha256(raw).hexdigest()

    def write_plan(self, launch: JsonObject | None = None) -> str:
        trajectory_hash = self.write_trajectory()
        launch = launch if launch is not None else launch_document(trajectory_hash)
        raw = json.dumps(launch).encode("utf-8")
        self.plan_path.write_bytes(raw)
        return hashlib.sha256(raw).hexdigest()

    def bind_template_to_plan(self) -> str:
        plan_hash = hashlib.sha256(self.plan_path.read_bytes()).hexdigest()
        self.write_template(plan_hash)
        return plan_hash

    def write_template(self, plan_hash: str, **overrides: Json) -> None:
        self.template_path.write_text(json.dumps(template_document(plan_hash, **overrides)))

    def write_state(self, state: bytes) -> str:
        self.state_path.write_bytes(state)
        return hashlib.sha256(state).hexdigest()

    def write_record(self, payload: bytes, **overrides: Json) -> None:
        self.record_path.write_text(json.dumps(record_document(payload, **overrides)))

    def run(self, expected_file: str, expected_plan: str) -> int:
        return main(["--template", str(self.template_path), "--bundle", str(self.bundle),
                     "--plan", str(self.plan_path), "--trajectory-plan", str(self.trajectory_path),
                     "--expected-file-sha256", expected_file,
                     "--expected-plan-sha256", expected_plan, "--output", str(self.output)])


def baseline(tmp_path: Path) -> tuple[Inputs, str, str]:
    inputs = Inputs(tmp_path)
    expected_plan = inputs.write_plan()
    inputs.write_template(expected_plan)
    expected_file = inputs.write_state(build_state())
    inputs.write_record(payload_bytes())
    return inputs, expected_file, expected_plan


def plan_bound(tmp_path: Path, launch: JsonObject) -> tuple[Inputs, str, str]:
    inputs = Inputs(tmp_path)
    expected_plan = inputs.write_plan(launch)
    inputs.bind_template_to_plan()
    expected_file = inputs.write_state(build_state())
    inputs.write_record(payload_bytes())
    return inputs, expected_file, expected_plan
