"""CLI, streaming-IO and publication behaviour tests for snapshot preparation."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import stat
import types
from typing import BinaryIO, Callable, cast

import pytest
from pytest import CaptureFixture, MonkeyPatch

from snapshot_manifest_fixtures import (
    IDENTITY, MAGIC, Inputs, baseline, build_state, launch_document, payload_bytes, plan_bound,
    template_document, trajectory_document,
)
from tools import json_types
from tools import prepare_snapshot_manifest as psm
from tools.json_types import Json, JsonObject, object_value
from tools.prepare_snapshot_manifest import (
    PreparationError, PublicationUncertainError, publish_create_only,
)


class Recording:
    """Counts bytes actually read through a wrapped file handle."""

    def __init__(self, path: str, handle: BinaryIO, reads: dict[str, int]) -> None:
        self._path, self._handle, self._reads = path, handle, reads

    def read(self, size: int = -1) -> bytes:
        data = self._handle.read(size)
        self._reads[self._path] = self._reads.get(self._path, 0) + len(data)
        return data

    def __enter__(self) -> "Recording":
        return self

    def __exit__(self, exc_type: type[BaseException] | None,
                 exc_value: BaseException | None,
                 traceback: types.TracebackType | None) -> object:
        return self._handle.__exit__(exc_type, exc_value, traceback)


def recording_open(reads: dict[str, int], real_open: object) -> Callable[..., Recording]:
    opener = cast(Callable[..., BinaryIO], real_open)

    def open_file(file: object, *args: object, **kwargs: object) -> Recording:
        return Recording(str(file), opener(file, *args, **kwargs), reads)

    return open_file


def prepared_manifest(path: Path) -> Json:
    return json_types.decode(path.read_text())


def test_prepares_baseline_manifest_with_filled_bindings(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    assert inputs.run(expected_file, expected_plan) == 0
    prepared = object_value(prepared_manifest(inputs.output))
    assert prepared["snapshot"] == "state.bin"
    assert prepared["plan"] == "v3-launch-plan.json"
    assert prepared["file_sha256"] == expected_file
    assert prepared["coefficient_sha256"] == hashlib.sha256(payload_bytes()).hexdigest()
    assert prepared["identity"] == IDENTITY and prepared["elapsed"] == 4096
    assert prepared["plan_sha256"] == expected_plan


def test_changed_payload_refuses_stale_expected_file_hash(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    changed = build_state(payload=payload_bytes(seed=1))
    inputs.write_state(changed)
    inputs.write_record(payload_bytes(seed=1))
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_changed_header_clock_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.write_state(build_state(clock=3072))
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_intermediate_record_clock_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.write_record(payload_bytes(), clock=3072)
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_changed_record_hash_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.write_record(payload_bytes(), state_sha256="f" * 64)
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_wrong_expected_plan_hash_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, _ = baseline(tmp_path)
    assert inputs.run(expected_file, "0" * 64) == 1
    assert not inputs.output.exists()


def test_unbound_trajectory_hash_is_refused(tmp_path: Path) -> None:
    trajectory_hash = "2" * 64
    inputs, expected_file, expected_plan = plan_bound(
        tmp_path, launch_document(trajectory_hash[:-1] + "0"))
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_wrong_launch_schema_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = plan_bound(
        tmp_path, launch_document("2" * 64, schema="p10-n512-m512-v2-launch-plan-v1"))
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_truncated_and_trailing_snapshots_are_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    intact = inputs.state_path.read_bytes()
    for tampered in (intact[:-16], intact + b"\x00"):
        inputs.write_state(tampered)
        assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_conflicting_nonplaceholder_template_binding_is_refused(tmp_path: Path) -> None:
    inputs = Inputs(tmp_path)
    expected_plan = inputs.write_plan()
    inputs.write_template(expected_plan, coefficient_sha256="f" * 64)
    expected_file = inputs.write_state(build_state())
    inputs.write_record(payload_bytes())
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_conflicting_template_identity_is_refused(tmp_path: Path) -> None:
    inputs = Inputs(tmp_path)
    expected_plan = inputs.write_plan()
    inputs.write_template(expected_plan, identity=IDENTITY + ";extra=field")
    expected_file = inputs.write_state(build_state())
    inputs.write_record(payload_bytes())
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_duplicate_record_keys_are_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    text = inputs.record_path.read_text()
    inputs.record_path.write_text(text.replace('"epoch": 48', '"epoch": 48, "epoch": 48', 1))
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_non_finite_template_numbers_are_refused(tmp_path: Path) -> None:
    inputs = Inputs(tmp_path)
    expected_plan = inputs.write_plan()
    template = template_document(expected_plan)
    evolution: JsonObject = dict(object_value(template["evolution"]))
    evolution["viscosity"] = -1e400
    template["evolution"] = evolution
    inputs.template_path.write_text(json.dumps(template))
    expected_file = inputs.write_state(build_state())
    inputs.write_record(payload_bytes())
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_non_finite_trajectory_numbers_are_refused(tmp_path: Path) -> None:
    inputs = Inputs(tmp_path)
    trajectory = trajectory_document()
    profile: JsonObject = dict(object_value(trajectory["profile"]))
    profile["advective_limit"] = float("nan")
    trajectory["profile"] = profile
    raw = json.dumps(trajectory).encode("utf-8")
    inputs.trajectory_path.write_bytes(raw)
    inputs.write_plan(launch_document(hashlib.sha256(raw).hexdigest()))
    inputs.bind_template_to_plan()
    expected_file = inputs.write_state(build_state())
    inputs.write_record(payload_bytes())
    assert inputs.run(expected_file, hashlib.sha256(inputs.plan_path.read_bytes()).hexdigest()) == 1
    assert not inputs.output.exists()


def test_resumable_or_qualifying_record_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.write_record(payload_bytes(), resumable=True)
    assert inputs.run(expected_file, expected_plan) == 1
    inputs.write_record(payload_bytes(), qualification=True)
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_magic_mismatch_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    intact = inputs.state_path.read_bytes()
    inputs.write_state(b"P10AVXSNAP2\0" + intact[12:])
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_load_json_bounded_accepts_exact_bound_and_refuses_one_more(tmp_path: Path) -> None:
    document = tmp_path / "document.json"
    document.write_bytes(b'{"a": 1}')
    bound = len(document.read_bytes())
    assert psm.load_json_bounded(document, bound, "document") == {"a": 1}
    with pytest.raises(PreparationError, match="exceeds"):
        psm.load_json_bounded(document, bound - 1, "document")


def test_oversized_plan_is_refused_by_bounded_read(tmp_path: Path, monkeypatch: MonkeyPatch,
                                                   capsys: CaptureFixture[str]) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    monkeypatch.setattr(psm, "MAX_PLAN_BYTES", 4096)
    inputs.plan_path.write_bytes(b"{" + b" " * (256 * 1024))
    reads: dict[str, int] = {}
    monkeypatch.setattr("builtins.open", recording_open(reads, open))

    def unbounded_read_bytes(*args: object, **kwargs: object) -> bytes:
        raise AssertionError("the tool must not allocate the whole file with Path.read_bytes")

    monkeypatch.setattr(Path, "read_bytes", unbounded_read_bytes)
    assert inputs.run(expected_file, expected_plan) == 1
    assert "plan exceeds 4096 byte bound" in capsys.readouterr().err
    assert reads[str(inputs.plan_path)] <= 4097
    assert not inputs.output.exists()


def test_oversized_trajectory_is_refused_by_bounded_read(tmp_path: Path, monkeypatch: MonkeyPatch,
                                                        capsys: CaptureFixture[str]) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    monkeypatch.setattr(psm, "MAX_PLAN_BYTES", 4096)
    inputs.trajectory_path.write_bytes(b"{" + b" " * (256 * 1024))
    reads: dict[str, int] = {}
    monkeypatch.setattr("builtins.open", recording_open(reads, open))
    assert inputs.run(expected_file, expected_plan) == 1
    assert "trajectory plan exceeds 4096 byte bound" in capsys.readouterr().err
    assert reads[str(inputs.trajectory_path)] <= 4097
    assert not inputs.output.exists()


def test_trajectory_plan_is_read_exactly_once(tmp_path: Path, monkeypatch: MonkeyPatch) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    size = inputs.trajectory_path.stat().st_size
    reads: dict[str, int] = {}
    monkeypatch.setattr("builtins.open", recording_open(reads, open))
    assert inputs.run(expected_file, expected_plan) == 0
    assert reads[str(inputs.trajectory_path)] == size


def test_existing_output_manifest_is_preserved(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.output.write_text("reviewed original bytes\n")
    assert inputs.run(expected_file, expected_plan) == 1
    assert inputs.output.read_text() == "reviewed original bytes\n"
    assert not list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_link_failure_with_existing_destination_is_uncertain_and_retained(
        tmp_path: Path, monkeypatch: MonkeyPatch) -> None:
    target = tmp_path / "comparison-input.json"

    def racing_link(source: str | os.PathLike[str],
                    destination: str | os.PathLike[str]) -> None:
        Path(destination).write_text("previous result bytes\n")
        raise OSError(5, "mocked link I/O failure")

    monkeypatch.setattr(psm.os, "link", racing_link)
    with pytest.raises(PublicationUncertainError, match="retained"):
        publish_create_only(target, b"prepared payload\n")
    assert target.read_text() == "previous result bytes\n"
    assert list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_link_failure_without_destination_refuses_cleanly(tmp_path: Path,
                                                         monkeypatch: MonkeyPatch) -> None:
    target = tmp_path / "comparison-input.json"

    def failing_link(*args: object, **kwargs: object) -> None:
        raise OSError(5, "mocked link I/O failure")

    monkeypatch.setattr(psm.os, "link", failing_link)
    with pytest.raises(PreparationError) as refused:
        publish_create_only(target, b"prepared payload\n")
    assert not isinstance(refused.value, PublicationUncertainError)
    assert not target.exists()
    assert not list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_unlink_failure_after_link_is_uncertain_and_retained(tmp_path: Path,
                                                             monkeypatch: MonkeyPatch,
                                                             capsys: CaptureFixture[str]) -> None:
    real_unlink = os.unlink

    def failing_unlink(path: str | os.PathLike[str]) -> None:
        if Path(str(path)).name.startswith(".comparison-input.json.partial."):
            raise OSError(5, "mocked staged-copy unlink failure")
        real_unlink(path)

    monkeypatch.setattr(psm.os, "unlink", failing_unlink)
    inputs, expected_file, expected_plan = baseline(tmp_path)
    assert inputs.run(expected_file, expected_plan) == 2
    assert "publication uncertain" in capsys.readouterr().err
    assert object_value(prepared_manifest(inputs.output))["snapshot"] == "state.bin"
    assert list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_directory_open_failure_after_link_is_uncertain_and_retained(
        tmp_path: Path, monkeypatch: MonkeyPatch, capsys: CaptureFixture[str]) -> None:
    real_open = os.open

    def failing_directory_open(path: str | os.PathLike[str], flags: int,
                               mode: int = 0o644) -> int:
        if not flags & os.O_CREAT:
            raise OSError(5, "mocked directory open failure")
        return real_open(path, flags, mode)

    monkeypatch.setattr(psm.os, "open", failing_directory_open)
    inputs, expected_file, expected_plan = baseline(tmp_path)
    assert inputs.run(expected_file, expected_plan) == 2
    assert "publication uncertain" in capsys.readouterr().err
    assert object_value(prepared_manifest(inputs.output))["snapshot"] == "state.bin"
    assert not list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_directory_fsync_failure_reports_uncertain_publication(tmp_path: Path,
                                                               monkeypatch: MonkeyPatch,
                                                               capsys: CaptureFixture[str]) -> None:
    import stat as stat_module
    real_fsync = os.fsync

    def failing_directory_fsync(fd: int) -> None:
        if stat_module.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(5, "mocked directory fsync failure")
        real_fsync(fd)

    monkeypatch.setattr(psm.os, "fsync", failing_directory_fsync)
    inputs, expected_file, expected_plan = baseline(tmp_path)
    assert inputs.run(expected_file, expected_plan) == 2
    assert "publication uncertain" in capsys.readouterr().err
    assert object_value(prepared_manifest(inputs.output))["snapshot"] == "state.bin"
    assert not list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_directory_close_failure_reports_uncertain_publication(tmp_path: Path,
                                                               monkeypatch: MonkeyPatch,
                                                               capsys: CaptureFixture[str]) -> None:
    import stat as stat_module
    real_close = os.close

    def failing_directory_close(fd: int) -> None:
        if stat_module.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(5, "mocked directory close failure")
        real_close(fd)

    monkeypatch.setattr(psm.os, "close", failing_directory_close)
    inputs, expected_file, expected_plan = baseline(tmp_path)
    assert inputs.run(expected_file, expected_plan) == 2
    error = capsys.readouterr().err
    assert "publication uncertain" in error and "could not be closed" in error
    assert object_value(prepared_manifest(inputs.output))["snapshot"] == "state.bin"


def test_directory_fsync_and_close_failures_keep_both_context(tmp_path: Path,
                                                              monkeypatch: MonkeyPatch,
                                                              capsys: CaptureFixture[str]) -> None:
    import stat as stat_module
    real_fsync, real_close = os.fsync, os.close

    def failing_fsync(fd: int) -> None:
        if stat_module.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(5, "mocked directory fsync failure")
        real_fsync(fd)

    def failing_close(fd: int) -> None:
        if stat_module.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(5, "mocked directory close failure")
        real_close(fd)

    monkeypatch.setattr(psm.os, "fsync", failing_fsync)
    monkeypatch.setattr(psm.os, "close", failing_close)
    inputs, expected_file, expected_plan = baseline(tmp_path)
    assert inputs.run(expected_file, expected_plan) == 2
    error = capsys.readouterr().err
    assert "publication uncertain" in error
    assert "fsync failed" in error and "close failed" in error
    assert object_value(prepared_manifest(inputs.output))["snapshot"] == "state.bin"


def state_with(length_field: bytes | None = None, identity_override: bytes | None = None) -> bytes:
    identity = IDENTITY.encode("utf-8")
    identity_bytes = identity if identity_override is None else identity_override
    assert len(identity_bytes) == len(identity)
    body = MAGIC + (length_field or len(identity).to_bytes(8, "little")) + identity_bytes
    for word in (4096, 8192, 48, 48):
        body += word.to_bytes(16, "little")
    coefficients = payload_bytes()
    return body + coefficients + hashlib.sha256(coefficients).digest()


def test_state_identity_length_mismatch_is_refused(tmp_path: Path) -> None:
    inputs, _, expected_plan = baseline(tmp_path)
    inputs.write_state(state_with(length_field=(len(IDENTITY) + 8).to_bytes(8, "little")))
    assert inputs.run("0" * 64, expected_plan) == 1
    assert not inputs.output.exists()


def test_state_identity_bytes_mismatch_is_refused(tmp_path: Path) -> None:
    inputs, _, expected_plan = baseline(tmp_path)
    foreign = IDENTITY.replace("profile=test-profile", "profile=tesu-profile").encode("utf-8")
    inputs.write_state(state_with(identity_override=foreign))
    assert inputs.run("0" * 64, expected_plan) == 1
    assert not inputs.output.exists()


def test_state_coefficient_trailer_mismatch_is_refused(tmp_path: Path) -> None:
    inputs, _, expected_plan = baseline(tmp_path)
    intact = build_state()
    corrupted = intact[:-32] + b"\x11" * 32
    inputs.write_state(corrupted)
    assert inputs.run(hashlib.sha256(corrupted).hexdigest(), expected_plan) == 1
    assert not inputs.output.exists()


def test_conflicting_plan_binding_name_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.write_template(expected_plan, plan="other-name.json")
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_plan_not_beside_output_is_refused(tmp_path: Path) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    inputs.output = tmp_path / "subdir" / "comparison-input.json"
    inputs.output.parent.mkdir()
    assert inputs.run(expected_file, expected_plan) == 1
    assert not inputs.output.exists()


def test_link_appearing_during_publication_is_refused(tmp_path: Path,
                                                     monkeypatch: MonkeyPatch) -> None:
    def existing_link(*args: object, **kwargs: object) -> None:
        raise FileExistsError(17, "mocked race")

    monkeypatch.setattr(psm.os, "link", existing_link)
    target = tmp_path / "comparison-input.json"
    with pytest.raises(PreparationError, match="appeared during publication") as refused:
        publish_create_only(target, b"prepared payload\n")
    assert not isinstance(refused.value, PublicationUncertainError)
    assert not list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_stage_write_failure_refuses_cleanly(tmp_path: Path, monkeypatch: MonkeyPatch) -> None:
    real_fsync = os.fsync

    def failing_file_fsync(fd: int) -> None:
        if not stat.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(5, "mocked staged-file fsync failure")
        real_fsync(fd)

    monkeypatch.setattr(psm.os, "fsync", failing_file_fsync)
    target = tmp_path / "comparison-input.json"
    with pytest.raises(PreparationError, match="could not be staged"):
        publish_create_only(target, b"prepared payload\n")
    assert not target.exists()
    assert not list(tmp_path.glob(".comparison-input.json.partial.*"))


def test_multi_kibibyte_deep_template_is_a_one_line_refusal(tmp_path: Path,
                                                            capsys: CaptureFixture[str]) -> None:
    inputs, expected_file, expected_plan = baseline(tmp_path)
    deep = b'{"nest": ' + b"[" * 1100 + b"]" * 1100 + b"}"
    inputs.template_path.write_bytes(deep)
    assert inputs.run(expected_file, expected_plan) == 1
    error = capsys.readouterr().err.strip().splitlines()
    assert len(error) == 1 and error[0].startswith("refused:") and "nests deeper" in error[0]
    assert not inputs.output.exists()


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
