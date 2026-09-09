"""Exact v2 scalar geometry, bounded root solve and independent field formulas.

All callers select precision with mpmath.workdps; run precision studies sequentially.
See COMPLETE_DESIGN.md sections 3.1–3.3. No integrated state is accepted here.
"""
from dataclasses import dataclass
from fractions import Fraction
from mpmath import mp, mpf


def rational(value: str) -> mpf:
    """Convert rational text without passing through binary64."""
    fraction = Fraction(value)
    return mp.mpf(fraction.numerator) / fraction.denominator


class CoordinateUnresolved(ValueError):
    """The configured root-work limit cannot establish the requested accuracy."""


@dataclass(frozen=True)
class Root:
    value: mpf
    lower: mpf
    upper: mpf
    residual: mpf
    iterations: int


def root(z: mpf, t: mpf, max_iterations: int = 2048) -> Root:
    """Safeguarded Newton solve with a fixed cap and residual error estimate."""
    terminal = rational('1/128')
    if not mp.isfinite(z) or not mp.isfinite(t) or not 0 <= t < terminal:
        raise ValueError('Require finite z and 0 <= t < T_star')
    if max_iterations < 1:
        raise ValueError('Require a positive iteration cap')
    tau = terminal - t
    if z == 0:
        return Root(tau, tau, tau, mp.mpf(0), 0)
    lower = max(tau, abs(z) ** rational('8/3'))
    upper = 4 * lower
    q = (lower + upper) / 2
    for iteration in range(1, max_iterations + 1):
        residual = q - z*z*q**rational('1/4') - tau
        if abs(residual) <= 16 * mp.eps * tau:
            return Root(q, lower, upper, residual, iteration)
        if residual < 0:
            lower = q
        else:
            upper = q
        slope = 1 - z*z*q**rational('-3/4') / 4
        candidate = q - residual / slope
        q = candidate if lower < candidate < upper else (lower + upper) / 2
    raise CoordinateUnresolved('Root iteration cap exhausted')


def step(s: mpf) -> mpf:
    """Smooth step in overflow-resistant logistic form, including exact flats."""
    if s <= 0:
        return mp.mpf(0)
    if s >= 1:
        return mp.mpf(1)
    log_ratio = 1/(1-s) - 1/s
    if log_ratio <= 0:
        exponential = mp.exp(log_ratio)
        return exponential / (1 + exponential)
    return 1 / (1 + mp.exp(-log_ratio))


def step_derivative(s: mpf) -> mpf:
    if s <= 0 or s >= 1:
        return mp.mpf(0)
    value = step(s)
    return value * (1-value) * (1/s**2 + 1/(1-s)**2)


def fields(x: mpf, y: mpf, z: mpf, t: mpf) -> tuple[tuple[mpf, mpf, mpf], mpf]:
    """Independent scalar velocity and raw pressure, using explicit first derivatives."""
    if not mp.isfinite(x) or not mp.isfinite(y):
        raise ValueError('Require finite spatial coordinates')
    q = root(z, t).value
    inner2 = rational('9/100')
    width = rational('441/2500') - inner2
    radius2 = x*x + y*y + z*z
    s = (radius2-inner2) / width
    ramp = step(512*t)
    cutoff = (1-step(s)) * ramp
    cutoff_r2 = -step_derivative(s) * ramp / width
    eta = z*q**rational('-3/8')
    radial = (x*x+y*y)/(2*q)
    gaussian = mp.exp(-radial)
    amplitude = q**rational('-5/8') * gaussian / 2
    g = amplitude * (eta+rational('1/32'))
    swirl = q**rational('-9/8') * gaussian / 4
    qz = 2*z*q**rational('1/4') / (1-eta*eta/4)
    etaz = q**rational('-3/8') - rational('3/8')*eta*qz/q
    gz = amplitude*(etaz+(eta+rational('1/32'))*(radial-rational('5/8'))*qz/q)
    hz = 2*z*cutoff_r2*g + cutoff*gz
    radial_h = 2*cutoff_r2*g - cutoff*g/q
    velocity = (-x*hz-y*cutoff*swirl, -y*hz+x*cutoff*swirl,
                2*cutoff*g+(x*x+y*y)*radial_h)
    pressure = -cutoff**2*q**rational('-5/4')*mp.exp(-2*radial)/32
    return velocity, pressure
