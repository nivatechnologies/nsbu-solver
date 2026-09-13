from __future__ import annotations

import json
import time
from pathlib import Path

import pytest

from tools.local_agents import QueueRunner, RunnerConfig, TaskPacket
from tools.local_agents.contracts import JsonValue
from tools.local_agents.runner import validate_contract
from tools.local_agents.transport import HttpTransport, ModelReply, ToolCall
from tools.local_agents.validators import exact_expected


class FakeTransport:
    def __init__(self, replies: list[str | ModelReply]) -> None:
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


def config(tmp_path: Path, **changes: object) -> RunnerConfig:
    values: dict[str, object] = {
        "endpoint": "http://spark.local:8000",
        "model": "qwen-test",
        "state_path": tmp_path / "state.json",
    }
    values.update(changes)
    return RunnerConfig(**values)  # type: ignore[arg-type]


def runner(
    tmp_path: Path, replies: list[str | ModelReply], **changes: object
) -> tuple[QueueRunner, FakeTransport]:
    fake = FakeTransport(replies)
    queue = QueueRunner(
        config(tmp_path, **changes),
        fake,
        validators={"exact_expected": exact_expected},
        trusted_nonreview_validators=frozenset({"exact_expected"}),
    )
    return queue, fake


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
    task = TaskPacket(**{**task.__dict__, "artifacts": {"fixture": artifact}})
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
    assert fake.requests[1]["messages"][3]["content"] == "fixed"
    assert fake.requests[1]["messages"][3]["role"] == "tool"
    assert fake.timeouts[0] < fake.timeouts[1]


def test_invalid_outputs_get_two_repairs_with_exact_errors_then_fail_closed(tmp_path: Path) -> None:
    replies = ['{}', '{}', '{"sum":"bad"}', '{"sum":"bad"}', 'not json', 'not json']
    queue, fake = runner(tmp_path, replies, max_tool_calls=0, max_repairs=2)
    receipt = queue.run([packet()])[0]
    assert receipt["status"] == "failed"
    assert len(receipt["attempts"]) == 3
    repair_prompt = json.loads(fake.requests[2]["messages"][1]["content"])
    assert repair_prompt["repair_validator_errors"] == ["missing required output field: sum"]
    assert receipt["output"] is None


def test_review_capacity_reserves_inflight_slots_and_defers_without_failure(tmp_path: Path) -> None:
    queue, _ = runner(tmp_path, ['{"sum":5}', '{"sum":5}'], pending_review_cap=1)
    receipts = queue.run([packet("a"), packet("b")])
    assert sorted(r["status"] for r in receipts) == ["deferred_review_backpressure", "validated_candidate"]
    assert queue._family_failures == {"math": 0}


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
    assert "TimeoutError" in receipt["errors"][0]
    assert receipt["errors"][-1] == "server-side cancellation is unknown"


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
    assert HttpTransport("http://spark/v1")._endpoint == "http://spark/v1/chat/completions"
    queue, _ = runner(
        tmp_path,
        [ModelReply('{"sum":5}', finish_reason="length")],
        max_tool_calls=0,
        max_repairs=0,
    )
    receipt = queue.run([packet()])[0]
    assert receipt["status"] == "failed"
    assert "finish_reason=length" in receipt["errors"][0]
