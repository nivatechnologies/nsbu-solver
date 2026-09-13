from __future__ import annotations

import time
from io import BytesIO
from collections.abc import Sequence
from dataclasses import replace
from pathlib import Path
from typing import TypedDict, Unpack
from unittest.mock import patch

import pytest

from tools.local_agents import QueueRunner, RunnerConfig, TaskPacket
from tools.local_agents.contracts import JsonValue
from tools.local_agents.contracts import validate_contract
from tools.local_agents.transport import HttpTransport, ModelReply, ToolCall
from tools.local_agents.validators import exact_expected
from tools.local_agents.__main__ import load_config, load_tasks
from tools.json_types import array_value, decode, object_value, string_value


class ConfigChanges(TypedDict, total=False):
    concurrency: int
    max_run_seconds: float
    request_timeout_seconds: float
    task_timeout_seconds: float
    max_tool_calls: int
    max_repairs: int
    pending_review_cap: int
    max_output_bytes: int
    final_report_reserve_seconds: float


class FakeTransport:
    def __init__(self, replies: Sequence[str | ModelReply]) -> None:
        self.replies = iter(replies)
        self.requests: list[dict[str, JsonValue]] = []
        self.timeouts: list[float] = []

    def complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply:
        self.requests.append(request)
        self.timeouts.append(timeout)
        reply = next(self.replies)
        return reply if isinstance(reply, ModelReply) else ModelReply(reply, 10, 4, 2)


class HangingTransport:
    def complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply:
        time.sleep(1)
        return ModelReply("{}")


def packet(task_id: str = "one", *, review: bool = True, family: str = "math") -> TaskPacket:
    return TaskPacket(
        task_id=task_id,
        family=family,
        instructions="Return the exact checked sum.",
        inputs={"expected_output": {"sum": 5}},
        output_contract={
            "required": ["sum"],
            "properties": {"sum": {"type": "integer"}},
        },
        validator="exact_expected",
        review_required=review,
    )


def config(
    tmp_path: Path,
    *,
    concurrency: int = 8,
    max_run_seconds: float = 7200,
    request_timeout_seconds: float = 180,
    task_timeout_seconds: float = 900,
    max_tool_calls: int = 4,
    max_repairs: int = 2,
    pending_review_cap: int = 2,
    max_output_bytes: int = 200_000,
    final_report_reserve_seconds: float = 60,
) -> RunnerConfig:
    return RunnerConfig(
        endpoint="http://spark.local:8000",
        model="qwen-test",
        state_path=tmp_path / "state.json",
        concurrency=concurrency,
        max_run_seconds=max_run_seconds,
        request_timeout_seconds=request_timeout_seconds,
        task_timeout_seconds=task_timeout_seconds,
        max_tool_calls=max_tool_calls,
        max_repairs=max_repairs,
        pending_review_cap=pending_review_cap,
        max_output_bytes=max_output_bytes,
        final_report_reserve_seconds=final_report_reserve_seconds,
    )


def runner(
    tmp_path: Path, replies: Sequence[str | ModelReply], **changes: Unpack[ConfigChanges]
) -> tuple[QueueRunner, FakeTransport]:
    fake = FakeTransport(replies)
    queue = QueueRunner(
        config(tmp_path, **changes),
        fake,
        validators={"exact_expected": exact_expected},
        trusted_nonreview_validators=frozenset({"exact_expected"}),
    )
    return queue, fake


def messages(fake: FakeTransport, request_index: int) -> Sequence[JsonValue]:
    return array_value(fake.requests[request_index]["messages"])


def errors(receipt: dict[str, JsonValue]) -> list[str]:
    return [string_value(item) for item in array_value(receipt["errors"])]


def attempts(receipt: dict[str, JsonValue]) -> Sequence[JsonValue]:
    return array_value(receipt["attempts"])


def test_final_phase_omits_tools_and_all_requests_enable_thinking(tmp_path: Path) -> None:
    queue, fake = runner(tmp_path, ['{"sum":5}', '{"sum":5}'])
    receipt = queue.run([packet()])[0]
    assert receipt["status"] == "validated_candidate"
    assert "tools" in fake.requests[0]
    assert "tools" not in fake.requests[-1]
    assert all(r["chat_template_kwargs"] == {"enable_thinking": True} for r in fake.requests)
    assert receipt["reasoning_tokens"] == 4
    assert receipt["scientific_accepted"] is False


