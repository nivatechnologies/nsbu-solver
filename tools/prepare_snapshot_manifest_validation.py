"""Bounded JSON loading and closed-contract scalar guards for snapshot preparation.

Every refusal is a PreparationError raised before any staging happens.
Standard library only.
"""

from __future__ import annotations

from collections.abc import Mapping
import json
import math
from pathlib import Path
from typing import BinaryIO, cast, Final

from tools.json_types import Json, JsonObject, object_value

MAX_IDENTITY_BYTES: Final = 16 * 1024
MAX_MANIFEST_BYTES: Final = 64 * 1024
MAX_RECORD_BYTES: Final = 2 * 4096
MAX_PLAN_BYTES: Final = 1024 * 1024
IO_BUFFER_BYTES: Final = 1024 * 1024
U128_LIMIT: Final = 1 << 128
TOLERANCE_COUNT: Final = 2
MAX_JSON_DEPTH: Final = 32


class PreparationError(ValueError):
    """A concrete refusal; refusals raised before staging publish nothing."""


class PublicationUncertainError(PreparationError):
    """A failure at or after the destination link; output may be published."""


def _reject_constant(value: str) -> Json:
    raise PreparationError(f"invalid JSON constant {value}")


def _reject_duplicates(pairs: list[tuple[str, Json]]) -> JsonObject:
    result: JsonObject = {}
    for key, value in pairs:
        if key in result:
            raise PreparationError(f"duplicate JSON keys: {key}")
        result[key] = value
    return result


def _require_depth(value: Json, label: str) -> None:
    stack: list[tuple[Json, int]] = [(value, 1)]
    while stack:
        item, depth = stack.pop()
        if not isinstance(item, (dict, list)):
            continue
        if depth > MAX_JSON_DEPTH:
            raise PreparationError(f"{label} nests deeper than {MAX_JSON_DEPTH} levels")
        children = item.values() if isinstance(item, dict) else item
        stack.extend((child, depth + 1) for child in children)


def loads_checked(raw: bytes, label: str) -> Mapping[str, Json]:
    try:
        decoded = cast(Json, json.loads(raw.decode("utf-8"), object_pairs_hook=_reject_duplicates,
                                        parse_constant=_reject_constant))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PreparationError(f"{label} is invalid JSON: {error}") from error
    except RecursionError as error:
        raise PreparationError(f"{label} nests deeper than {MAX_JSON_DEPTH} levels") from error
    values = object_value(decoded)
    _require_depth(values, label)
    require_finite(values, label)
    return values


def read_capped(handle: BinaryIO, limit: int) -> bytes:
    chunks: list[bytes] = []
    total = 0
    while total <= limit:
        chunk = handle.read(min(IO_BUFFER_BYTES, limit + 1 - total))
        if not chunk:
            break
        chunks.append(chunk)
        total += len(chunk)
    return b"".join(chunks)


def load_json_bounded(path: Path, limit: int, label: str) -> Mapping[str, Json]:
    with open(path, "rb") as handle:
        raw = read_capped(handle, limit)
    if len(raw) > limit:
        raise PreparationError(f"{label} exceeds {limit} byte bound")
    return loads_checked(raw, label)


def require_finite(value: Json, label: str) -> None:
    if isinstance(value, float) and not math.isfinite(value):
        raise PreparationError(f"{label} contains a non-finite number")
    if isinstance(value, dict):
        for item in value.values():
            require_finite(item, label)
    elif isinstance(value, list):
        for item in value:
            require_finite(item, label)


def string_value(value: Json, label: str) -> str:
    if not isinstance(value, str):
        raise PreparationError(f"{label} must be a string")
    return value


def count_value(value: Json, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value < U128_LIMIT:
        raise PreparationError(f"{label} must be a u128-range non-negative integer")
    return value


def flag_value(value: Json, label: str) -> bool:
    if not isinstance(value, bool):
        raise PreparationError(f"{label} must be a boolean")
    return value


def positive_value(value: Json, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or \
            not math.isfinite(float(value)) or float(value) <= 0.0:
        raise PreparationError(f"{label} must be a positive finite number")
    return float(value)


def hex_value(value: Json, length: int, label: str) -> str:
    text = string_value(value, label)
    if len(text) != length or any(byte not in "0123456789abcdefABCDEF" for byte in text):
        raise PreparationError(f"{label} is not {length} hex digits")
    return text.lower()


def grid_value(value: Json, label: str) -> list[int]:
    if not isinstance(value, list) or len(value) != 3:
        raise PreparationError(f"{label} must hold three grid dimensions")
    grid = [count_value(item, label) for item in value]
    if any(size < 2 or size % 2 != 0 for size in grid):
        raise PreparationError(f"{label} must hold even dimensions of at least two")
    return grid


def exact_keys(values: Mapping[str, Json], required: frozenset[str], optional: frozenset[str],
               label: str) -> None:
    keys = set(values)
    if required - keys:
        raise PreparationError(f"{label} omits {sorted(required - keys)}")
    if keys - required - optional:
        raise PreparationError(f"{label} has unknown fields {sorted(keys - required - optional)}")


def identity_field(identity: str, key: str) -> list[str]:
    values: list[str] = []
    for field in identity.split(";"):
        name, separator, value = field.partition("=")
        if separator and name == key:
            values.append(value)
    return values


def relative_name(value: str, label: str) -> str:
    candidate = Path(value)
    if not value or candidate.is_absolute() or candidate.name != value:
        raise PreparationError(f"{label} must be a plain relative file name")
    return value


def tolerance_list(value: Json, label: str) -> list[float]:
    if not isinstance(value, list) or len(value) != TOLERANCE_COUNT:
        raise PreparationError(f"{label} must hold exactly {TOLERANCE_COUNT} positive values")
    return [positive_value(item, label) for item in value]
