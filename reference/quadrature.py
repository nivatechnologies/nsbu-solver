"""Finite composite Simpson quadrature; refinements are empirical, never enclosures."""
from collections.abc import Callable
from mpmath import mp, mpf


def simpson(function: Callable[[mpf],mpf], low: mpf, high: mpf, panels: int) -> mpf:
    """Visit exactly panels+1 nodes, including endpoints; refuse nonfinite samples."""
    if panels < 2 or panels%2 or not mp.isfinite(low) or not mp.isfinite(high) or high < low:
        raise ValueError('Require a finite ordered interval and positive even panels')
    width=(high-low)/panels
    total=mp.mpf(0)
    for index in range(panels+1):
        value=function(low+index*width)
        if not mp.isfinite(value):
            raise ArithmeticError('Nonfinite quadrature sample')
        weight=1 if index in (0,panels) else 2 if index%2==0 else 4
        total+=weight*value
    return total*width/3
