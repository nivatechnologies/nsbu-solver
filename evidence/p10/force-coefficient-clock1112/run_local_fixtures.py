"""Run eight bounded Qwen fixture tasks with exact expected values hidden in this validator."""
from __future__ import annotations

import json
import sys
from pathlib import Path

from tools.local_agents.__main__ import load_config, load_tasks
from tools.local_agents.contracts import JsonValue
from tools.local_agents.runner import QueueRunner

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent

EXPECTED: dict[str, dict[str, JsonValue]] = {
    "transverse": {"case": "transverse", "shell": 0, "l2_squared": 50, "h1_squared": 250, "curl_squared": 200, "divergence_squared": 0},
    "longitudinal": {"case": "longitudinal", "shell": 0, "l2_squared": 5, "h1_squared": 50, "curl_squared": 0, "divergence_squared": 45},
    "mixed_complex": {"case": "mixed_complex", "shell": 0, "l2_squared": 24, "h1_squared": 360, "curl_squared": 278, "divergence_squared": 58},
    "zero_mode": {"case": "zero_mode", "shell": 0, "l2_squared": 16, "h1_squared": 16, "curl_squared": 0, "divergence_squared": 0},
    "boundary_n48": {"case": "boundary_n48", "shell": 1, "l2_squared": 2, "h1_squared": 1154, "curl_squared": 1152, "divergence_squared": 0},
    "boundary_n192": {"case": "boundary_n192", "shell": 5, "l2_squared": 2, "h1_squared": 18434, "curl_squared": 18432, "divergence_squared": 0},
    "boundary_n256": {"case": "boundary_n256", "shell": 6, "l2_squared": 2, "h1_squared": 32770, "curl_squared": 32768, "divergence_squared": 0},
    "conjugacy_weight": {"case": "conjugacy_weight", "shell": 0, "l2_squared": 40, "h1_squared": 3000, "curl_squared": 2220, "divergence_squared": 740},
}


def fixture_oracle(output: dict[str, JsonValue], inputs: dict[str, JsonValue]) -> list[str]:
    expected = EXPECTED.get(str(inputs.get("case")))
    return [] if output == expected else [f"exact fixture mismatch: expected {expected!r}, got {output!r}"]


def main() -> None:
    compact = sys.argv[1:] == ["--authorized-compact-redesign"]
    prefix = "local-fixtures-compact" if compact else "local-fixtures"
    config = load_config(HERE / f"{prefix}-config.json", HERE / f"{prefix}-state.json")
    tasks = load_tasks(HERE / f"{prefix}-tasks.json")
    runner = QueueRunner(
        config,
        validators={"fixture_oracle": fixture_oracle},
        trusted_nonreview_validators=frozenset({"fixture_oracle"}),
    )
    receipts = runner.run(tasks)
    summary = {
        "schema": "p10-clock1112-local-fixture-batch-v1",
        "tasks": len(tasks),
        "validated": sum(item.get("status") == "validated_candidate" for item in receipts),
        "first_attempt": sum(item.get("status") == "validated_candidate" and len(item.get("attempts", [])) == 1 for item in receipts),
        "repairs": sum(max(0, len(item.get("attempts", [])) - 1) for item in receipts),
        "terminal_failures": sum(item.get("status") in {"failed", "budget_exhausted"} for item in receipts),
        "scientific_accepted": False,
    }
    (HERE / f"{prefix}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")


if __name__ == "__main__":
    main()
