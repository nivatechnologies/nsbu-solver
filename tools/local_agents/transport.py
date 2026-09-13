"""Small OpenAI-compatible HTTP transport with no credential or cloud fallback."""

from __future__ import annotations

import json
from collections.abc import Sequence
from dataclasses import dataclass
from http.client import HTTPResponse
from typing import Protocol, cast
from urllib.request import Request, urlopen

from .contracts import JsonValue
from tools.json_types import Json, decode, object_value, string_value


@dataclass(frozen=True)
class ToolCall:
    call_id: str
    name: str
    arguments: str


@dataclass(frozen=True)
class ModelReply:
    content: str
    prompt_tokens: int = 0
    completion_tokens: int = 0
    reasoning_tokens: int = 0
    tool_calls: tuple[ToolCall, ...] = ()
    finish_reason: str = "stop"


class Transport(Protocol):
    def complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply: ...


class HttpTransport:
    def __init__(self, endpoint: str, max_response_bytes: int = 200_000) -> None:
        base = endpoint.rstrip("/")
        self._endpoint = base + "/chat/completions" if base.endswith("/v1") else base + "/v1/chat/completions"
        self._max_response_bytes = max_response_bytes

    @property
    def endpoint(self) -> str:
        return self._endpoint

    def complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply:
        encoded = json.dumps(request, separators=(",", ":")).encode()
        http_request = Request(
            self._endpoint,
            data=encoded,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        response = cast(  # noqa: S310 local URL is operator supplied
            HTTPResponse, urlopen(http_request, timeout=timeout)
        )
        try:
            encoded_body = response.read(self._max_response_bytes + 1)
        finally:
            response.close()
        if len(encoded_body) > self._max_response_bytes:
            raise ValueError("HTTP model response exceeds byte limit")
        body = object_value(decode(encoded_body.decode()))
        choices = body.get("choices")
        if not isinstance(choices, Sequence) or isinstance(choices, str) or not choices:
            raise ValueError("model response choices must be a nonempty array")
        choice = object_value(choices[0])
        message = object_value(choice.get("message"))
        calls = _tool_calls(message.get("tool_calls", []))
        usage = object_value(body.get("usage", {}))
        details = object_value(usage.get("completion_tokens_details", {}))
        return ModelReply(
            content=_optional_string(message.get("content")),
            prompt_tokens=_integer(usage.get("prompt_tokens", 0)),
            completion_tokens=_integer(usage.get("completion_tokens", 0)),
            reasoning_tokens=_integer(details.get("reasoning_tokens", 0)),
            tool_calls=calls,
            finish_reason=_optional_string(choice.get("finish_reason")),
        )


def _tool_calls(value: Json) -> tuple[ToolCall, ...]:
    if not isinstance(value, Sequence) or isinstance(value, str):
        raise ValueError("tool_calls must be an array")
    result: list[ToolCall] = []
    for raw in value:
        call = object_value(raw)
        function = object_value(call.get("function"))
        result.append(
            ToolCall(
                string_value(call.get("id")),
                string_value(function.get("name")),
                string_value(function.get("arguments")),
            )
        )
    return tuple(result)


def _integer(value: Json) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError("token count must be an integer")
    return value


def _optional_string(value: Json) -> str:
    if value is None:
        return ""
    return string_value(value)
