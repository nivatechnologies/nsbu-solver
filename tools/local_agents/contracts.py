"""Configuration and packet contracts for the local agent queue."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Mapping, TypeAlias

JsonScalar: TypeAlias = str | int | float | bool | None
JsonValue: TypeAlias = JsonScalar | list["JsonValue"] | dict[str, "JsonValue"]
Validator: TypeAlias = Callable[[dict[str, JsonValue], dict[str, JsonValue]], list[str]]


@dataclass(frozen=True)
class RunnerConfig:
    endpoint: str
    model: str
    concurrency: int = 8
    max_run_seconds: float = 7200.0
    request_timeout_seconds: float = 180.0
    task_timeout_seconds: float = 900.0
    max_tool_calls: int = 4
    max_repairs: int = 2
    pending_review_cap: int = 2
    max_artifact_bytes: int = 1_000_000
    max_output_bytes: int = 200_000
    max_tokens: int = 4096
    final_report_reserve_seconds: float = 60.0
    state_path: Path = Path("work/local-agent-queue-state.json")

    def checked(self) -> "RunnerConfig":
        if not self.endpoint or not self.model:
            raise ValueError("local endpoint and model are required; no fallback exists")
        if not self.endpoint.startswith(("http://", "https://")):
            raise ValueError("endpoint must be an explicit HTTP(S) URL")
        if not 1 <= self.concurrency <= 12:
            raise ValueError("concurrency must be between 1 and 12")
        positive = (
            self.max_run_seconds,
            self.request_timeout_seconds,
            self.task_timeout_seconds,
            self.pending_review_cap,
            self.max_artifact_bytes,
            self.max_output_bytes,
            self.max_tokens,
        )
        if any(value <= 0 for value in positive):
            raise ValueError("timeouts and pending review cap must be positive")
        if self.max_tool_calls < 0 or not 0 <= self.max_repairs <= 2:
            raise ValueError("tool calls must be nonnegative and repairs at most 2")
        if not 0 <= self.final_report_reserve_seconds < self.task_timeout_seconds:
            raise ValueError("final report reserve must fit inside the task timeout")
        return self


@dataclass(frozen=True)
class TaskPacket:
    task_id: str
    family: str
    instructions: str
    inputs: dict[str, JsonValue]
    output_contract: dict[str, JsonValue]
    validator: str
    artifacts: Mapping[str, Path] = field(default_factory=dict)
    review_required: bool = True

    def checked(self) -> "TaskPacket":
        if not all((self.task_id, self.family, self.instructions, self.validator)):
            raise ValueError("task id, family, instructions, and validator are required")
        for name, path in self.artifacts.items():
            if not name or not path.is_file():
                raise ValueError(f"declared artifact is unavailable: {name}")
        _check_contract_definition(self.output_contract)
        return self


def _check_contract_definition(contract: dict[str, JsonValue]) -> None:
    unknown = set(contract) - {"required", "properties"}
    if unknown:
        raise ValueError(f"unsupported output contract fields: {sorted(unknown)}")
    required = contract.get("required", [])
    properties = contract.get("properties", {})
    if not isinstance(required, list) or not all(isinstance(name, str) for name in required):
        raise ValueError("output contract required must be a string array")
    if not isinstance(properties, dict):
        raise ValueError("output contract properties must be an object")
    allowed_types = {"string", "object", "array", "boolean", "number", "integer"}
    for name, rule in properties.items():
        if not isinstance(name, str) or not isinstance(rule, dict) or set(rule) != {"type"}:
            raise ValueError(f"unsupported output property contract: {name}")
        if rule.get("type") not in allowed_types:
            raise ValueError(f"unsupported output property type: {rule.get('type')}")
