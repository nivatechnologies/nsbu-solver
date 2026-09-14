#!/usr/bin/env python3
"""Builder for the unqualified N512 clock-1536 offline balance manifest.

Binds ONLY externally reviewed, already-durable inputs:
  * the clock-1536 captured-state record (identity, clock/epoch/steps, state hash),
  * the frozen v3 launch plan file (its hash must equal the frozen value and the
    trajectory plan hash it pins),
  * an externally reviewed full-file SHA-256 for the archived state file.

It never loads a snapshot, never computes a coefficient hash itself, and refuses
to invent any unknown hash. The full-file SHA may remain a clearly marked
all-zero placeholder ONLY with --unexecuted-placeholder, for the arithmetic-only
`preflight` path (which never opens the snapshot file); `run` mode would then
fail closed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
import sys
from pathlib import Path

HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")

# Frozen reviewed inputs (verified against the repository at preparation time).
EXPECTED_RECORD_SHA256 = "4fbb8c213a41a5e88eca4599953e3fa4cd5d4a2af8975abe6062b51a066d6376"
EXPECTED_PLAN_SHA256 = "4c14ee169cfbbb2a182d972633aba1292ed041878a9c4322e64a527f87d85da8"
EXPECTED_TRAJECTORY_PLAN_SHA256 = (
    "6e8103a1937e3e31be8b147a936b843d4dc166877ef5de5540fb51429e672634"
)
EXPECTED_IDENTITY_SOURCE = "e25f3816f83c6a7c07202cac2878f58ace460511"
EXPECTED_CASE_SHA256 = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"
EXPECTED_PROFILE = "n512-m512-h64to2048-h128to4096-cadv33-w3-9eba11f"
EXPECTED_STATE_SHA256 = "6501282b8224d19baf2cd68e201e5c6db7ffbf906c277d1a2ebb0495fbc3d509"

RECORD_SCHEMA = "p10-avx-n512-observer-state-v1"
PLAN_SCHEMA = "p10-n512-m512-v3-launch-plan-v1"
ELAPSED = 1536
TARGET = 8192
EPOCH = 24
ACCEPTED_STEPS = 24
ZERO_FILE_PLACEHOLDER = "0" * 64
# One bounded byte buffer per input: the SHA-256 and the JSON parse below are
# computed from the SAME bytes, so the verified hash cannot drift from the
# parsed content (no re-open between hash and parse).
MAX_INPUT_BYTES = 1 << 20


def fail(message: str) -> None:
    raise SystemExit(f"builder-refusal: {message}")


def publication_uncertain(operation: str, output: Path, error: OSError) -> None:
    """Post-link(2) durability failure: destination published, durability unproven.

    Prints an explicit publication-uncertainty report to stderr carrying the
    primary context (published destination and the failed operation) and the
    secondary context (OS errno and message), then exits with status 2,
    distinct from a clean pre-publication refusal (status 1). The published
    destination is preserved; the caller must verify its bytes before reuse.
    """
    print(
        f"builder-uncertainty: publication uncertain -- link(2) already "
        f"published {output} but {operation} failed afterwards "
        f"(errno={error.errno} {error.strerror}: {error}); "
        f"published destination {output} preserved -- verify its bytes "
        f"before reuse",
        file=sys.stderr,
    )
    raise SystemExit(2)


def read_bounded(path: Path, label: str) -> bytes:
    try:
        with path.open("rb") as handle:
            data = handle.read(MAX_INPUT_BYTES + 1)
    except OSError as error:
        fail(f"cannot read {label} {path}: {error}")
    if len(data) > MAX_INPUT_BYTES:
        fail(f"{label} exceeds the {MAX_INPUT_BYTES}-byte bounded-buffer cap")
    return data


def bind_hash_and_parse(path: Path, label: str, expected_sha256: str, mismatch: str) -> dict:
    data = read_bounded(path, label)
    if hashlib.sha256(data).hexdigest() != expected_sha256:
        fail(mismatch)
    try:
        return json.loads(data)
    except ValueError as error:
        fail(f"{label} is not valid JSON: {error}")


def identity_field(identity: str, key: str) -> str:
    values = [
        value
        for field in identity.split(";")
        if (split := field.split("=", 1))[0] == key and len(split) == 2
        for value in [split[1]]
    ]
    if len(values) != 1:
        fail(f"identity must carry exactly one {key}= field")
    return values[0]


def load_record(path: Path) -> dict:
    record = bind_hash_and_parse(
        path,
        "clock record",
        EXPECTED_RECORD_SHA256,
        "clock record is not the frozen reviewed clock1536-record.json bytes",
    )
    checks = {
        "schema": RECORD_SCHEMA,
        "clock": ELAPSED,
        "epoch": EPOCH,
        "accepted_steps": ACCEPTED_STEPS,
        "resumable": False,
        "qualification": False,
        "observation_status": "CapturedActualState",
        "offline_observer_node": True,
        "observer_execution": "offline-baccus-required",
        "coefficient_bytes": 3233808384,
    }
    for field, expected in checks.items():
        if record.get(field) != expected:
            fail(f"record field {field!r} is {record.get(field)!r}, expected {expected!r}")
    if record.get("state_sha256") != EXPECTED_STATE_SHA256:
        fail("record state_sha256 differs from the reviewed captured-state hash")
    identity = record.get("identity")
    if not isinstance(identity, str):
        fail("record carries no identity string")
    for key, expected in {
        "source": EXPECTED_IDENTITY_SOURCE,
        "case": EXPECTED_CASE_SHA256,
        "profile": EXPECTED_PROFILE,
        "endpoint": "4096",
        "retained": "512",
        "force_samples": "512",
        "resume": "unsupported",
        "schema": RECORD_SCHEMA,
    }.items():
        if identity_field(identity, key) != expected:
            fail(f"identity field {key} differs from the reviewed captured state")
    return record


def load_plan(path: Path) -> dict:
    plan = bind_hash_and_parse(
        path,
        "plan",
        EXPECTED_PLAN_SHA256,
        "plan file is not the frozen v3 launch plan bytes (SHA-256 mismatch)",
    )
    if plan.get("schema") != PLAN_SCHEMA:
        fail("plan schema mismatch")
    if plan.get("harness_commit_and_run_source") != EXPECTED_IDENTITY_SOURCE:
        fail("plan run source differs from the captured-state identity source")
    historical = plan.get("historical_preparation", {})
    if historical.get("immutable_trajectory_plan_sha256") != EXPECTED_TRAJECTORY_PLAN_SHA256:
        fail("plan does not pin the frozen trajectory plan SHA-256 6e8103a1...")
    if plan.get("launch_authorized") is not False:
        fail("plan is not marked launch_authorized=false")
    return plan


def build(record: dict, plan_relative: str, plan_sha256: str, snapshot: str,
          file_sha256: str) -> dict:
    identity = record["identity"]
    evolution = {
        "case_sha256": EXPECTED_CASE_SHA256,
        "quantum_exponent": -20,
        "clock_target": TARGET,
        "comparison_endpoint": ELAPSED,
        "lengths": [1.0, 1.0, 1.0],
        "viscosity": 1.0,
        "method": "cox-matthews",
        "integration_force_dimensions": [512, 512, 512],
        "schedule": [
            {"from_inclusive": 0, "until_exclusive": ELAPSED, "step_ticks": 64}
        ],
        "absolute_tolerances": [1e-05, 0.0001],
        "relative_tolerances": [1e-05, 1e-05],
    }
    execution = ";".join(
        f"{key}={identity_field(identity, key)}"
        for key in (
            "provider",
            "rhs_w3",
            "force_w3",
            "sampling_workers",
            "rhs_w3_workers",
            "provider_w3_workers",
            "host",
        )
    )
    return {
        "schema": "p10-snapshot-comparison-input-v1",
        "comparison_kind": "FORCE_RESOLUTION_DIAGNOSTIC",
        "snapshot": snapshot,
        "plan": plan_relative,
        "identity": identity,
        "source_commit": EXPECTED_IDENTITY_SOURCE,
        "plan_sha256": plan_sha256,
        "coefficient_sha256": record["state_sha256"],
        "file_sha256": file_sha256,
        "backend": identity_field(identity, "backend"),
        "execution": execution,
        "dimensions": [512, 512, 512],
        "evolution": evolution,
        "elapsed": ELAPSED,
        "target": TARGET,
        "epoch": EPOCH,
        "accepted_steps": ACCEPTED_STEPS,
        "profile": {"kind": "identity-profile-field", "value": EXPECTED_PROFILE},
        "admission_guard": {"advective_limit": 3.3, "maximum_attempts": 48},
    }


def publish_new(output: Path, text: str, input_paths: tuple[Path, ...]) -> None:
    """Create-only atomic publication, rejecting aliases with all inputs.

    Any pre-existing path (regular file, hard link or symbolic link, live or
    broken) is refused, so replay and publication races resolve to exactly one
    winner: the content is written to a process-unique temporary file in the
    destination directory, fsynced, then attached with link(2), which fails
    atomically with EEXIST for every existing alias. Once link(2) has created
    the destination, a directory fsync/close or temporary-cleanup failure
    reports explicit publication uncertainty (exit status 2) while preserving
    the published destination; pre-publication failures remain clean
    refusals (exit status 1).
    """
    resolved_inputs = {path.resolve() for path in input_paths}
    if len(resolved_inputs) != len(input_paths) or os.path.samefile(*input_paths):
        fail("builder inputs alias each other")
    if output.resolve() in resolved_inputs:
        fail("output aliases a builder input")
    if os.path.lexists(output):
        fail(f"output path already exists (create-only publication; no overwrite): {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        dir=output.parent, prefix=f".{output.name}.", suffix=f".tmp-{os.getpid()}"
    )
    published = False
    try:
        with os.fdopen(descriptor, "w") as handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, output)
        except FileExistsError:
            fail(f"output path already exists (create-only publication; no overwrite): {output}")
        published = True
        try:
            directory_fd = os.open(output.parent, os.O_RDONLY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
        except OSError as error:
            raise publication_uncertain("directory fsync/close", output, error) from error
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        except OSError as error:
            if not published:
                raise
            raise publication_uncertain("temporary-file cleanup", output, error) from error


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True,
                        help="frozen v3 launch plan file (hash-verified here)")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--file-sha256", help="externally reviewed archived-file SHA-256")
    parser.add_argument("--snapshot", default="PENDING_ROOT_ARCHIVE_BINDING/state.bin",
                        help="archived state path placed in the manifest (root-provided)")
    parser.add_argument("--unexecuted-placeholder", action="store_true",
                        help="allow the all-zero file-SHA placeholder for arithmetic-only preflight")
    arguments = parser.parse_args()

    record = load_record(arguments.record)
    load_plan(arguments.plan)

    if arguments.file_sha256 is None:
        if not arguments.unexecuted_placeholder:
            fail("an externally reviewed full-file SHA-256 is required "
                 "(pass --unexecuted-placeholder only for arithmetic-only preflight)")
        if arguments.snapshot != "PENDING_ROOT_ARCHIVE_BINDING/state.bin":
            fail("placeholder mode must not name a snapshot path")
        file_sha256 = ZERO_FILE_PLACEHOLDER
    else:
        if not HEX64.fullmatch(arguments.file_sha256):
            fail("file SHA-256 must be 64 lowercase hex characters")
        if arguments.file_sha256 == ZERO_FILE_PLACEHOLDER:
            fail("all-zero file SHA-256 is only allowed with --unexecuted-placeholder")
        file_sha256 = arguments.file_sha256

    manifest = build(
        record,
        os.path.relpath(arguments.plan.resolve(), arguments.output.resolve().parent),
        EXPECTED_PLAN_SHA256,
        arguments.snapshot,
        file_sha256,
    )
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    publish_new(
        arguments.output,
        json.dumps(manifest, indent=2) + "\n",
        (arguments.record, arguments.plan),
    )
    print(f"wrote {arguments.output}")
    print(f"manifest-coefficient-binding={record['state_sha256']}")
    print(f"manifest-file-binding={file_sha256}"
          + (" (UNEXECUTED PLACEHOLDER)" if file_sha256 == ZERO_FILE_PLACEHOLDER else ""))


if __name__ == "__main__":
    main()
