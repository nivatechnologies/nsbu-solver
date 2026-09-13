"""Small deterministic validators available to JSON task packets."""

from __future__ import annotations

from .contracts import JsonValue, Validator


def exact_expected(output: dict[str, JsonValue], inputs: dict[str, JsonValue]) -> list[str]:
    expected = inputs.get("expected_output")
    if output == expected:
        return []
    return [f"output differs from inputs.expected_output: expected {expected!r}, got {output!r}"]


BUILTIN_VALIDATORS: dict[str, Validator] = {"exact_expected": exact_expected}
TRUSTED_NONREVIEW = frozenset(BUILTIN_VALIDATORS)
