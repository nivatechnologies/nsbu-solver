"""Pure time-keyed force sampling and bounded RHS work for the diagnostic pilot."""
from dataclasses import dataclass, field
from mpmath import mp, mpf
from reference.dft import Spectrum, inverse_vector, nonlinear, project
from reference.tuples import triple
from reference.verify_sampling import sample_force
from tools.json_types import JsonObject


@dataclass
class PilotForce:
    n: int
    precision: int
    cap_bytes: int
    dt: mpf
    maximum_evaluations: int
    cache_slots: int
    evaluations: int = 0
    cache: dict[int,Spectrum] = field(default_factory=dict)
    reports: list[JsonObject] = field(default_factory=list)

    def __call__(self, current: Spectrum, time: mpf) -> Spectrum:
        self.evaluations += 1
        tick = self.stage_tick(time)
        velocity = inverse_vector(current,3*self.n//2)
        guard = self.dt*max(sum(abs(v) for v in vector) for vector in velocity)*2*mp.pi*(self.n//2-1)
        if guard > mp.mpf('0.3'):
            raise ArithmeticError('Advective guard failed; refine time in a new diagnostic branch')
        if tick not in self.cache:
            if len(self.cache) >= self.cache_slots:
                raise ArithmeticError('Pure force cache exceeded the preflight slot bound')
            force, report = sample_force(self.n,time,self.precision,self.cap_bytes)
            self.cache[tick] = {k:project(k,v) for k,v in force.items()}
            self.reports.append(report)
        product = nonlinear(current,self.n)
        return {k:triple(a+b for a,b in zip(product[k],self.cache[tick][k])) for k in current}

    def stage_tick(self, time: mpf) -> int:
        if not mp.isfinite(time):
            raise ArithmeticError('Stage time is not finite')
        tick = int(time*2**20)
        if self.evaluations > self.maximum_evaluations or time*2**20 != tick or not 0 <= tick <= 4096:
            raise ArithmeticError('Stage time or nonlinear evaluation bound violated')
        remaining = mp.mpf(8192-tick)/2**20
        if mp.mpf(8192)/2**20-time != remaining:
            raise ArithmeticError('Reference-provider dyadic conversion lost exact remaining time')
        return tick
