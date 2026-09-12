#!/usr/bin/env python3
"""Verify a locally extracted alpha archive and exercise its downloaded binary.

This script deliberately performs no download.  The extracted archive root,
the exact source checkout, and an output directory are supplied by the caller;
all command transcripts and derived results are written below that output
directory.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

SOURCE = "def4730b08025fdd06e7a8a0d78116aea24b6e2c"
TAG = "alpha-20260912"
SHA_LINE = re.compile(r"^([0-9a-f]{64}) [ *](.+)$")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest_value(text: str, key: str) -> str:
    values = [line.split("=", 1)[1] for line in text.splitlines()
              if line.startswith(key + "=")]
    if len(values) != 1:
        raise ValueError(f"manifest must contain exactly one {key}= entry")
    return values[0]


class Transcript:
    def __init__(self, out: Path) -> None:
        self.out = out
        self.records: list[dict[str, object]] = []

    def run(self, label: str, argv: list[str], cwd: Path | None = None,
            expect: int | None = 0) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(argv, cwd=cwd, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        record = {"label": label, "command": argv, "cwd": str(cwd or Path.cwd()),
                  "status": result.returncode, "stdout": result.stdout,
                  "stderr": result.stderr}
        self.records.append(record)
        (self.out / f"{len(self.records):02d}-{label}.stdout").write_text(result.stdout)
        (self.out / f"{len(self.records):02d}-{label}.stderr").write_text(result.stderr)
        if expect is not None and result.returncode != expect:
            raise RuntimeError(f"{label} exited {result.returncode}, expected {expect}")
        return result


def check_checksums(root: Path) -> set[str]:
    entries: dict[str, str] = {}
    for line in (root / "SHA256SUMS").read_text().splitlines():
        match = SHA_LINE.fullmatch(line)
        if not match:
            raise ValueError(f"invalid SHA256SUMS line: {line!r}")
        expected, name = match.groups()
        if name in entries:
            raise ValueError(f"duplicate SHA256SUMS entry: {name}")
        path = root / name
        if not path.is_file() or digest(path) != expected:
            raise ValueError(f"checksum mismatch: {name}")
        entries[name] = expected
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()}
    if actual != set(entries) | {"SHA256SUMS"}:
        raise ValueError("archive files and SHA256SUMS inventory differ")
    return set(entries)


def check_source(root: Path, repo: Path, source: str) -> list[str]:
    names = subprocess.run(["git", "-C", str(repo), "ls-tree", "-r", "-z",
                            "--name-only", source], check=True,
                           stdout=subprocess.PIPE).stdout.decode().split("\0")
    names = [name for name in names if name]
    for name in names:
        expected = subprocess.run(["git", "-C", str(repo), "show", f"{source}:{name}"],
                                  check=True, stdout=subprocess.PIPE).stdout
        actual_path = root / name
        if not actual_path.is_file() or actual_path.read_bytes() != expected:
            raise ValueError(f"source tree mismatch: {name}")
    return names


def report(stdout: str) -> dict[str, object]:
    value = json.loads(stdout)
    if not isinstance(value, dict):
        raise ValueError("diagnostic output is not a JSON object")
    return value


def completed_zero(stdout: str) -> None:
    value = report(stdout)
    if value.get("status") != "completed" or value.get("accepted_pde_windows") != 0:
        raise ValueError(f"unexpected diagnostic result: {value.get('status')}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("extracted_root", type=Path)
    parser.add_argument("--source-repo", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = args.extracted_root.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    transcript = Transcript(out)
    try:
        manifest = (root / "SOURCE-MANIFEST.txt").read_text()
        if manifest_value(manifest, "source_commit") != SOURCE:
            raise ValueError("SOURCE-MANIFEST source commit mismatch")
        if manifest_value(manifest, "snapshot_tag") != TAG:
            raise ValueError("SOURCE-MANIFEST snapshot tag mismatch")
        if manifest_value(manifest, "target") != "x86_64-unknown-linux-gnu":
            raise ValueError("SOURCE-MANIFEST target mismatch")
        source_names = check_source(root, args.source_repo.resolve(), SOURCE)
        listed = check_checksums(root)
        if listed != set(source_names) | {"bin/nsbu", "SOURCE-MANIFEST.txt"}:
            raise ValueError("SHA256SUMS differs from the exact source and release inventory")

        binary = root / "bin" / "nsbu"
        version = transcript.run("version", [str(binary), "--version"])
        if version.stdout.strip() != "NSBU Solver 0.1.0-alpha.1":
            raise ValueError("unexpected binary version")
        transcript.run("help", [str(binary), "--help"])
        commands = {
            "cm": [str(binary), "v2"],
            "ho": [str(binary), "v2", "--method", "ho"],
        }
        results: dict[str, dict[str, object]] = {}
        for label, command in commands.items():
            command_result = transcript.run(label, command)
            results[label] = report(command_result.stdout)
            completed_zero(command_result.stdout)

        checkpoint = out / "ho.checkpoint"
        saved = transcript.run("ho-checkpoint", [str(binary), "v2", "--method", "ho",
                                                   "--checkpoint", str(checkpoint),
                                                   "--checkpoint-after", "16"])
        saved_value = report(saved.stdout)
        if saved_value.get("status") != "checkpoint_saved" or not checkpoint.is_file():
            raise ValueError("HO checkpoint was not saved")
        resumed = transcript.run("ho-resume", [str(binary), "resume-v2", "--method", "ho",
                                                "--checkpoint", str(checkpoint)])
        resumed_value = report(resumed.stdout)
        if resumed_value.get("status") != "completed":
            raise ValueError("HO resume did not complete")
        left, right = dict(results["ho"]), dict(resumed_value)
        if left.pop("origin_status", None) != "internal_from_rest":
            raise ValueError("unexpected fresh HO origin status")
        if right.pop("origin_status", None) != "external_unverified" or left != right:
            raise ValueError("HO resume differs beyond origin_status")

        for label, method in (("cached-cm", "cm"), ("cached-ho", "ho")):
            record = transcript.run(label, [str(binary), "v2", "--cache-force",
                                            "--method", method])
            completed_zero(record.stdout)
        cached_checkpoint = out / "cached.checkpoint"
        refused = transcript.run("cached-checkpoint-refusal", [str(binary), "v2",
            "--cache-force", "--method", "cm", "--checkpoint", str(cached_checkpoint),
            "--checkpoint-after", "16"], expect=None)
        if refused.returncode == 0 or cached_checkpoint.exists():
            raise ValueError("cached checkpoint was not refused safely")
        if "cached_force_checkpoint_unsupported" not in refused.stdout:
            raise ValueError("cached checkpoint refusal omitted expected error code")
        (out / "manifest.txt").write_text(manifest)
        summary = {"source_commit": SOURCE, "snapshot_tag": TAG,
                   "source_files_compared": len(source_names),
                   "sha256sums_entries": len(listed),
                   "checkpoint_resume_equal_except_origin_status": True,
                   "cached_checkpoint_refused": True,
                   "accepted_pde_windows": 0, "commands": transcript.records}
        (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary, indent=2))
        return 0
    except Exception as error:
        (out / "failure.json").write_text(json.dumps({"error": str(error),
                                                        "commands": transcript.records}, indent=2) + "\n")
        print(f"verification failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
