"""Focused controls for the coverage-sharding runner's orchestration boundaries."""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch
from typing import cast

import coverage_shard_prototype as prototype
from tools.json_types import array_value, decode, object_value, string_value


def coverage_document(branches: int = 1) -> dict[str, object]:
    """Produce a minimal LLVM export retaining every compared shape."""
    return {
        "data": [{
            "totals": {"lines": {"count": 2, "covered": 2}, "branches": {"count": 1, "covered": branches}},
            "files": [{"filename": "/source.rs", "summary": {"lines": {"count": 2, "covered": 2}}}],
            "functions": [{"name": "function", "count": 1, "filenames": ["/source.rs"], "regions": []}],
        }],
    }


def write_document(path: Path, document: dict[str, object]) -> None:
    path.write_text(json.dumps(document))


def test_command_environment_uses_validated_override_and_parses_both_assignment_forms() -> None:
    with tempfile.TemporaryDirectory() as directory:
        tool_bin = Path(directory)
        wrapper = tool_bin / "cargo-llvm-cov"
        wrapper.touch()
        output = "\n".join((
            "LLVM_PROFILE_FILE='/profiles/%p.profraw'",
            f"RUSTC_WRAPPER={wrapper}",
            "CARGO_LLVM_COV=1",
        ))
        completed = subprocess.CompletedProcess("command", 0, output, "")
        with patch.object(prototype.subprocess, "run", return_value=completed) as run:
            environment = prototype.command_env(tool_bin)
        self_path = environment["PATH"].split(":", 1)[0]
        assert self_path == str(tool_bin)
        assert environment["LLVM_PROFILE_FILE"] == "/profiles/%p.profraw"
        assert environment["RUSTC_WRAPPER"] == str(wrapper)
        assert environment["CARGO_LLVM_COV"] == "1"
        assert run.call_args.args[0][-2:] == ["show-env", "--branch"]


def test_command_environment_refuses_missing_or_incomplete_tool_directory() -> None:
    with tempfile.TemporaryDirectory() as directory:
        missing = Path(directory) / "missing"
        try:
            prototype.command_env(missing)
        except ValueError as error:
            assert "does not exist" in str(error)
        else:
            raise AssertionError("accepted a missing tool directory")
        try:
            prototype.command_env(Path(directory))
        except ValueError as error:
            assert "lacks cargo-llvm-cov" in str(error)
        else:
            raise AssertionError("accepted a directory without cargo-llvm-cov")


def test_inventory_retains_only_executable_cargo_target_kinds() -> None:
    metadata = {
        "packages": [{"name": "example", "targets": [
            {"name": "unit", "kind": ["test"], "src_path": "/unit.rs", "test": True},
            {"name": "profile", "kind": ["example"], "src_path": "/profile.rs", "test": False},
            {"name": "build", "kind": ["custom-build"], "src_path": "/build.rs", "test": False},
        ]}],
    }
    completed = subprocess.CompletedProcess("command", 0, json.dumps(metadata), "")
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory)
        with patch.object(prototype, "OUT", output), patch.object(prototype.subprocess, "run", return_value=completed), patch.object(prototype.subprocess, "check_output", return_value="source\n"):
            prototype.inventory()
        document = object_value(decode((output / "all-target-inventory.json").read_text()))
    targets = [object_value(target) for target in array_value(document["targets"])]
    assert [string_value(target["name"]) for target in targets] == ["unit", "profile"]
    assert string_value(document["source"]) == "source"


def test_compare_requires_equal_file_function_and_branch_payloads() -> None:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory)
        serial = output / "serial.json"
        parallel = output / "parallel.json"
        write_document(serial, coverage_document())
        write_document(parallel, coverage_document())
        with patch.object(prototype, "OUT", output):
            prototype.compare(serial, parallel)
            compared = object_value(decode((output / "comparison.json").read_text()))
            assert compared["serial"] == compared["parallel"]
            write_document(parallel, coverage_document(branches=0))
            try:
                prototype.compare(serial, parallel)
            except RuntimeError as error:
                assert "serial and parallel" in str(error)
            else:
                raise AssertionError("accepted mismatched branch payloads")


def test_build_records_each_expected_artifact_and_refuses_missing_artifact() -> None:
    artifact = {
        "reason": "compiler-artifact",
        "executable": "/binary",
        "target": {"name": "fft"},
    }
    completed = subprocess.CompletedProcess("command", 0, json.dumps(artifact), "")
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory)
        with patch.object(prototype, "OUT", output), patch.object(prototype, "TARGETS", (("fft", ["--test", "fft"]),)), patch.object(prototype, "run", return_value=completed), patch.object(prototype.subprocess, "check_output", return_value="source\n"):
            assert prototype.build({}) == {"fft": "/binary"}
        manifest = object_value(decode((output / "manifest.json").read_text()))
        assert string_value(manifest["source"]) == "source"
        with patch.object(prototype, "OUT", output), patch.object(prototype, "TARGETS", (("fft", ["--test", "fft"]),)), patch.object(prototype, "run", return_value=subprocess.CompletedProcess("command", 0, "{}", "")):
            try:
                prototype.build({})
            except RuntimeError as error:
                assert "did not emit" in str(error)
            else:
                raise AssertionError("accepted a missing compiler artifact")


def test_execute_writes_distinct_profiles_and_refuses_missing_profile() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        target = root / "target"
        target.mkdir()
        output = root / "output"
        def successful(binary: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
            environment = cast(dict[str, str], kwargs["env"])
            profile = Path(environment["LLVM_PROFILE_FILE"].replace("%m", "module").replace("%p", "process"))
            profile.touch()
            return subprocess.CompletedProcess(binary, 0, "", "")
        with patch.object(prototype, "OUT", output), patch.object(prototype, "TARGET", target), patch.object(prototype.subprocess, "run", side_effect=successful):
            prototype.execute({"one": "/one", "two": "/two"}, "parallel", 2, {})
        profiles = array_value(decode((output / "parallel-profiles.json").read_text()))
        assert len(profiles) == 2
        with patch.object(prototype, "OUT", output), patch.object(prototype, "TARGET", target), patch.object(prototype.subprocess, "run", return_value=subprocess.CompletedProcess("binary", 0, "", "")):
            try:
                prototype.execute({"one": "/one"}, "serial", 1, {})
            except RuntimeError as error:
                assert "wrote 0 profiles" in str(error)
            else:
                raise AssertionError("accepted a run with no raw profile")


def test_run_and_report_preserve_failures_and_written_output() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        log = root / "log.txt"
        with patch.object(prototype.subprocess, "run", return_value=subprocess.CompletedProcess("command", 0, "out", "err")):
            assert prototype.run(["command"], env={}, log=log).stdout == "out"
        assert "stdout:\nout" in log.read_text()
        with patch.object(prototype.subprocess, "run", return_value=subprocess.CompletedProcess("command", 7, "", "failed")):
            try:
                prototype.run(["command"], env={})
            except RuntimeError as error:
                assert "(7)" in str(error)
            else:
                raise AssertionError("accepted a failed subprocess")
        report = root / "serial-coverage.json"
        def write_report(*args: object, **kwargs: object) -> subprocess.CompletedProcess[str]:
            report.write_text(json.dumps(coverage_document()))
            return subprocess.CompletedProcess("report", 0, "", "")
        with patch.object(prototype, "OUT", root), patch.object(prototype, "run", side_effect=write_report):
            assert prototype.report("serial", {}) == report
