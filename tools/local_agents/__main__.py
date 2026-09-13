"""Command-line entry point; loading packets never starts the queue implicitly."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

from .contracts import JsonValue, RunnerConfig, TaskPacket
from .runner import QueueRunner
from .validators import BUILTIN_VALIDATORS, TRUSTED_NONREVIEW


def _object(path: Path) -> dict[str, JsonValue]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object in {path}")
    return value


def _config(path: Path, state: Path | None) -> RunnerConfig:
    raw = _object(path)
    endpoint = str(raw.pop("endpoint", "") or os.environ.get("LOCAL_QWEN_ENDPOINT", ""))
    model = str(raw.pop("model", "") or os.environ.get("LOCAL_QWEN_MODEL", ""))
    if state is not None:
        raw["state_path"] = str(state)
    allowed = {
        "concurrency",
        "max_run_seconds",
        "request_timeout_seconds",
        "task_timeout_seconds",
        "max_tool_calls",
        "max_repairs",
        "pending_review_cap",
        "max_artifact_bytes",
        "max_output_bytes",
        "max_tokens",
        "final_report_reserve_seconds",
        "state_path",
    }
    unknown = set(raw) - allowed
    if unknown:
        raise ValueError(f"unknown config fields: {sorted(unknown)}")
    if "state_path" in raw:
        raw["state_path"] = Path(str(raw["state_path"]))
    return RunnerConfig(endpoint=endpoint, model=model, **raw)  # type: ignore[arg-type]


def _tasks(path: Path) -> list[TaskPacket]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw, list):
        raise ValueError("task file must be a JSON array")
    packets: list[TaskPacket] = []
    for item in raw:
        if not isinstance(item, dict):
            raise ValueError("each task packet must be an object")
        artifacts = {name: Path(value) for name, value in item.get("artifacts", {}).items()}
        packets.append(
            TaskPacket(
                task_id=item["task_id"],
                family=item["family"],
                instructions=item["instructions"],
                inputs=item["inputs"],
                output_contract=item["output_contract"],
                validator=item["validator"],
                artifacts=artifacts,
                review_required=item.get("review_required", True),
            )
        )
    return packets


def main() -> int:
    parser = argparse.ArgumentParser(description="Run bounded tasks against an explicit local model")
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--tasks", type=Path, required=True)
    parser.add_argument("--state", type=Path)
    args = parser.parse_args()
    config = _config(args.config, args.state)
    QueueRunner(
        config,
        validators=BUILTIN_VALIDATORS,
        trusted_nonreview_validators=TRUSTED_NONREVIEW,
    ).run(_tasks(args.tasks))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
