"""Bounded independent full/two-half CyclicSine trajectories for current-grid arithmetic studies."""
from collections.abc import Callable
from dataclasses import dataclass
from mpmath import mp, mpf
from reference.cyclic.bits import FixedForcing
from reference.cyclic.profile import projected_force
from reference.dft import Spectrum, nonlinear, preflight, retained
from reference.steps import cm_step, ho_step
from reference.tuples import triple
from tools.json_types import JsonObject

Prescribed = Callable[[mpf], Spectrum]


@dataclass(frozen=True)
class StudyPlan:
    """Fixed eight-macro-step unit-cube profile; every run starts at exact rest."""
    n: int
    precision: int
    method: str
    cap_bytes: int
    forcing: FixedForcing | None = None

    def reservation(self, fixture: bool = False) -> JsonObject:
        """Admit the reference grids, precision, complete stage schedule and retained fixture storage."""
        calls = self.maximum_calls()
        if self.forcing is not None and self.forcing.n != self.n:
            raise ValueError('Fixed force grid differs from the evolved grid')
        base = preflight(self.n, self.precision, self.cap_bytes)
        extra = 768*1024**2 if self.forcing is not None or fixture else 0
        total = int(str(base['reserved_bytes'])) + extra
        if total > self.cap_bytes:
            raise ValueError('Combined reference and force fixture reservation exceeds cap')
        return {'base': base, 'force_fixture_reservation': extra, 'reserved_bytes': total,
                'cap_bytes': self.cap_bytes, 'maximum_rhs_evaluations': calls,
                'classification': 'conservative allocating Python reference reservation, not a hard allocator bound'}

    def maximum_calls(self) -> int:
        """Every full and two-half CM/HO stage is counted, including the unused coarse proposal."""
        if self.method not in ('CM', 'HO'):
            raise ValueError('CyclicSine study requires CM or HO')
        return 8 * (12 if self.method == 'CM' else 15)


@dataclass
class StudyRhs:
    """Independently sampled prescribed data and direct-DFT nonlinearity with a finite call cap."""
    n: int
    maximum: int
    force: Prescribed
    calls: int = 0

    def __call__(self, current: Spectrum, time: mpf) -> Spectrum:
        if self.calls >= self.maximum:
            raise ArithmeticError('CyclicSine reference RHS allowance exhausted')
        self.calls += 1
        scaled = time * 65536
        if not mp.isfinite(scaled) or not 0 <= scaled <= 128:
            raise ArithmeticError('CyclicSine stage outside the exact declared window')
        tick = int(scaled)
        if scaled != tick or tick % 4:
            raise ArithmeticError('CyclicSine stage is not an exact quarter-step clock')
        if current.keys() != set(retained(self.n)):
            raise ArithmeticError('CyclicSine current state has the wrong retained band')
        product = nonlinear(current, self.n)
        force = self.force(time)
        if product.keys() != force.keys():
            raise ArithmeticError('CyclicSine prescribed force has the wrong full retained band')
        return {k: triple(a+b for a, b in zip(product[k], force[k])) for k in current}


def trajectory(plan: StudyPlan) -> tuple[Spectrum, JsonObject]:
    """Evolve each fine proposal independently from rest; never assign an analytical reference."""
    reservation = plan.reservation()
    with mp.workdps(plan.precision):
        forcing: Prescribed = plan.forcing.evaluate if plan.forcing is not None else lambda t: projected_force(plan.n, t)
        rhs = StudyRhs(plan.n, plan.maximum_calls(), forcing)
        method = cm_step if plan.method == 'CM' else ho_step
        state: Spectrum = {k: (mp.mpc(0), mp.mpc(0), mp.mpc(0)) for k in retained(plan.n)}
        dt = mp.mpf(1)/4096
        differences: list[str] = []
        for tick in range(8):
            time = tick*dt
            coarse = method(state, time, dt, rhs, mp.mpf(1))
            middle = method(state, time, dt/2, rhs, mp.mpf(1))
            fine = method(middle, time+dt/2, dt/2, rhs, mp.mpf(1))
            delta = maximum_difference(fine, coarse)
            differences.append(mp.nstr(delta, plan.precision))
            state = fine
        if rhs.calls != plan.maximum_calls():
            raise ArithmeticError('CyclicSine reference did not execute every declared stage')
        return state, report(plan, reservation, rhs.calls, differences)


def maximum_difference(left: Spectrum, right: Spectrum) -> mpf:
    """Complete retained-band coefficient maximum; no common-band truncation or alignment."""
    if not left or left.keys() != right.keys():
        raise ValueError('Comparison requires equal nonempty retained bands')
    values = [abs(a-b) for k in left for a, b in zip(left[k], right[k])]
    if not all(mp.isfinite(value) for value in values):
        raise ArithmeticError('Nonfinite CyclicSine reference state')
    return max(values)


def report(plan: StudyPlan, reservation: JsonObject, calls: int, differences: list[str]) -> JsonObject:
    """Record actual work and precision/input identity without promoting diagnostic status."""
    return {'case': 'CyclicSine' if plan.forcing is None else 'fixed-stage-arithmetic-fixture',
            'nominal_comparison_case': 'CyclicSine', 'grid': plan.n, 'precision': plan.precision, 'method': plan.method,
            'endpoint': '1/512', 'macro_step': '1/4096', 'macro_steps': 8,
            'fine_steps_committed': 16, 'rhs_calls': calls, 'reference_assignments': 0,
            'preflight': reservation, 'full_step_two_half_max_differences': differences,
            'fixed_force_sha256': plan.forcing.sha256 if plan.forcing is not None else None,
            'status': 'diagnostic-completed', 'accepted_pde_windows': 0,
            'scope': 'independent same-grid smooth arithmetic study; no concentrating qualification'}
