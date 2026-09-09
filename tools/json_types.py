"""Typed JSON boundaries; shape errors remain explicit input errors."""
from collections.abc import Mapping, Sequence
import json
from typing import cast


type Json = None | bool | int | float | str | Sequence[Json] | Mapping[str, Json]
type JsonObject = dict[str, Json]


def decode(text: str) -> Json:
    # The stdlib decoder returns only the recursively declared JSON value types.
    return cast(Json, json.loads(text))


def object_value(value: Json) -> Mapping[str, Json]:
    if not isinstance(value, Mapping):
        raise ValueError('Expected a JSON object')
    return value


def array_value(value: Json) -> Sequence[Json]:
    if not isinstance(value, Sequence) or isinstance(value, str):
        raise ValueError('Expected a JSON array')
    return value


def string_value(value: Json) -> str:
    if not isinstance(value, str):
        raise ValueError('Expected a JSON string')
    return value