def test_tool_reads_only_declared_artifact_and_honors_call_budget(tmp_path: Path) -> None:
    artifact = tmp_path / "fixture.txt"
    artifact.write_text("fixed", encoding="utf-8")
    task = packet()
    task = replace(task, artifacts={"fixture": artifact})
    queue, fake = runner(
        tmp_path,
        [
            ModelReply("", tool_calls=(ToolCall("call-1", "read_artifact", '{"name":"fixture"}'),)),
            '{"sum":5}',
        ],
        max_tool_calls=1,
        request_timeout_seconds=1000,
        final_report_reserve_seconds=100,
    )
    receipt = queue.run([task])[0]
    assert receipt["tool_calls"] == 1
    tool_message = object_value(messages(fake, 1)[3])
    assert tool_message["content"] == "fixed"
    assert tool_message["role"] == "tool"
    assert fake.timeouts[0] < fake.timeouts[1]


def test_invalid_outputs_get_two_repairs_with_exact_errors_then_fail_closed(tmp_path: Path) -> None:
    replies = ['{}', '{}', '{"sum":"bad"}', '{"sum":"bad"}', 'not json', 'not json']
    queue, fake = runner(tmp_path, replies, max_tool_calls=0, max_repairs=2)
    receipt = queue.run([packet()])[0]
    assert receipt["status"] == "failed"
    assert len(attempts(receipt)) == 3
    prompt_text = string_value(object_value(messages(fake, 2)[1])["content"])
    repair_prompt = object_value(decode(prompt_text))
    assert repair_prompt["repair_validator_errors"] == ["missing required output field: sum"]
    assert receipt["output"] is None


def test_review_capacity_reserves_inflight_slots_and_defers_without_failure(tmp_path: Path) -> None:
    queue, _ = runner(tmp_path, ['{"sum":5}', '{"sum":5}'], pending_review_cap=1)
    receipts = queue.run([packet("a"), packet("b")])
    assert sorted(string_value(r["status"]) for r in receipts) == [
        "deferred_review_backpressure",
        "validated_candidate",
    ]


def test_terminal_state_prevents_duplicate_rerun(tmp_path: Path) -> None:
    first, _ = runner(tmp_path, ['{"sum":5}', '{"sum":5}'])
    first.run([packet()])
    second, fake = runner(tmp_path, [])
    receipts = second.run([packet()])
    assert len(receipts) == 1
    assert not fake.requests


def test_no_endpoint_fallback_and_nonreview_requires_trust(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="no fallback"):
        RunnerConfig(endpoint="", model="qwen").checked()
    queue = QueueRunner(config(tmp_path), FakeTransport([]), validators={"exact_expected": exact_expected})
    with pytest.raises(ValueError, match="trusted deterministic"):
        queue.run([packet(review=False)])


def test_output_byte_limit_and_expired_budget_fail_closed(tmp_path: Path) -> None:
    queue, _ = runner(tmp_path, ['{"sum":55555}'] * 6, max_tool_calls=0, max_output_bytes=4)
    assert queue.run([packet()])[0]["status"] == "failed"
    expired, fake = runner(tmp_path / "other", [], max_run_seconds=0.000001)
    time.sleep(0.001)
    receipt = expired.run([packet()])[0]
    assert receipt["status"] == "deferred_on_budget"
    assert not fake.requests


def test_three_consecutive_failures_pause_family_without_more_inference(tmp_path: Path) -> None:
    replies = ["bad", "bad"] * 3
    for index in range(3):
        queue, _ = runner(tmp_path, replies[index * 2 : index * 2 + 2], max_repairs=0)
        assert queue.run([packet(str(index))])[-1]["status"] == "failed"
    paused, fake = runner(tmp_path, [])
    receipt = paused.run([packet("fourth")])[-1]
    assert receipt["status"] == "family_paused"
    assert not fake.requests


def test_outer_wall_guard_does_not_wait_for_uncooperative_transport(tmp_path: Path) -> None:
    queue = QueueRunner(
        config(
            tmp_path,
            request_timeout_seconds=0.01,
            task_timeout_seconds=0.02,
            final_report_reserve_seconds=0.005,
        ),
        HangingTransport(),
        validators={"exact_expected": exact_expected},
    )
    started = time.monotonic()
    receipt = queue.run([packet()])[0]
    assert time.monotonic() - started < 0.2
    assert receipt["status"] == "budget_exhausted"
    assert "TimeoutError" in errors(receipt)[0]
    assert errors(receipt)[-1] == "server-side cancellation is unknown"
    assert receipt["prompt_tokens"] is None


