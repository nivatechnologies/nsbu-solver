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
    def __init__(self, endpoint: str) -> None:
        base = endpoint.rstrip("/")
        self._endpoint = base + "/chat/completions" if base.endswith("/v1") else base + "/v1/chat/completions"

    def complete(self, request: dict[str, JsonValue], timeout: float) -> ModelReply:
        encoded = json.dumps(request, separators=(",", ":")).encode()
        http_request = Request(
            self._endpoint,
            data=encoded,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urlopen(http_request, timeout=timeout) as response:  # noqa: S310 local URL is operator supplied
            body = json.loads(response.read())
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
