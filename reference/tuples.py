"""Dimension-checked tuple construction at the numerical API boundaries."""
from collections.abc import Iterable


def triple[T](values: Iterable[T]) -> tuple[T, T, T]:
    a, b, c = values
    return a, b, c


def quadruple[T](values: Iterable[T]) -> tuple[T, T, T, T]:
    a, b, c, d = values
    return a, b, c, d
