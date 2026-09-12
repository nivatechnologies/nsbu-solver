#!/usr/bin/env python3
"""Bounded serial/parallel LLVM coverage execution prototype.

This tool deliberately does not modify CI. It validates that direct execution of
Cargo's all-target-style artifacts can be sharded without changing the merged
coverage report. Doctests are outside this prototype because the hosted
workflow runs them separately.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import cast

from tools.json_types import Json, array_value, decode, object_value, string_value

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "work" / "coverage-shard-prototype"
TARGET = OUT / "target"
CARGO = ["cargo", "+nightly-2026-03-03"]
COV_ARGS = ["llvm-cov"]
REPORT_ARGS = [
    "--branch",
    "--no-default-ignore-filename-regex",
    "--ignore-filename-regex",
    r"(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)",
]
TARGETS = (
    ("fft", ["-p", "nsbu-solver", "--test", "fft"]),
    ("allocation", ["-p", "nsbu-solver", "--test", "allocation"]),
    ("force_cache_profile", ["-p", "nsbu-benchmarks", "--example", "force_cache_profile"]),
)


def command_env(tool_bin: Path | None) -> dict[str, str]:
    """Get cargo-llvm-cov's instrumentation environment from PATH or an override."""
    env = os.environ.copy()
    if tool_bin is not None:
        if not tool_bin.is_dir():
            raise ValueError(f"tool directory does not exist: {tool_bin}")
        if not (tool_bin / "cargo-llvm-cov").is_file():
            raise ValueError(f"tool directory lacks cargo-llvm-cov: {tool_bin}")
        env["PATH"] = f"{tool_bin}{os.pathsep}{env['PATH']}"
    env["CARGO_TARGET_DIR"] = str(TARGET)
    show = subprocess.run(
        CARGO + COV_ARGS + ["show-env", "--branch"], cwd=ROOT, env=env, check=True,
        text=True, capture_output=True,
    )
    result = env.copy()
    # cargo-llvm-cov emits shell-safe single-quoted assignments. Its only
    # embedded separator here is a literal unit-separator in Rust flags.
    for line in show.stdout.splitlines():
        match = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)=(?:'(.*)'|([^\s]+))", line)
        if match:
            result[match.group(1)] = match.group(2) if match.group(2) is not None else match.group(3)
    result["CARGO_TARGET_DIR"] = str(TARGET)
    result["CARGO_LLVM_COV_TARGET_DIR"] = str(TARGET)
    return result


def run(argv: list[str], *, env: dict[str, str], log: Path | None = None) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(argv, cwd=ROOT, env=env, text=True, capture_output=True)
    if log is not None:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text("$ " + " ".join(argv) + "\n\nstdout:\n" + completed.stdout + "\nstderr:\n" + completed.stderr)
    if completed.returncode:
        raise RuntimeError(f"command failed ({completed.returncode}): {' '.join(argv)}")
    return completed


def inventory() -> None:
    """Record Cargo's complete all-target candidate inventory without building it."""
    metadata = subprocess.run(
        CARGO + ["metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, check=True, text=True, capture_output=True,
    )
    executable_kinds = {"lib", "bin", "example", "test", "bench"}
    targets: list[dict[str, Json]] = []
    payload = object_value(decode(metadata.stdout))
    for package_value in array_value(payload["packages"]):
        package = object_value(package_value)
        package_name = string_value(package["name"])
        for target_value in array_value(package["targets"]):
            target = object_value(target_value)
            kinds = [string_value(kind) for kind in array_value(target["kind"])]
            test_enabled = target["test"]
            if not isinstance(test_enabled, bool):
                raise ValueError("Cargo target test field must be boolean")
            if executable_kinds.intersection(kinds):
                targets.append({
                    "package": package_name,
                    "name": string_value(target["name"]),
                    "kind": kinds,
                    "source": string_value(target["src_path"]),
                    "cargo_test_enabled": test_enabled,
                })
    document: dict[str, Json] = {
        "source": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "scope": "cargo metadata executable candidates for cargo test --workspace --all-targets; doctests excluded",
        "targets": targets,
    }
    (OUT / "all-target-inventory.json").write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")


def build(env: dict[str, str]) -> dict[str, str]:
    """Compile each representative target once and record Cargo artifact paths."""
    executable: dict[str, str] = {}
    for name, selector in TARGETS:
        output = run(CARGO + ["test"] + selector + ["--no-run", "--message-format=json"], env=env, log=OUT / "logs" / f"build-{name}.log")
        for line in output.stdout.splitlines():
            try:
                message = object_value(decode(line))
            except (json.JSONDecodeError, ValueError):
                continue
            executable_path = message.get("executable")
            target_value = message.get("target")
            if message.get("reason") == "compiler-artifact" and isinstance(executable_path, str) and target_value is not None:
                target = object_value(target_value)
                if target.get("name") == name:
                    executable[name] = executable_path
    absent = [name for name, _ in TARGETS if name not in executable]
    if absent:
        raise RuntimeError(f"Cargo did not emit representative artifacts: {absent}")
    manifest = {"source": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), "targets": executable}
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return executable


