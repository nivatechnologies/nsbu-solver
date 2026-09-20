"""Typed, runtime-validated JSON object boundary for every packet reader.

``json.loads`` returns an untyped value, so every packet reader crosses the
JSON boundary through this module instead: the parse itself is the standard
library's, but each parsed object is rebuilt through an ``object_pairs_hook``
whose declared parameter types fix keys as ``str`` and leave values as
``object``.  Nothing here trusts the bytes: a top-level non-object raises
:class:`JsonNotObject` (never a silent Any), and every nested
value stays ``object`` until the consuming module's shape validator
(for example ``h32_contract.validate_plan_shape``) has runtime-checked it.
Malformed JSON surfaces as the same ``json.JSONDecodeError`` /
``UnicodeDecodeError`` that ``json.loads`` would have raised.
"""

from __future__ import annotations

import json


class JsonNotObject(ValueError):
    """The top-level JSON value parsed fine but is not an object."""


def load_json_object(text: str) -> dict[str, object]:
    """Parse ``text`` as a JSON object; nested values remain ``object``.

    Raises ``json.JSONDecodeError`` for malformed JSON and
    :class:`JsonNotObject` (a ``ValueError``) when the top-level value is
    not an object — mirroring a ``json.loads`` + ``isinstance(_, dict)``
    check exactly.
    """
    objects: list[dict[str, object]] = []

    def pairs_hook(pairs: list[tuple[str, object]]) -> dict[str, object]:
        obj: dict[str, object] = {}
        for key, value in pairs:
            obj[key] = value
        objects.append(obj)
        return obj

    if not isinstance(json.loads(text, object_pairs_hook=pairs_hook), dict) \
            or not objects:
        raise JsonNotObject("json_not_object")
    return objects[-1]


def load_unique_json_object(text: str, label: str) -> dict[str, object]:
    """Like :func:`load_json_object`, but any object with a duplicated key
    raises ``ValueError(duplicate_key(label,key))`` mid-parse."""
    objects: list[dict[str, object]] = []

    def reject_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
        seen: set[str] = set()
        for key, _value in pairs:
            if key in seen:
                raise ValueError(f"duplicate_key({label},{key})")
            seen.add(key)
        obj: dict[str, object] = {}
        for key, value in pairs:
            obj[key] = value
        objects.append(obj)
        return obj

    if not isinstance(json.loads(text, object_pairs_hook=reject_pairs), dict) \
            or not objects:
        raise ValueError(f"json_not_object({label})")
    return objects[-1]