def test_queue_refills_configured_worker_slot_for_trusted_tasks(tmp_path: Path) -> None:
    queue, fake = runner(tmp_path, ['{"sum":5}'] * 3, concurrency=1, max_tool_calls=0)
    receipts = queue.run([packet(str(i), review=False) for i in range(3)])
    assert [r["status"] for r in receipts] == ["validated_candidate"] * 3
    assert len(fake.requests) == 3


def test_numeric_contract_rejects_bool_and_nonfinite() -> None:
    contract: dict[str, JsonValue] = {"properties": {"x": {"type": "integer"}}}
    assert validate_contract({"x": True}, contract)
    contract = {"properties": {"x": {"type": "number"}}}
    assert validate_contract({"x": float("nan")}, contract)


def test_endpoint_normalization_and_truncated_reply_refusal(tmp_path: Path) -> None:
    assert HttpTransport("http://spark/v1").endpoint == "http://spark/v1/chat/completions"
    queue, _ = runner(
        tmp_path,
        [ModelReply('{"sum":5}', prompt_tokens=10, completion_tokens=4, finish_reason="length")],
        max_tool_calls=0,
        max_repairs=0,
    )
    receipt = queue.run([packet()])[0]
    assert receipt["status"] == "failed"
    assert "finish_reason=length" in errors(receipt)[0]
    assert receipt["completion_tokens"] == 4


def test_unsupported_contract_is_rejected_before_inference(tmp_path: Path) -> None:
    task = packet()
    invalid = replace(task, output_contract={"additionalProperties": False})
    queue, fake = runner(tmp_path, [])
    with pytest.raises(ValueError, match="unsupported output contract"):
        queue.run([invalid])
    assert not fake.requests


def test_duplicate_task_id_with_different_packet_is_rejected(tmp_path: Path) -> None:
    first = packet("same")
    second = replace(first, instructions="different work")
    queue, fake = runner(tmp_path, [])
    with pytest.raises(ValueError, match="task id reused"):
        queue.run([first, second])
    assert not fake.requests


@pytest.mark.parametrize("value", [float("nan"), float("inf")])
def test_nonfinite_time_budgets_are_rejected(value: float) -> None:
    with pytest.raises(ValueError, match="timeouts"):
        RunnerConfig(endpoint="http://spark", model="qwen", max_run_seconds=value).checked()


def test_final_tool_calls_and_unknown_finish_reasons_fail_closed(tmp_path: Path) -> None:
    replies = [
        ModelReply('{"sum":5}', tool_calls=(ToolCall("x", "read_artifact", '{}'),)),
        ModelReply('{"sum":5}', finish_reason="error"),
    ]
    queue, _ = runner(tmp_path, replies, max_tool_calls=0, max_repairs=1)
    receipt = queue.run([packet()])[0]
    assert receipt["status"] == "failed"
    assert len(attempts(receipt)) == 2


def test_task_timeouts_count_toward_family_pause(tmp_path: Path) -> None:
    for index in range(3):
        queue = QueueRunner(
            config(
                tmp_path,
                request_timeout_seconds=0.001,
                task_timeout_seconds=0.002,
                final_report_reserve_seconds=0.0005,
                max_repairs=0,
            ),
            HangingTransport(),
            validators={"exact_expected": exact_expected},
        )
        assert queue.run([packet(f"timeout-{index}")])[-1]["status"] == "budget_exhausted"
    paused, fake = runner(tmp_path, [])
    assert paused.run([packet("paused")])[-1]["status"] == "family_paused"
    assert not fake.requests


