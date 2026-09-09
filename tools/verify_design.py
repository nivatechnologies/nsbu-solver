"""Run the preserved NSBU design checks with immutable-input and assertion guards.

No Rust compilation, PDE integration or source admission is performed. A fresh
report adds execution metadata without modifying the original review evidence.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile

if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.check_repository import check_repository, sha256
from typing import cast
from tools.json_types import JsonObject, decode, object_value


UNPERFORMED = ["Rust compilation", "PDE integration", "source instance admissibility",
               "formal proof build"]
DEPENDENCIES = {"sympy": "1.14.0", "mpmath": "1.3.0"}


def output_path(root: Path, requested: Path) -> Path:
    """Keep reports out of reviewed inputs and other checkout source files."""
    target = (root / requested).resolve()
    if target.is_relative_to(root):
        parts = target.relative_to(root).parts
        if not parts or parts[0] not in {"work", "evidence"}:
            raise ValueError("Reports inside the checkout must be under work/ or evidence/")
    if target.suffix.lower() != ".json":
        raise ValueError("Report output must have a .json suffix")
    return target


def execute(root: Path) -> JsonObject:
    """Verify packaging before invoking the original script with assertions on."""
    repository = check_repository(root)
    if repository["status"] != "passed":
        return {"status": "failed", "stage": "repository", "repository": cast(JsonObject, dict(repository))}
    versions = {name: importlib.metadata.version(name) for name in DEPENDENCIES}
    if versions != DEPENDENCIES:
        raise ValueError(f"Install requirements-dev.txt with this interpreter; found {versions}")
    script = root / "docs/design/navier-runtime-verification.py"
    completed = subprocess.run([sys.executable, "-E", str(script)], cwd=root,
                               capture_output=True, text=True, encoding="utf-8", timeout=180)
    if completed.returncode:
        return {"status": "failed", "stage": "mathematical_checks",
                "returncode": completed.returncode,
                "details": completed.stderr[-4000:].replace(str(root), "<repo>")}
    result: JsonObject = dict(object_value(decode(completed.stdout)))
    if result.get("status") != "passed" or result.get("not_performed") != UNPERFORMED:
        raise ValueError("Verification result has an unexpected status or evidence scope")
    preserved = object_value(decode((root / "docs/design/navier-runtime-verification-results.json")
                           .read_text(encoding="utf-8")))
    if result != preserved:
        changed = sorted(key for key in result.keys() | preserved.keys()
                         if result.get(key) != preserved.get(key))
        raise ValueError(f"Recomputed checks differ from preserved evidence in: {changed}")
    result["bootstrap_execution"] = {
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "python_version": platform.python_version(), "platform": platform.system(),
        "machine": platform.machine(), "dependencies": versions,
        "command": "python -E docs/design/navier-runtime-verification.py",
        "source_script_sha256": sha256(script),
        "baseline_sha256": repository["checks"]["review"]["baseline_sha256"],
        "repository_checks": "passed", "assertions_enabled": True,
        "matches_preserved_report": True,
        "not_performed_by_bootstrap": UNPERFORMED,
    }
    return result


def write_report(path: Path, report: JsonObject) -> None:
    """Replace the selected report atomically, including a failed-run report."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n",
                                         dir=path.parent, suffix=".tmp", delete=False) as handle:
            temporary = Path(handle.name)
            handle.write(json.dumps(report, indent=2) + "\n")
        os.replace(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


class Arguments(argparse.Namespace):
    root: Path
    output: Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path, default=Path("work/design-checks.json"),
                        help="JSON report; relative paths resolve inside the checkout")
    args = Arguments()
    parser.parse_args(namespace=args)
    root = args.root.resolve()
    try:
        target = output_path(root, args.output)
    except (OSError, ValueError) as error:
        print(json.dumps({"status": "failed", "stage": "arguments", "error": str(error)}))
        return 2
    report: JsonObject
    try:
        if sys.flags.optimize:
            raise ValueError("Optimized Python is refused: run without -O, -OO or PYTHONOPTIMIZE")
        report = execute(root)
    except (OSError, UnicodeError, ValueError, KeyError, TypeError,
            importlib.metadata.PackageNotFoundError, subprocess.TimeoutExpired) as error:
        report = {"status": "failed", "stage": "verification",
                  "error": str(error).replace(str(root), "<repo>")}
    try:
        write_report(target, report)
    except OSError as error:
        report = {"status": "failed", "stage": "report_write",
                  "error": str(error).replace(str(root), "<repo>")}
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
