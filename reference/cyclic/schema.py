"""Strict typed JSON admission for reproducible numerical inputs."""
from collections.abc import Sequence
import json
from typing import cast
from tools.json_types import Json, JsonObject


def unique_object(pairs: Sequence[tuple[str, Json]]) -> JsonObject:
    """A repeated property cannot silently select one of two conflicting scientific inputs."""
    result: JsonObject = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'Duplicate JSON property: {key}')
        result[key] = value
    return result


def nonfinite_constant(value: str) -> Json:
    """JSON's nonstandard NaN/Infinity extensions are not accepted numerical data."""
    raise ValueError(f'Nonfinite JSON constant: {value}')


def decode_fixture(data: bytes) -> Json:
    """Decode bounded bytes; depth errors remain explicit input refusals."""
    bounded_nesting(data)
    try:
        return cast(Json, json.loads(data.decode('utf-8'), object_pairs_hook=unique_object,
                                     parse_constant=nonfinite_constant))
    except RecursionError as error:
        raise ValueError('JSON nesting exceeds parser capacity') from error


def bounded_nesting(data: bytes) -> None:
    """Admit at most 32 JSON container levels before the allocating parser runs."""
    depth = 0
    quoted = False
    escaped = False
    for byte in data:
        if quoted:
            if escaped:
                escaped = False
            elif byte == 92:
                escaped = True
            elif byte == 34:
                quoted = False
        elif byte == 34:
            quoted = True
        elif byte in (91,123):
            depth += 1
            if depth > 32:
                raise ValueError('JSON nesting exceeds 32 container levels')
        elif byte in (93,125):
            depth -= 1