def test_cli_loads_checked_config_and_task_packets(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    config_path = tmp_path / "config.json"
    config_path.write_text('{"concurrency":3,"max_tokens":99}', encoding="utf-8")
    monkeypatch.setenv("LOCAL_QWEN_ENDPOINT", "http://local/v1")
    monkeypatch.setenv("LOCAL_QWEN_MODEL", "fixed-model")
    loaded = load_config(config_path, tmp_path / "state.json")
    assert (loaded.endpoint, loaded.model, loaded.concurrency, loaded.max_tokens) == (
        "http://local/v1",
        "fixed-model",
        3,
        99,
    )
    task_path = tmp_path / "tasks.json"
    task_path.write_text(
        '[{"task_id":"t","family":"f","instructions":"i","inputs":{},'
        '"output_contract":{"required":[],"properties":{}},"validator":"exact_expected"}]',
        encoding="utf-8",
    )
    assert load_tasks(task_path)[0].task_id == "t"


@pytest.mark.parametrize(
    ("payload", "message"),
    [
        ('{"unknown":1}', "unknown config"),
        ('{"concurrency":"many"}', "must be an integer"),
    ],
)
def test_cli_rejects_malformed_config(tmp_path: Path, payload: str, message: str) -> None:
    path = tmp_path / "bad.json"
    path.write_text(payload, encoding="utf-8")
    with pytest.raises(ValueError, match=message):
        load_config(path)


@pytest.mark.parametrize(
    "payload",
    [
        '{}',
        '[1]',
        '[{"task_id":"missing"}]',
        '[{"task_id":"t","family":"f","instructions":"i","inputs":{},'
        '"output_contract":{},"validator":"v","review_required":"yes"}]',
    ],
)
def test_cli_rejects_malformed_task_payloads(tmp_path: Path, payload: str) -> None:
    path = tmp_path / "bad-tasks.json"
    path.write_text(payload, encoding="utf-8")
    with pytest.raises(ValueError):
        load_tasks(path)


class FakeHttpResponse:
    def __init__(self, body: bytes, *, fail_read: bool = False) -> None:
        self.body = BytesIO(body)
        self.fail_read = fail_read
        self.closed = False

    def read(self, limit: int) -> bytes:
        if self.fail_read:
            raise OSError("read failed")
        return self.body.read(limit)

    def close(self) -> None:
        self.closed = True


def test_http_transport_parses_usage_tools_and_closes_response() -> None:
    body = (
        b'{"choices":[{"finish_reason":"tool_calls","message":{"content":null,'
        b'"tool_calls":[{"id":"c","function":{"name":"read_artifact",'
        b'"arguments":"{\\"name\\":\\"fixture\\"}"}}]}}],"usage":{'
        b'"prompt_tokens":7,"completion_tokens":3,"completion_tokens_details":'
        b'{"reasoning_tokens":2}}}'
    )
    response = FakeHttpResponse(body)
    with patch("tools.local_agents.transport.urlopen", return_value=response):
        reply = HttpTransport("http://local/v1").complete({"model": "m"}, 1)
    assert response.closed
    assert (reply.prompt_tokens, reply.completion_tokens, reply.reasoning_tokens) == (7, 3, 2)
    assert reply.tool_calls[0].name == "read_artifact"


def test_http_transport_bounds_and_closes_failed_reads() -> None:
    oversized = FakeHttpResponse(b"12345")
    with patch("tools.local_agents.transport.urlopen", return_value=oversized):
        with pytest.raises(ValueError, match="byte limit"):
            HttpTransport("http://local", max_response_bytes=4).complete({}, 1)
    assert oversized.closed
    failed = FakeHttpResponse(b"", fail_read=True)
    with patch("tools.local_agents.transport.urlopen", return_value=failed):
        with pytest.raises(OSError, match="read failed"):
            HttpTransport("http://local").complete({}, 1)
    assert failed.closed


@pytest.mark.parametrize(
    "body",
    [
        b'{"choices":{},"usage":{}}',
        b'{"choices":[],"usage":{}}',
        b'{"choices":[{"finish_reason":"stop","message":{"tool_calls":{}}}],"usage":{}}',
        b'{"choices":[{"finish_reason":"stop","message":{}}],"usage":{"prompt_tokens":true}}',
    ],
)
def test_http_transport_rejects_malformed_response_shapes(body: bytes) -> None:
    response = FakeHttpResponse(body)
    with patch("tools.local_agents.transport.urlopen", return_value=response):
        with pytest.raises(ValueError):
            HttpTransport("http://local").complete({}, 1)
    assert response.closed


def test_cli_rejects_boolean_numeric_config(tmp_path: Path) -> None:
    path = tmp_path / "bad-number.json"
    path.write_text('{"max_run_seconds":true}', encoding="utf-8")
    with pytest.raises(ValueError, match="must be a number"):
        load_config(path)