def clear_profiles() -> None:
    for path in TARGET.glob("*.profraw"):
        path.unlink()
    for path in TARGET.glob("*.profdata"):
        path.unlink()


def execute(executable: dict[str, str], mode: str, workers: int, env: dict[str, str]) -> None:
    """Run every manifest entry once, retaining status and isolated raw profiles."""
    clear_profiles()
    (OUT / "logs" / mode).mkdir(parents=True, exist_ok=True)
    def one(item: tuple[str, str]) -> tuple[str, int]:
        name, binary = item
        local = env.copy()
        local["LLVM_PROFILE_FILE"] = str(TARGET / f"{mode}-{name}-%m-%p.profraw")
        completed = subprocess.run([binary], cwd=ROOT, env=local, text=True, capture_output=True)
        (OUT / "logs" / mode / f"{name}.log").write_text(
            "$ " + binary + "\nexit=" + str(completed.returncode) + "\n\nstdout:\n" + completed.stdout + "\nstderr:\n" + completed.stderr
        )
        return name, completed.returncode
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        outcomes = list(pool.map(one, executable.items()))
    failures = {name: code for name, code in outcomes if code}
    if failures:
        raise RuntimeError(f"{mode} target failures: {failures}")
    profiles = sorted(path.name for path in TARGET.glob(f"{mode}-*.profraw"))
    if len(profiles) != len(executable):
        raise RuntimeError(f"{mode} wrote {len(profiles)} profiles for {len(executable)} executables: {profiles}")
    (OUT / f"{mode}-profiles.json").write_text(json.dumps(profiles, indent=2) + "\n")


def report(mode: str, env: dict[str, str]) -> Path:
    output = OUT / f"{mode}-coverage.json"
    run(CARGO + COV_ARGS + ["report", "--json", "--output-path", str(output)] + REPORT_ARGS, env=env, log=OUT / "logs" / f"report-{mode}.log")
    if not output.exists():
        raise RuntimeError(f"missing {output}")
    return output


def canonical_coverage(path: Path) -> dict[str, Json]:
    """Keep every summary and function record used for equality admission."""
    exported = object_value(decode(path.read_text()))
    entries = array_value(exported["data"])
    if len(entries) != 1:
        raise ValueError("LLVM export must contain one coverage data entry")
    data = object_value(entries[0])
    files: dict[str, Json] = {}
    for file_value in array_value(data.get("files", [])):
        file = object_value(file_value)
        files[string_value(file["filename"])] = file["summary"]
    return {
        "totals": data["totals"],
        "files": files,
        "functions": data.get("functions", []),
    }


def compare(serial: Path, parallel: Path) -> None:
    left = canonical_coverage(serial)
    right = canonical_coverage(parallel)
    (OUT / "comparison.json").write_text(json.dumps({"serial": left, "parallel": right}, indent=2, sort_keys=True) + "\n")
    if left != right:
        raise RuntimeError("serial and parallel merged coverage differ; see comparison.json")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument(
        "--tool-bin",
        type=Path,
        default=Path(os.environ["NSBU_RUST_TOOLS"]) if "NSBU_RUST_TOOLS" in os.environ else None,
        help="directory containing cargo-llvm-cov; defaults to PATH (or NSBU_RUST_TOOLS)",
    )
    arguments = parser.parse_args()
    workers = cast(int, arguments.workers)
    tool_bin = cast(Path | None, arguments.tool_bin)
    if not 1 <= workers <= 4:
        raise SystemExit("workers must be 1..4")
    OUT.mkdir(parents=True, exist_ok=True)
    inventory()
    env = command_env(tool_bin)
    executable = build(env)
    execute(executable, "serial", 1, env)
    serial = report("serial", env)
    execute(executable, "parallel", workers, env)
    parallel = report("parallel", env)
    compare(serial, parallel)
    print(f"coverage execution prototype passed: {serial} == {parallel}")
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"coverage execution prototype failed: {error}", file=sys.stderr)
        raise
