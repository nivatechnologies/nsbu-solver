"""Small OpenAI-compatible HTTP transport with no credential or cloud fallback."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Protocol
from urllib.request import Request, urlopen

from .contracts import JsonValue


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

    def complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply:
        encoded = json.dumps(request, separators=(",", ":")).encode()
        http_request = Request(
            self._endpoint,
            data=encoded,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urlopen(http_request, timeout=timeout) as response:  # noqa: S310 local URL is operator supplied
            encoded_body = response.read(self._max_response_bytes + 1)
        if len(encoded_body) > self._max_response_bytes:
            raise ValueError("HTTP model response exceeds byte limit")
        body = json.loads(encoded_body)
        message = body["choices"][0]["message"]
        calls = tuple(
            ToolCall(call["id"], call["function"]["name"], call["function"]["arguments"])
            for call in message.get("tool_calls", [])
        )
        usage = body.get("usage", {})
        return ModelReply(
            content=message.get("content") or "",
            prompt_tokens=int(usage.get("prompt_tokens", 0)),
            completion_tokens=int(usage.get("completion_tokens", 0)),
            reasoning_tokens=int(usage.get("completion_tokens_details", {}).get("reasoning_tokens", 0)),
            tool_calls=calls,
            finish_reason=str(body["choices"][0].get("finish_reason", "")),
        )
