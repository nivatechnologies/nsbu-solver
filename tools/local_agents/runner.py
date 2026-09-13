"""Bounded queue controller for local Qwen task packets."""

from __future__ import annotations

import hashlib
import json
import math
import os
import queue
import signal
import threading
import time
from concurrent.futures import FIRST_COMPLETED, Future, ThreadPoolExecutor, wait
from dataclasses import asdict
from pathlib import Path
from typing import Mapping

from .contracts import JsonValue, RunnerConfig, TaskPacket, Validator
from .transport import HttpTransport, ModelReply, Transport

READ_TOOL: dict[str, JsonValue] = {
    "type": "function",
    "function": {
        "name": "read_artifact",
        "description": "Read one artifact declared by name in this task packet.",
        "parameters": {
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "additionalProperties": False,
        },
    },
}


class QueueRunner:
    def __init__(
        self,
        config: RunnerConfig,
        transport: Transport | None = None,
        validators: Mapping[str, Validator] | None = None,
        trusted_nonreview_validators: frozenset[str] = frozenset(),
    ) -> None:
        self.config = config.checked()
        self.transport = transport or HttpTransport(config.endpoint)
        self._stop = threading.Event()
        self.receipts: list[dict[str, JsonValue]] = []
        self._family_failures: dict[str, int] = {}
        self._validators = dict(validators or {})
        self._trusted_nonreview = trusted_nonreview_validators
        self._artifact_bytes: dict[str, bytes] = {}
        self._inflight: dict[str, dict[str, JsonValue]] = {}
        self._load_state()

    def stop(self) -> None:
        self._stop.set()

    def run(self, tasks: list[TaskPacket]) -> list[dict[str, JsonValue]]:
        started = time.monotonic()
        ready = self._unique_tasks(tasks)
        pending_review = sum(
            r.get("status") == "validated_candidate" and r.get("review_required", True)
            for r in self.receipts
        )
        active_review = 0
        old_handler = self._install_signal_handler()
        try:
            with ThreadPoolExecutor(max_workers=self.config.concurrency) as pool:
                futures: dict[Future[dict[str, JsonValue]], TaskPacket] = {}
                while ready or futures:
                    run_expired = time.monotonic() - started >= self.config.max_run_seconds
                    while (
                        ready
                        and len(futures) < self.config.concurrency
                        and not self._stop.is_set()
                        and not run_expired
                    ):
                        index = self._next_dispatchable(ready, pending_review + active_review)
                        if index is None:
                            break
                        task = ready.pop(index)
                        if self._family_failures.get(task.family, 0) >= 3:
                            self.receipts.append(self._deferred_receipt(task, "family_paused"))
                            continue
                        self._inflight[task.task_id] = {
                            "task_id": task.task_id,
                            "family": task.family,
                            "input_hash": self._task_hash(task),
                            "status": "running",
                            "attempts_reserved": self.config.max_repairs + 1,
                            "tool_calls_reserved": self.config.max_tool_calls,
                        }
                        self.save_state()
                        futures[pool.submit(self._run_task, task, started)] = task
                        active_review += int(task.review_required)
                    if not futures:
                        break
                    done, _ = wait(futures, return_when=FIRST_COMPLETED)
                    for future in done:
                        task = futures.pop(future)
                        active_review -= int(task.review_required)
                        receipt = future.result()
                        self._inflight.pop(task.task_id, None)
                        self.receipts.append(receipt)
                        family = str(receipt["family"])
                        if receipt["status"] == "validated_candidate":
                            self._family_failures[family] = 0
                            pending_review += int(task.review_required)
                        elif receipt["status"] == "failed":
                            self._family_failures[family] = self._family_failures.get(family, 0) + 1
                        self.save_state()
                for task in ready:
                    status = (
                        "deferred_on_shutdown"
                        if self._stop.is_set()
                        else "deferred_on_budget"
                        if time.monotonic() - started >= self.config.max_run_seconds
                        else "deferred_review_backpressure"
                    )
                    self.receipts.append(self._deferred_receipt(task, status))
        finally:
            self.save_state()
            if old_handler is not None:
                signal.signal(signal.SIGINT, old_handler)
        return self.receipts

    def _next_dispatchable(self, ready: list[TaskPacket], used_review: int) -> int | None:
        for index, task in enumerate(ready):
            if not task.review_required or used_review < self.config.pending_review_cap:
                return index
        return None

    def _run_task(self, task: TaskPacket, run_started: float) -> dict[str, JsonValue]:
        begun = time.monotonic()
        attempts: list[JsonValue] = []
        errors: list[str] = []
        tokens = [0, 0, 0]
        tool_calls = 0
        status = "failed"
        output: dict[str, JsonValue] | None = None
        original_hash = self._task_hash(task)
        if self._family_failures.get(task.family, 0) >= 3:
            errors = ["task family paused after 3 consecutive failures"]
            status = "family_paused"
        else:
            for attempt in range(self.config.max_repairs + 1):
                if self._expired(begun, run_started):
                    errors = ["task or run wall-clock budget exhausted"]
                    status = "budget_exhausted"
                    break
                deadline = min(
                    begun + self.config.task_timeout_seconds,
                    run_started + self.config.max_run_seconds,
                )
                usage = [tool_calls, tokens[0], tokens[1], tokens[2]]
                try:
                    result = self._attempt(task, errors, usage, deadline)
                except Exception as exc:
                    errors = [f"inference failed: {type(exc).__name__}: {exc}"]
                    attempts.append({"number": attempt + 1, "validator_errors": list(errors)})
                    tool_calls, tokens[0], tokens[1], tokens[2] = usage
                    if isinstance(exc, TimeoutError):
                        status = "budget_exhausted"
                        errors.append("server-side cancellation is unknown")
                        break
                    continue
                tool_calls, tokens[0], tokens[1], tokens[2] = usage
                output, errors, checker_seconds = self._validate(task, result)
                attempts.append(
                    {
                        "number": attempt + 1,
                        "validator_errors": list(errors),
                        "checker_seconds": checker_seconds,
                    }
                )
                if not errors and self._task_hash(task, refresh=True) == original_hash:
                    status = "validated_candidate"
                    break
                if not errors:
                    errors = ["declared artifact changed during task"]
        if status != "validated_candidate":
            output = None
        elapsed = round(time.monotonic() - begun, 6)
        return {
            "task_id": task.task_id,
            "family": task.family,
            "status": status,
            "scientific_accepted": False,
            "review_required": task.review_required,
            "attempts": attempts,
            "tool_calls": tool_calls,
            "timing_seconds": elapsed,
            "prompt_tokens": tokens[0],
            "completion_tokens": tokens[1],
            "reasoning_tokens": tokens[2],
            "input_hash": self._task_hash(task),
            "output_hash": self._json_hash(output) if output is not None else None,
            "output": output,
            "errors": errors,
        }

    def _attempt(
        self, task: TaskPacket, errors: list[str], usage: list[int], deadline: float
    ) -> dict[str, JsonValue] | None:
        messages: list[JsonValue] = [{"role": "system", "content": self._system_prompt(task)}]
        prompt = {"task": task.instructions, "inputs": task.inputs, "output_contract": task.output_contract}
        if errors:
            prompt["repair_validator_errors"] = list(errors)
        messages.append({"role": "user", "content": json.dumps(prompt, sort_keys=True)})
        tool_deadline = deadline - self.config.final_report_reserve_seconds
        while usage[0] < self.config.max_tool_calls and time.monotonic() < tool_deadline:
            reply = self._complete(messages, include_tools=True, deadline=tool_deadline)
            self._add_tokens(usage, reply, offset=1)
            if not reply.tool_calls:
                break
            if len(reply.tool_calls) != 1:
                raise ValueError("exactly one tool call is permitted per response")
            call = reply.tool_calls[0]
            if call.name != "read_artifact":
                raise ValueError(f"unsupported tool call: {call.name}")
            arguments = json.loads(call.arguments)
            request = arguments.get("name") if isinstance(arguments, dict) else None
            if not isinstance(request, str):
                raise ValueError("read_artifact requires string name")
            usage[0] += 1
            artifact = task.artifacts.get(request)
            content = self._read_artifact(artifact)
            messages.extend(
                [
                    {
                        "role": "assistant",
                        "content": reply.content,
                        "tool_calls": [{"id": call.call_id, "type": "function", "function": {"name": call.name, "arguments": call.arguments}}],
                    },
                    {"role": "tool", "tool_call_id": call.call_id, "content": content},
                ]
            )
        messages.append({"role": "user", "content": "FINAL REPORT: return only the contracted JSON object."})
        final_reply = self._complete(messages, include_tools=False, deadline=deadline)
        self._add_tokens(usage, final_reply, offset=1)
        if len(final_reply.content.encode()) > self.config.max_output_bytes:
            return None
        return self._parse_object(final_reply.content)

    def _complete(
        self, messages: list[JsonValue], *, include_tools: bool, deadline: float
    ) -> ModelReply:
        request: dict[str, JsonValue] = {
            "model": self.config.model,
            "messages": messages,
            "chat_template_kwargs": {"enable_thinking": True},
            "temperature": 0,
            "max_tokens": self.config.max_tokens,
        }
        if include_tools:
            request["tools"] = [READ_TOOL]
        timeout = min(self.config.request_timeout_seconds, deadline - time.monotonic())
        if timeout <= 0:
            raise TimeoutError("task or run wall-clock budget exhausted")
        reply = self._bounded_complete(request, timeout)
        if reply.finish_reason in {"length", "content_filter"}:
            raise ValueError(f"model response refused due to finish_reason={reply.finish_reason}")
        if len(reply.content.encode()) > self.config.max_output_bytes:
            raise ValueError("model response exceeds byte limit")
        return reply

    def _bounded_complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply:
        result: queue.Queue[ModelReply | BaseException] = queue.Queue(maxsize=1)

        def call() -> None:
            try:
                result.put(self.transport.complete(request, timeout))
            except BaseException as exc:
                result.put(exc)

        worker = threading.Thread(target=call, name="local-model-request", daemon=True)
        worker.start()
        try:
            value = result.get(timeout=timeout)
        except queue.Empty as exc:
            raise TimeoutError("model request exceeded remaining wall-clock budget") from exc
        if isinstance(value, BaseException):
            raise value
        return value

    def _validate(
        self, task: TaskPacket, result: dict[str, JsonValue] | None
    ) -> tuple[dict[str, JsonValue] | None, list[str], float]:
        if result is None:
            return None, ["final output is not one JSON object"], 0.0
        errors = validate_contract(result, task.output_contract)
        if errors:
            return result, errors, 0.0
        checker_started = time.monotonic()
        try:
            checker = self._validators.get(task.validator)
            if checker is None:
                errors.append(f"validator is not registered: {task.validator}")
            else:
                errors.extend(checker(result, task.inputs))
        except Exception as exc:  # validator failures must fail closed
            errors.append(f"validator raised {type(exc).__name__}: {exc}")
        return result, errors, round(time.monotonic() - checker_started, 6)

    def _expired(self, task_started: float, run_started: float) -> bool:
        return (
            self._stop.is_set()
            or time.monotonic() - task_started >= self.config.task_timeout_seconds
            or time.monotonic() - run_started >= self.config.max_run_seconds
        )

    def _unique_tasks(self, tasks: list[TaskPacket]) -> list[TaskPacket]:
        terminal = {
            "validated_candidate",
            "failed",
            "budget_exhausted",
            "family_paused",
            "interrupted_requires_explicit_requeue",
        }
        seen = {str(r.get("input_hash")) for r in self.receipts if r.get("status") in terminal}
        result: list[TaskPacket] = []
        for task in tasks:
            task.checked()
            if not task.review_required and task.validator not in self._trusted_nonreview:
                raise ValueError("review_required=false needs a trusted deterministic validator")
            digest = self._task_hash(task)
            if digest not in seen:
                seen.add(digest)
                result.append(task)
        return result

    def save_state(self) -> None:
        state: dict[str, JsonValue] = {
            "schema": "nsbu-local-agent-queue-v1",
            "receipts": self.receipts,
            "family_consecutive_failures": self._family_failures,
            "inflight": self._inflight,
        }
        path = self.config.state_path
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + ".tmp")
        temporary.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        os.replace(temporary, path)

    @staticmethod
    def _parse_object(content: str) -> dict[str, JsonValue] | None:
        try:
            value = json.loads(content)
        except (json.JSONDecodeError, TypeError):
            return None
        return value if isinstance(value, dict) else None

    @staticmethod
    def _system_prompt(task: TaskPacket) -> str:
        names = sorted(task.artifacts)
        return (
            "Reasoning is enabled. Stay within the fixed task packet. During the tool phase, "
            f"use only the provided read_artifact tool with one declared name. Names: {names}. "
            "Do not request shell commands, paths, network access, commits, merges, or scientific acceptance."
        )

    def _task_hash(self, task: TaskPacket, *, refresh: bool = False) -> str:
        artifact_hashes: dict[str, str] = {}
        for name, path in sorted(task.artifacts.items()):
            key = str(path.resolve())
            if refresh or key not in self._artifact_bytes:
                size = path.stat().st_size
                if size > self.config.max_artifact_bytes:
                    raise ValueError(f"artifact exceeds byte limit: {name}")
                data = path.read_bytes()
                if len(data) != size:
                    raise ValueError(f"artifact changed while freezing: {name}")
                if not refresh:
                    self._artifact_bytes[key] = data
            else:
                data = self._artifact_bytes[key]
            artifact_hashes[name] = hashlib.sha256(data).hexdigest()
        value = {
            "id": task.task_id,
            "family": task.family,
            "instructions": task.instructions,
            "inputs": task.inputs,
            "contract": task.output_contract,
            "validator": task.validator,
            "artifacts": artifact_hashes,
            "review_required": task.review_required,
        }
        return self._json_hash(value)

    @staticmethod
    def _json_hash(value: object) -> str:
        encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
        return hashlib.sha256(encoded).hexdigest()

    @staticmethod
    def _add_tokens(counts: list[int], reply: ModelReply, *, offset: int = 0) -> None:
        counts[offset] += reply.prompt_tokens
        counts[offset + 1] += reply.completion_tokens
        counts[offset + 2] += reply.reasoning_tokens

    def _read_artifact(self, path: Path | None) -> str | None:
        if path is None:
            return None
        data = self._artifact_bytes[str(path.resolve())]
        return data.decode("utf-8")

    def _load_state(self) -> None:
        path = self.config.state_path
        if not path.is_file():
            return
        value = json.loads(path.read_text(encoding="utf-8"))
        if value.get("schema") != "nsbu-local-agent-queue-v1":
            raise ValueError("unsupported queue state schema")
        self.receipts = list(value.get("receipts", []))
        self._family_failures = dict(value.get("family_consecutive_failures", {}))
        abandoned = value.get("inflight", {})
        if isinstance(abandoned, dict):
            for item in abandoned.values():
                if isinstance(item, dict):
                    receipt = dict(item)
                    receipt["status"] = "interrupted_requires_explicit_requeue"
                    receipt["scientific_accepted"] = False
                    receipt["errors"] = ["prior process ended with this task in flight"]
                    self.receipts.append(receipt)

    def _deferred_receipt(self, task: TaskPacket, status: str) -> dict[str, JsonValue]:
        return {
            "task_id": task.task_id,
            "family": task.family,
            "status": status,
            "scientific_accepted": False,
            "review_required": task.review_required,
            "attempts": [],
            "input_hash": self._task_hash(task),
            "errors": [],
        }

    def _install_signal_handler(self) -> object | None:
        if threading.current_thread() is not threading.main_thread():
            return None
        previous = signal.getsignal(signal.SIGINT)
        signal.signal(signal.SIGINT, lambda _signum, _frame: self.stop())
        return previous


def validate_contract(output: dict[str, JsonValue], contract: dict[str, JsonValue]) -> list[str]:
    errors: list[str] = []
    required = contract.get("required", [])
    if isinstance(required, list):
        for name in required:
            if isinstance(name, str) and name not in output:
                errors.append(f"missing required output field: {name}")
    properties = contract.get("properties", {})
    type_map: Mapping[str, type[object] | tuple[type[object], ...]] = {
        "string": str,
        "object": dict,
        "array": list,
        "boolean": bool,
        "number": (int, float),
        "integer": int,
    }
    if isinstance(properties, dict):
        for name, rule in properties.items():
            if name in output and isinstance(rule, dict) and isinstance(rule.get("type"), str):
                expected = type_map.get(rule["type"])
                value = output[name]
                numeric = rule["type"] in {"number", "integer"}
                invalid_number = numeric and (
                    isinstance(value, bool)
                    or (isinstance(value, float) and not math.isfinite(value))
                )
                if expected is not None and (not isinstance(value, expected) or invalid_number):
                    errors.append(f"output field {name} must have type {rule['type']}")
    return errors
