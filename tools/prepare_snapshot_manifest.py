"""Prepare N512 endpoint comparison input manifests from captured artifacts.

This binds a captured endpoint bundle and the reviewed v3 launch plan into an
explicitly supplied reviewed comparison-input template. The launch plan is
admitted against its closed schema and the reviewed trajectory capture plan is
required explicitly and hash-checked against the launch plan's
historical_preparation.immutable_trajectory_plan_sha256. It prepares bindings;
it never accepts a numerical result, blesses a candidate, infers settings or
admits intermediate clocks. Standard library only.
"""

from __future__ import annotations

import argparse
from collections.abc import Mapping, Sequence
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import BinaryIO, Final, TypeVar

if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.json_types import Json, JsonObject
from tools.prepare_snapshot_manifest_contract import (
    PENDING_PLAN, PENDING_SNAPSHOT, PLACEHOLDER_HASHES,
    endpoint_comparison_template, validate_template, verify_capture_plan, verify_launch_plan,
    verify_record_document,
)
from tools.prepare_snapshot_manifest_validation import (
    MAX_IDENTITY_BYTES, MAX_MANIFEST_BYTES, MAX_PLAN_BYTES, MAX_RECORD_BYTES, PreparationError,
    PublicationUncertainError, count_value, grid_value, hex_value, load_json_bounded,
    loads_checked, read_capped, string_value,
)

MAGIC: Final = b"P10AVXSNAP1\0"
TRAILER_BYTES: Final = 32
HEADER_WORDS: Final = 4
COEFFICIENT_BYTES_TOTAL: Final = 3 * 16
IO_BUFFER_BYTES: Final = 1024 * 1024


