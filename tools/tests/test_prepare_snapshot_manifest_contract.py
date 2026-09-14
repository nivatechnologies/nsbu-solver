"""Closed-contract tests: actual v3 plan bytes, record refusal, endpoint template.

These use real repository inputs read-only and one small independent state
fixture; they prove admission and refusal algebra, not capture execution.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess

import pytest

from snapshot_manifest_fixtures import (
    CLOCK1536_SHA256, REPO, TRAJECTORY_PLAN, V3_PLAN, clock1536_payload,
)
from tools import json_types
from tools.json_types import JsonObject, array_value, object_value
from tools.prepare_snapshot_manifest import PreparationError, main
from tools.prepare_snapshot_manifest_contract import (
    ENDPOINT_ADVECTIVE_LIMIT, ENDPOINT_IDENTITY, ENDPOINT_MAX_ATTEMPTS, ENDPOINT_N512_SOURCE,
    ENDPOINT_PLAN_SHA256, ENDPOINT_PROFILE, ENDPOINT_TRAJECTORY_PLAN_SHA256,
    endpoint_comparison_template, validate_template, verify_capture_plan, verify_launch_plan,
    verify_record_document,
)
from tools.prepare_snapshot_manifest_validation import loads_checked

DECODER_BINARY = (REPO / "evidence/p10/snapshot-comparison-adapter/harness/target/debug/"
                  "p10-snapshot-comparison-adapter")
DECODER_MANIFEST = "v3-launch-plan.json"
NO_DECODER = "decoder binary not built; cargo build the snapshot-comparison-adapter harness"


def actual_launch() -> JsonObject:
    return dict(loads_checked(V3_PLAN.read_bytes(), "v3 launch plan"))


def test_actual_v3_plan_binds_actual_trajectory_plan() -> None:
    plan_bytes = V3_PLAN.read_bytes()
    assert hashlib.sha256(plan_bytes).hexdigest() == ENDPOINT_PLAN_SHA256
    template = validate_template(endpoint_comparison_template())
    assert verify_launch_plan(loads_checked(plan_bytes, "v3 launch plan"),
                              template) == ENDPOINT_TRAJECTORY_PLAN_SHA256
    trajectory_bytes = TRAJECTORY_PLAN.read_bytes()
    assert hashlib.sha256(trajectory_bytes).hexdigest() == ENDPOINT_TRAJECTORY_PLAN_SHA256
    observer = verify_capture_plan(loads_checked(trajectory_bytes, "trajectory plan"), template)
    assert observer == "offline-baccus-required"


def test_actual_launch_plan_authorization_tamper_is_refused() -> None:
    template = validate_template(endpoint_comparison_template())
    launch = actual_launch()
    launch["launch_authorized"] = True
    with pytest.raises(PreparationError, match="unauthorized launch plan"):
        verify_launch_plan(launch, template)


def test_actual_launch_plan_executed_attempts_tamper_is_refused() -> None:
    template = validate_template(endpoint_comparison_template())
    launch = actual_launch()
    launch["endpoint_attempts_executed"] = 1
    with pytest.raises(PreparationError, match="executed endpoint attempts"):
        verify_launch_plan(launch, template)


def test_actual_launch_plan_qualification_tamper_is_refused() -> None:
    template = validate_template(endpoint_comparison_template())
    launch = actual_launch()
    guards: JsonObject = dict(object_value(launch["guards"]))
    guards["qualification"] = True
    launch["guards"] = guards
    with pytest.raises(PreparationError, match="qualification"):
        verify_launch_plan(launch, template)


def test_clock1536_record_is_incomplete_for_endpoint() -> None:
    template = validate_template(endpoint_comparison_template())
    payload = clock1536_payload()
    assert hashlib.sha256(payload).hexdigest() == CLOCK1536_SHA256
    record = loads_checked(payload, "clock1536 record")
    assert record["clock"] == 1536 and record["accepted_steps"] == 24
    with pytest.raises(PreparationError, match="frozen comparison endpoint"):
        verify_record_document(record, template, 3233808384, "a" * 64, "offline-baccus-required")


def test_endpoint_template_matches_closed_contract() -> None:
    template = validate_template(endpoint_comparison_template())
    assert template["target"] == 8192 and template["elapsed"] == 4096
    assert template["epoch"] == 48 and template["accepted_steps"] == 48
    assert template["plan_sha256"] == ENDPOINT_PLAN_SHA256
    assert template["identity"] == ENDPOINT_IDENTITY
    assert template["source_commit"] == ENDPOINT_N512_SOURCE
    assert object_value(template["profile"])["value"] == ENDPOINT_PROFILE
    assert object_value(template["profile"])["kind"] == "identity-profile-field"
    assert object_value(template["admission_guard"]) == {
        "advective_limit": ENDPOINT_ADVECTIVE_LIMIT, "maximum_attempts": ENDPOINT_MAX_ATTEMPTS}
    evolution = object_value(template["evolution"])
    assert len(array_value(evolution["absolute_tolerances"])) == 2
    assert len(array_value(evolution["relative_tolerances"])) == 2
    assert evolution["integration_force_dimensions"] == [512, 512, 512]
    assert evolution["schedule"] == [{"from_inclusive": 0, "until_exclusive": 2048,
                                      "step_ticks": 64},
                                     {"from_inclusive": 2048, "until_exclusive": 4096,
                                      "step_ticks": 128}]


def test_endpoint_template_is_decoder_field_compatible() -> None:
    template = endpoint_comparison_template()
    required = {"schema", "snapshot", "plan", "identity", "source_commit", "plan_sha256",
                "coefficient_sha256", "file_sha256", "backend", "execution", "dimensions",
                "evolution", "elapsed", "target", "epoch", "accepted_steps"}
    optional = {"comparison_kind", "profile", "admission_guard", "arithmetic_control"}
    assert required <= set(template) <= required | optional
    assert set(object_value(template["evolution"])) == {
        "case_sha256", "quantum_exponent", "clock_target", "comparison_endpoint", "lengths",
        "viscosity", "method", "integration_force_dimensions", "schedule",
        "absolute_tolerances", "relative_tolerances"}


def test_endpoint_template_hash_placeholders_remain() -> None:
    template = endpoint_comparison_template()
    assert template["coefficient_sha256"] == "PENDING_COEFFICIENT_SHA256"
    assert template["file_sha256"] == "PENDING_FILE_SHA256"
    assert template["snapshot"] == "PENDING_SNAPSHOT_BINDING"
    assert template["plan"] == "PENDING_PLAN_BINDING"


def test_emit_endpoint_template_writes_reviewed_template(tmp_path: Path) -> None:
    output = tmp_path / "endpoint-template.json"
    assert main(["--emit-endpoint-template", "--output", str(output)]) == 0
    assert json_types.decode(output.read_text()) == validate_template(
        endpoint_comparison_template())
    assert main(["--emit-endpoint-template", "--output", str(output)]) == 1


def build_schema_only_manifest(directory: Path) -> Path:
    """Schema-only manifest: dummy hex for unknown artifact hashes, real reviewed plan."""
    template = endpoint_comparison_template()
    template["coefficient_sha256"] = "a" * 64
    template["file_sha256"] = "b" * 64
    template["snapshot"] = "endpoint-state-placeholder.bin"
    template["plan"] = DECODER_MANIFEST
    (directory / DECODER_MANIFEST).write_bytes(V3_PLAN.read_bytes())
    manifest = directory / "endpoint-schema-only.json"
    manifest.write_text(json.dumps(template))
    return manifest


def tamper_manifest(manifest: Path) -> None:
    broken: JsonObject = dict(object_value(json_types.decode(manifest.read_text())))
    evolution: JsonObject = dict(object_value(broken["evolution"]))
    evolution["clock_target"] = 4096
    broken["evolution"] = evolution
    manifest.write_text(json.dumps(broken))


def test_endpoint_template_passes_rust_decoder_manifest_only_admission(tmp_path: Path) -> None:
    if not DECODER_BINARY.exists():
        pytest.skip(NO_DECODER)
    manifest = build_schema_only_manifest(tmp_path)
    assert hashlib.sha256((tmp_path / DECODER_MANIFEST).read_bytes()).hexdigest() \
        == ENDPOINT_PLAN_SHA256
    result = subprocess.run([str(DECODER_BINARY), str(manifest), str(manifest), "0"],
                            capture_output=True, text=True, timeout=60)
    assert result.returncode != 0, "the payload is absent; full comparison cannot pass"
    for decode_stage in ("invalid comparison manifest binding", "invalid evolution",
                         "invalid comparison tolerance", "invalid schedule",
                         "frozen plan SHA-256", "manifest exceeds", "plan exceeds"):
        assert decode_stage not in result.stderr
    # Both read_manifest calls (left and right are this same template) admitted
    # the manifest, so the first refusal is the later pair-stage contract check.
    assert "contract mismatch" in result.stderr


def test_decoder_probe_detects_broken_plan_hash(tmp_path: Path) -> None:
    if not DECODER_BINARY.exists():
        pytest.skip(NO_DECODER)
    manifest = build_schema_only_manifest(tmp_path)
    broken: JsonObject = dict(object_value(json_types.decode(manifest.read_text())))
    broken["plan_sha256"] = "0" * 64
    manifest.write_text(json.dumps(broken))
    result = subprocess.run([str(DECODER_BINARY), str(manifest), str(manifest), "0"],
                            capture_output=True, text=True, timeout=60)
    assert "frozen plan SHA-256 mismatch" in result.stderr


def test_decoder_probe_detects_broken_evolution(tmp_path: Path) -> None:
    if not DECODER_BINARY.exists():
        pytest.skip(NO_DECODER)
    manifest = build_schema_only_manifest(tmp_path)
    tamper_manifest(manifest)
    result = subprocess.run([str(DECODER_BINARY), str(manifest), str(manifest), "0"],
                            capture_output=True, text=True, timeout=60)
    assert "invalid evolution semantics" in result.stderr


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
