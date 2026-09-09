"""Mathematical interior masks and explicitly empirical volume quadrature."""
from dataclasses import dataclass
from mpmath import mp, mpf


@dataclass(frozen=True)
class Coverage:
    status: str
    fraction: mpf
    refinement_change: mpf
    panels: int
    classification: str = 'EmpiricalQuadrature'


def radial_limit(eta: mpf, tau: mpf, radius: mpf) -> tuple[mpf,mpf]:
    q = tau/(1-eta*eta)
    z = eta*q**(mp.mpf(3)/8)
    weight = q**(mp.mpf(11)/8)*(1-eta*eta/4)/(1-eta*eta)
    return (radius*radius-z*z)/(2*q),weight


def integral(tau: mpf, low: mpf, high: mpf, radius: mpf, panels: int) -> mpf:
    """Composite Simpson rule on the complete declared eta interval."""
    numerator = mp.mpf(0)
    denominator = mp.mpf(0)
    for index in range(panels+1):
        eta = -mp.mpf(1)/2+mp.mpf(index)/panels
        limit,weight = radial_limit(eta,tau,radius)
        coefficient = 1 if index in (0,panels) else 2 if index%2 == 0 else 4
        numerator += coefficient*weight*min(high-low,max(mp.mpf(0),limit-low))
        denominator += coefficient*weight*(high-low)
    return numerator/denominator


def coverage(tau: mpf, low: mpf, high: mpf, radius: mpf | str = '3/10', panels: int = 256) -> Coverage:
    radius = mp.mpf(radius)
    if not all(mp.isfinite(value) for value in (tau,low,high,radius)):
        raise ValueError('Coverage inputs must be finite')
    if tau <= 0 or low < 0 or high <= low or radius <= 0:
        raise ValueError('Invalid coverage geometry')
    if panels < 2 or panels%2:
        raise ValueError('Simpson panel count must be positive and even')
    coarse = integral(tau,low,high,radius,panels)
    fine = integral(tau,low,high,radius,2*panels)
    # The maximum radial limit occurs at eta=0. This is a geometric emptiness
    # decision, never inferred from underflow or a missed quadrature sample.
    empty = radius*radius/(2*tau) <= low
    return Coverage('RegionEmpty' if empty else 'Nonempty',fine,abs(fine-coarse),2*panels)
