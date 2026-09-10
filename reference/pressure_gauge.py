"""Independent periodic pressure mean for the frozen unit-cube exact-v2 reference.

The raw reference has spherical support R_out=21/50 < 1/2 and is axisymmetric.
With w=x*x+y*y, dx dy=pi dw after angular integration. Only reference quadrature
uses this reduction: the PDE state still evolves all three Cartesian components.
"""
from dataclasses import dataclass
from fractions import Fraction
from mpmath import mp, mpf
from reference.quadrature import simpson
from reference.scalar import rational, root, step
from tools.json_types import JsonObject

CAP_BYTES=64*1024**2


@dataclass(frozen=True)
class MeanPlan:
    """Fixed exact time, precision, panel counts and finite work/storage admission."""
    time: Fraction
    axial_panels: int
    radial_panels: int
    precision: int=80
    root_iterations: int=2048
    cap_bytes: int=CAP_BYTES

    def __post_init__(self) -> None:
        if not 0 <= self.time < Fraction(1,128):
            raise ValueError('Require 0 <= exact time < T_star')
        if self.precision not in (80,120) or not 1 <= self.root_iterations <= 2048:
            raise ValueError('Unsupported precision or root-work cap')
        if any(n<2 or n>1024 or n%2 for n in (self.axial_panels,self.radial_panels)):
            raise ValueError('Each Simpson panel count must be even and within [2,1024]')
        if self.reserved_bytes > self.cap_bytes:
            raise ValueError('Pressure mean exceeds its conservative memory reservation')

    @property
    def reserved_bytes(self) -> int:
        """Streaming scalar temporaries plus interpreter allowance; no hard allocator guarantee."""
        return 32*1024**2+self.precision*65536

    @property
    def evaluations(self) -> int:
        """Exact collar sample count; each axial slab also performs one bounded root solve."""
        return (self.axial_panels+1)*(self.radial_panels+1)

    def preflight(self) -> JsonObject:
        """No numerical root or quadrature evaluation occurs during admission."""
        return {'case':'similarity-mms-v2','time':str(self.time),'precision':self.precision,
                'axial_panels':self.axial_panels,'radial_squared_panels':self.radial_panels,
                'collar_pressure_evaluations':self.evaluations,'root_solves':self.axial_panels+1,
                'maximum_root_iterations':(self.axial_panels+1)*self.root_iterations,
                'reserved_bytes':self.reserved_bytes,'cap_bytes':self.cap_bytes,
                'classification':'EmpiricalQuadrature; conservative Python planning',
                'domain':'centered unit periodic cube','support_radius':'21/50','radial_rule':'exact Gaussian core plus Simpson cutoff collar',
                'accepted_pde_windows':0}


def radial_pressure(w: mpf, z: mpf, time: mpf, q: mpf) -> mpf:
    """Raw kinematic pressure with w=r^2 and the independently solved positive q(z,t)."""
    inner2=rational('9/100')
    cutoff=(1-step((w+z*z-inner2)/(rational('441/2500')-inner2)))*step(512*time)
    return -cutoff*cutoff*q**rational('-5/4')*mp.exp(-w/q)/32


def slab(z: mpf, time: mpf, plan: MeanPlan) -> mpf:
    """One complete radial-squared interval at fixed z; cache only its single scalar root."""
    q=root(z,time,plan.root_iterations).value
    low=max(mp.mpf(0),rational('9/100')-z*z)
    high=max(mp.mpf(0),rational('441/2500')-z*z)
    # The cutoff is exactly flat on [0, low]. Integrate its Gaussian analytically;
    # this removes the narrowing core from numerical radial quadrature.
    core=-step(512*time)**2*q**rational('-1/4')*(1-mp.exp(-low/q))/32
    return core+simpson(lambda w: radial_pressure(w,z,time,q),low,high,plan.radial_panels)


@dataclass(frozen=True)
class PressureMean:
    """A computed empirical global mean, bound to its exact admitted reference request."""
    plan: MeanPlan
    value: mpf

    def subtract(self, raw_pressure: mpf) -> mpf:
        """Subtract this global gauge at its declared precision; never refit it on a region."""
        if not mp.isfinite(raw_pressure):
            raise ValueError('Require finite raw pressure')
        with mp.workdps(self.plan.precision):
            return raw_pressure-self.value


def evaluate(plan: MeanPlan) -> PressureMean:
    """Integrate the complete support, both signs of z, then divide by unit-cube volume.

    The result must be refined in both panel directions and arithmetic precision.
    No error bound, continuum mean certificate or PDE acceptance is returned.
    """
    with mp.workdps(plan.precision):
        time=mp.mpf(plan.time.numerator)/plan.time.denominator
        radius=rational('21/50')
        value=mp.pi*simpson(lambda z: slab(z,time,plan),-radius,radius,plan.axial_panels)
    return PressureMean(plan,value)
