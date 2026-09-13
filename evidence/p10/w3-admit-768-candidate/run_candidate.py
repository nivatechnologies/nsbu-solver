from __future__ import annotations

import json
import subprocess
from pathlib import Path

from tools.local_agents.__main__ import load_config, load_tasks
from tools.local_agents.contracts import JsonValue
from tools.local_agents.runner import QueueRunner

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
TARGET = "crates/nsbu-solver/src/spectral/w3/admission.rs"


def only_768_admission(output: dict[str, JsonValue], _: dict[str, JsonValue]) -> list[str]:
    if set(output) != {"patch"} or not isinstance(output.get("patch"), str):
        return ["output must contain exactly one string field: patch"]
    patch = output["patch"]
    assert isinstance(patch, str)
    errors: list[str] = []
    expected_headers = {
        f"diff --git a/{TARGET} b/{TARGET}",
        f"--- a/{TARGET}",
        f"+++ b/{TARGET}",
    }
    headers = {line for line in patch.splitlines() if line.startswith(("diff --git ", "--- ", "+++ "))}
    if headers != expected_headers:
        errors.append(f"patch headers must target only {TARGET}")
    if patch.count("@@") != 2 or "fn admit_layout" not in patch:
        errors.append("patch must contain one hunk confined to fn admit_layout")
    changed = [
        line for line in patch.splitlines()
        if (line.startswith("+") and not line.startswith("+++"))
        or (line.startswith("-") and not line.startswith("---"))
    ]
    if not changed or any(not line[1:].strip() for line in changed):
        errors.append("patch must have only nonblank changed source lines")
    removed = "".join("".join(line[1:].split()) for line in changed if line.startswith("-"))
    added = "".join("".join(line[1:].split()) for line in changed if line.startswith("+"))
    arm = "|[768,768,768]"
    if added.count(arm) != 1 or added.replace(arm, "", 1) != removed:
        errors.append("the sole semantic change must be one [768,768,768] match alternative")
    if "[1024,1024,1024]" in added or "[1152,1152,1152]" in added:
        errors.append("1024 and 1152 must remain unadmitted")
    checked = subprocess.run(
        ["git", "apply", "--check", "--whitespace=nowarn", "-"],
        cwd=ROOT,
        input=patch,
        text=True,
        capture_output=True,
        check=False,
    )
    if checked.returncode:
        errors.append(f"git apply --check failed: {checked.stderr.strip()}")
    return errors


def main() -> None:
    config = load_config(HERE / "config.json", HERE / "state.json")
    tasks = load_tasks(HERE / "tasks.json")
    runner = QueueRunner(config, validators={"only_768_admission": only_768_admission})
    receipts = runner.run(tasks)
    (HERE / "summary.json").write_text(
        json.dumps({"schema": "p10-w3-admit-768-qwen-candidate-v1", "receipts": receipts}, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
