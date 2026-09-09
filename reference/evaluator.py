"""Implicit jets and exact v2 force assembly, separate from scalar field formulas."""
from dataclasses import dataclass
from mpmath import mp, mpf
from reference.jets import Jet
from reference.scalar import rational, root


@dataclass(frozen=True)
class Evaluation:
    velocity: tuple[mpf, mpf, mpf]
    pressure_raw: mpf
    force: tuple[mpf, mpf, mpf]
    divergence: mpf
    root_residual_coefficients: tuple[mpf, ...]
    momentum_terms: tuple[tuple[mpf, mpf, mpf, mpf], ...]


def implicit_root(z: Jet, t: Jet) -> tuple[Jet, Jet]:
    scalar = root(z.value, t.value)
    q = Jet.constant(scalar.value, z.degree)
    tau = rational('1/128')-t
    for _ in range(3):
        residual = q-z*z*q**rational('1/4')-tau
        slope = 1-z*z*q**rational('-3/4')/4
        q = q-residual/slope
    return q, q-z*z*q**rational('1/4')-tau


def smooth_step(s: Jet) -> Jet:
    if s.value <= 0:
        return Jet.constant(mp.mpf(0), s.degree)
    if s.value >= 1:
        return Jet.constant(mp.mpf(1), s.degree)
    log_ratio = 1/(1-s)-1/s
    if log_ratio.value <= 0:
        exponential = log_ratio.exp()
        return exponential/(1+exponential)
    return 1/(1+(-log_ratio).exp())


def field_jets(x: Jet, y: Jet, z: Jet, t: Jet) -> tuple[tuple[Jet, Jet, Jet], Jet, Jet]:
    q, residual = implicit_root(z, t)
    eta = z*q**rational('-3/8')
    radial = (x*x+y*y)/(2*q)
    cutoff = (1-smooth_step((x*x+y*y+z*z-rational('9/100'))/rational('54/625')))*smooth_step(512*t)
    gaussian = (-radial).exp()
    g = q**rational('-5/8')*(eta+rational('1/32'))*gaussian/2
    swirl = q**rational('-9/8')*gaussian/4
    potential = cutoff*g
    velocity = (-x*potential.derivative(2)-y*cutoff*swirl,
                -y*potential.derivative(2)+x*cutoff*swirl,
                2*potential+x*potential.derivative(0)+y*potential.derivative(1))
    pressure = -(cutoff*cutoff)*q**rational('-5/4')*(-2*radial).exp()/32
    return velocity, pressure, residual


def evaluate(x: mpf, y: mpf, z: mpf, t: mpf) -> Evaluation:
    if not all(mp.isfinite(v) for v in (x,y,z,t)):
        raise ValueError('Require finite spacetime coordinates')
    coordinates = tuple(Jet.variable(value, axis) for axis,value in enumerate((x,y,z,t)))
    velocity, pressure, residual = field_jets(*coordinates)
    values = tuple(component.value for component in velocity)
    force = []
    terms = []
    for component in range(3):
        temporal = velocity[component].derivative(3).value
        advective = sum(values[j]*velocity[component].derivative(j).value for j in range(3))
        viscous = -sum(velocity[component].derivative(j).derivative(j).value for j in range(3))
        gradient = pressure.derivative(component).value
        terms.append((temporal, advective, viscous, gradient))
        force.append(temporal+advective+viscous+gradient)
    divergence = sum(velocity[j].derivative(j).value for j in range(3))
    return Evaluation(values, pressure.value, tuple(force), divergence,
                      residual.coefficients, tuple(terms))