def coefficient_bytes_for(dimensions: Sequence[int]) -> int:
    plane = dimensions[0] * dimensions[1]
    return plane * (dimensions[2] // 2 + 1) * COEFFICIENT_BYTES_TOTAL


def _read_exact(handle: BinaryIO, size: int, label: str) -> bytes:
    data = handle.read(size)
    if len(data) != size:
        raise PreparationError(f"{label} is truncated")
    return data


def stream_state(path: Path, identity: str, expected_coefficient_bytes: int) -> tuple[int, int, int, int, str, str]:
    identity_bytes = identity.encode("utf-8")
    expected_file_bytes = (len(MAGIC) + 8 + len(identity_bytes) + HEADER_WORDS * 16
                           + expected_coefficient_bytes + TRAILER_BYTES)
    if path.stat().st_size != expected_file_bytes:
        raise PreparationError("snapshot length mismatch")
    whole = hashlib.sha256()
    coefficient = hashlib.sha256()
    with open(path, "rb") as handle:
        if _read_exact(handle, len(MAGIC), "snapshot") != MAGIC:
            raise PreparationError("snapshot magic mismatch")
        whole.update(MAGIC)
        header = _read_exact(handle, 8 + len(identity_bytes) + HEADER_WORDS * 16, "snapshot header")
        whole.update(header)
        identity_len = int.from_bytes(header[:8], "little")
        if identity_len != len(identity_bytes) or identity_len > MAX_IDENTITY_BYTES:
            raise PreparationError("snapshot identity length mismatch")
        if header[8 : 8 + len(identity_bytes)] != identity_bytes:
            raise PreparationError("snapshot identity mismatch")
        start = 8 + len(identity_bytes)
        words = [int.from_bytes(header[start + 16 * i : start + 16 * (i + 1)], "little")
                 for i in range(HEADER_WORDS)]
        remaining = expected_coefficient_bytes
        while remaining:
            chunk = _read_exact(handle, min(IO_BUFFER_BYTES, remaining), "snapshot coefficients")
            remaining -= len(chunk)
            whole.update(chunk)
            coefficient.update(chunk)
        trailer = _read_exact(handle, TRAILER_BYTES, "snapshot trailer")
        if handle.read(1) != b"":
            raise PreparationError("snapshot trailing bytes")
    coefficient_hash = coefficient.hexdigest()
    if trailer != bytes.fromhex(coefficient_hash):
        raise PreparationError("coefficient SHA-256 trailer mismatch")
    whole.update(trailer)
    return words[0], words[1], words[2], words[3], coefficient_hash, whole.hexdigest()


def verify_clock(header: Sequence[int], template: Mapping[str, Json]) -> None:
    expected = tuple(count_value(template[name], f"template {name}")
                     for name in ("elapsed", "target", "epoch", "accepted_steps"))
    if tuple(header) != expected:
        raise PreparationError("snapshot clock header does not match the reviewed template")


def bind_artifacts(template: Mapping[str, Json], state: Path, plan: Path,
                   output: Path,
                   coefficient_hash: str, file_hash: str, expected_file: str,
                   expected_plan: str) -> JsonObject:
    directory = output.parent
    prepared = dict(template)
    for name, source, pending in (("snapshot", state, PENDING_SNAPSHOT), ("plan", plan, PENDING_PLAN)):
        current = string_value(prepared[name], f"template {name}")
        if current != pending and current != source.name:
            raise PreparationError(f"template {name} binding conflicts with the captured artifact")
        if (directory / source.name).resolve() != source.resolve():
            raise PreparationError(f"{name} is not the artifact resolved beside the output manifest")
        prepared[name] = source.name
    if string_value(prepared["plan_sha256"], "template plan_sha256").lower() != expected_plan:
        raise PreparationError("template plan_sha256 conflicts with the expected plan hash")
    for name, computed in (("coefficient_sha256", coefficient_hash), ("file_sha256", file_hash)):
        current = string_value(prepared[name], f"template {name}")
        if current not in (PLACEHOLDER_HASHES[name], computed):
            raise PreparationError(f"template {name} binding conflicts with the computed hash")
        prepared[name] = computed
    if file_hash != expected_file:
        raise PreparationError("computed file SHA-256 does not match the expected file hash")
    return prepared


def prepare(template_path: Path, bundle: Path, plan_path: Path, trajectory_path: Path,
            expected_file: str, expected_plan: str, output: Path) -> JsonObject:
    expected_file = hex_value(expected_file, 64, "expected file SHA256")
    expected_plan = hex_value(expected_plan, 64, "expected plan SHA256")
    template = validate_template(load_json_bounded(template_path, MAX_MANIFEST_BYTES,
                                                    "comparison template"))
    with open(plan_path, "rb") as handle:
        plan_bytes = read_capped(handle, MAX_PLAN_BYTES)
    if len(plan_bytes) > MAX_PLAN_BYTES:
        raise PreparationError(f"plan exceeds {MAX_PLAN_BYTES} byte bound")
    if hashlib.sha256(plan_bytes).hexdigest() != expected_plan:
        raise PreparationError("computed plan SHA-256 does not match the expected plan hash")
    trajectory_hash = verify_launch_plan(loads_checked(plan_bytes, "v3 launch plan"), template)
    with open(trajectory_path, "rb") as handle:
        trajectory_bytes = read_capped(handle, MAX_PLAN_BYTES)
    if len(trajectory_bytes) > MAX_PLAN_BYTES:
        raise PreparationError(f"trajectory plan exceeds {MAX_PLAN_BYTES} byte bound")
    if hashlib.sha256(trajectory_bytes).hexdigest() != trajectory_hash:
        raise PreparationError("trajectory plan hash does not match the launch plan's "
                               "immutable_trajectory_plan_sha256")
    observer_execution = verify_capture_plan(loads_checked(trajectory_bytes,
                                                           "trajectory capture plan"), template)
    coefficient_bytes = coefficient_bytes_for(grid_value(template["dimensions"], "template dimensions"))
    clock, target, epoch, accepted, coefficient_hash, file_hash = stream_state(
        bundle / "state.bin", string_value(template["identity"], "template identity"),
        coefficient_bytes)
    verify_clock((clock, target, epoch, accepted), template)
    record = load_json_bounded(bundle / "record.json", MAX_RECORD_BYTES, "captured record")
    verify_record_document(record, template, coefficient_bytes, coefficient_hash, observer_execution)
    return bind_artifacts(template, bundle / "state.bin", plan_path, output,
                          coefficient_hash, file_hash, expected_file, expected_plan)


def publish_create_only(path: Path, payload: bytes) -> None:
    if len(payload) > MAX_MANIFEST_BYTES:
        raise PreparationError("prepared manifest exceeds the 64 KiB input bound")
    temporary = path.parent / f".{path.name}.partial.{os.getpid()}"
    if path.exists() or temporary.exists():
        raise PreparationError("output manifest already exists; refusing to overwrite")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except OSError as error:
        if temporary.exists():
            os.unlink(temporary)
        raise PreparationError(f"prepared manifest could not be staged: {error}") from error
    try:
        os.link(temporary, path)
    except FileExistsError as error:
        os.unlink(temporary)
        raise PreparationError("output manifest appeared during publication") from error
    except OSError as error:
        if path.exists():
            raise PublicationUncertainError(
                f"linking the output manifest failed with {error}; the destination already exists "
                f"and both it and the staged copy at {temporary} were retained") from error
        os.unlink(temporary)
        raise PreparationError(f"prepared manifest could not be linked into place: {error}") from error
    # From here the destination is linked; every fallible step must report an
    # uncertain publication while preserving the destination.
    try:
        staged = temporary.exists()
        if staged:
            os.unlink(temporary)
    except OSError as error:
        raise PublicationUncertainError(
            f"the output manifest is linked at {path} but its staged copy at {temporary} could not "
            f"be removed with {error}; the staged copy is retained alongside the destination") from error
    try:
        directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    except OSError as error:
        raise PublicationUncertainError(
            f"the output manifest is linked at {path} but its directory could not be opened with "
            f"{error}; the staged copy was removed and the destination is retained") from error
    fsync_error: OSError | None = None
    try:
        os.fsync(directory_fd)
    except OSError as error:
        fsync_error = error
    close_error: OSError | None = None
    try:
        os.close(directory_fd)
    except OSError as error:
        close_error = error
    if fsync_error is not None and close_error is not None:
        raise PublicationUncertainError(
            f"the output manifest is linked at {path} but its directory fsync failed with "
            f"{fsync_error} and its close failed with {close_error}; publication is retained and "
            "its durability is uncertain") from fsync_error
    if fsync_error is not None:
        raise PublicationUncertainError(
            f"the output manifest is linked at {path} but its directory fsync failed with "
            f"{fsync_error}; publication is retained and its durability is uncertain") from fsync_error
    if close_error is not None:
        raise PublicationUncertainError(
            f"the output manifest is linked at {path} but its directory descriptor could not be "
            f"closed with {close_error}; publication is retained and its durability is uncertain"
        ) from close_error


def emit_endpoint_template(output: Path) -> None:
    template = validate_template(endpoint_comparison_template())
    publish_create_only(output, (json.dumps(template, indent=2) + "\n").encode("utf-8"))


_T = TypeVar("_T")


def _required(value: _T | None, name: str) -> _T:
    if value is None:
        raise PreparationError(f"missing required argument: {name}")
    return value


class Arguments(argparse.Namespace):
    template: Path | None
    bundle: Path | None
    plan: Path | None
    trajectory_plan: Path | None
    expected_file_sha256: str | None
    expected_plan_sha256: str | None
    output: Path
    emit_endpoint_template: bool


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--template", type=Path, help="reviewed comparison input manifest template")
    parser.add_argument("--bundle", type=Path, help="captured step bundle holding state.bin and record.json")
    parser.add_argument("--plan", type=Path,
                        help="reviewed v3 launch plan json bound by the template plan hash")
    parser.add_argument("--trajectory-plan", type=Path,
                        help="reviewed trajectory capture plan json named by the launch plan")
    parser.add_argument("--expected-file-sha256", help="external expected whole state.bin SHA-256")
    parser.add_argument("--expected-plan-sha256", help="external expected whole launch plan SHA-256")
    parser.add_argument("--output", type=Path, required=True, help="create-only output path")
    parser.add_argument("--emit-endpoint-template", action="store_true",
                        help="write the reviewed endpoint comparison template and exit")
    args = Arguments()
    parser.parse_args(argv, namespace=args)
    try:
        if args.emit_endpoint_template:
            emit_endpoint_template(args.output)
        else:
            prepared = prepare(_required(args.template, "--template"),
                               _required(args.bundle, "--bundle"),
                               _required(args.plan, "--plan"),
                               _required(args.trajectory_plan, "--trajectory-plan"),
                               _required(args.expected_file_sha256, "--expected-file-sha256"),
                               _required(args.expected_plan_sha256, "--expected-plan-sha256"),
                               args.output)
            publish_create_only(args.output, (json.dumps(prepared, indent=2) + "\n").encode("utf-8"))
    except PublicationUncertainError as error:
        print(f"publication uncertain: {error}", file=sys.stderr)
        return 2
    except (ValueError, OSError) as error:
        # PreparationError is a ValueError; json_types shape refusals are plain
        # ValueError. Either way a bad input is one refused line, never a traceback.
        print(f"refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
