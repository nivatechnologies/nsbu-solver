"""Command-line entry point; loading packets never starts the queue implicitly."""

from __future__ import annotations

import argparse
import os
from collections.abc import Mapping, Sequence
from pathlib import Path

from .contracts import JsonValue, RunnerConfig, TaskPacket
from .runner import QueueRunner
from .validators import BUILTIN_VALIDATORS, TRUSTED_NONREVIEW
from tools.json_types import Json, decode, object_value, string_value


def _object(path: Path) -> dict[str, JsonValue]:
    return dict(object_value(decode(path.read_text(encoding="utf-8"))))


def _config(path: Path, state: Path | None) -> RunnerConfig:
    raw = _object(path)
    endpoint = _optional_string(raw.pop("endpoint", "")) or os.environ.get("LOCAL_QWEN_ENDPOINT", "")
    model = _optional_string(raw.pop("model", "")) or os.environ.get("LOCAL_QWEN_MODEL", "")
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
    state_path = state or Path(_optional_string(raw.get("state_path", "work/local-agent-queue-state.json")))
    return RunnerConfig(
        endpoint=endpoint,
        model=model,
        concurrency=_integer(raw, "concurrency", 8),
        max_run_seconds=_number(raw, "max_run_seconds", 7200.0),
        request_timeout_seconds=_number(raw, "request_timeout_seconds", 180.0),
        task_timeout_seconds=_number(raw, "task_timeout_seconds", 900.0),
        max_tool_calls=_integer(raw, "max_tool_calls", 4),
        max_repairs=_integer(raw, "max_repairs", 2),
        pending_review_cap=_integer(raw, "pending_review_cap", 2),
        max_artifact_bytes=_integer(raw, "max_artifact_bytes", 1_000_000),
        max_output_bytes=_integer(raw, "max_output_bytes", 200_000),
        max_tokens=_integer(raw, "max_tokens", 4096),
        final_report_reserve_seconds=_number(raw, "final_report_reserve_seconds", 60.0),
        state_path=state_path,
    )


def _tasks(path: Path) -> list[TaskPacket]:
    raw = decode(path.read_text(encoding="utf-8"))
    if not isinstance(raw, Sequence) or isinstance(raw, str):
        raise ValueError("task file must be a JSON array")
    packets: list[TaskPacket] = []
    for item in raw:
        packet = object_value(item)
        raw_artifacts = object_value(packet.get("artifacts", {}))
        artifacts = {name: Path(string_value(value)) for name, value in raw_artifacts.items()}
        packets.append(
            TaskPacket(
                task_id=string_value(packet.get("task_id")),
                family=string_value(packet.get("family")),
                instructions=string_value(packet.get("instructions")),
                inputs=dict(object_value(packet.get("inputs"))),
                output_contract=dict(object_value(packet.get("output_contract"))),
                validator=string_value(packet.get("validator")),
                artifacts=artifacts,
                review_required=_bool(packet.get("review_required", True)),
            )
        )
    return packets


class _Args(argparse.Namespace):
    config: Path
    tasks: Path
    state: Path | None


def main() -> int:
    parser = argparse.ArgumentParser(description="Run bounded tasks against an explicit local model")
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--tasks", type=Path, required=True)
    parser.add_argument("--state", type=Path)
    args = parser.parse_args(namespace=_Args())
    config = _config(args.config, args.state)
    QueueRunner(
        config,
        validators=BUILTIN_VALIDATORS,
        trusted_nonreview_validators=TRUSTED_NONREVIEW,
    ).run(_tasks(args.tasks))
    return 0


def _optional_string(value: Json) -> str:
    return "" if value is None else string_value(value)


def _integer(values: Mapping[str, Json], key: str, default: int) -> int:
    value = values.get(key, default)
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError(f"config field {key} must be an integer")
    return value


def _number(values: Mapping[str, Json], key: str, default: float) -> float:
    value = values.get(key, default)
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        raise ValueError(f"config field {key} must be a number")
    return float(value)


def _bool(value: Json) -> bool:
    if not isinstance(value, bool):
        raise ValueError("review_required must be boolean")
    return value


if __name__ == "__main__":
    raise SystemExit(main())
