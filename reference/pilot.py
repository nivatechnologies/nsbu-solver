"""Bounded small-grid exact-v2 diagnostics, evolved independently from rest."""
import json
from mpmath import mp, mpf
from reference.dft import Spectrum, Vector, forward_vector, grid, preflight, retained
from reference.pilot_force import PilotForce
from reference.scalar import fields
from reference.steps import cm_step, ho_step
from reference.tuples import triple
from reference.verify_sampling import difference
from tools.json_types import JsonObject


def reservation(n: int, divisor: int, precision: int, method: str, cap: int) -> tuple[JsonObject,int,int]:
    base = preflight(n,precision,cap)
    if divisor not in (8192,16384,32768) or method not in ('CM','HO'):
        raise ValueError('Pilot requires a supported method and bounded timestep divisor')
    cache_bytes = (2*(divisor//256)+1)*(n-1)**3*3*4096
    total = int(str(base['reserved_bytes']))+cache_bytes
    if total > cap:
        raise ValueError('Pilot force-cache reservation exceeds the resource cap')
    return base,cache_bytes,total


def tracking(state: Spectrum, n: int, time: mpf) -> JsonObject:
    """Reference is evaluated after integration; never passed into a step method."""
    samples: list[Vector] = []
    for point in grid(n):
        coordinates = triple(mp.mpf(j if j < n//2 else j-n)/n for j in point)
        velocity, _ = fields(*coordinates,time)
        samples.append(triple(mp.mpc(v) for v in velocity))
    return difference(forward_vector(samples,n,n),state)


def pilot(n: int, divisor: int, precision: int, method_name: str, cap_bytes: int) -> JsonObject:
    """Fixed-step allocating diagnostic; no adaptive acceptance or PDE qualification."""
    base,cache_bytes,total = reservation(n,divisor,precision,method_name,cap_bytes)
    steps = divisor//256
    with mp.workdps(precision):
        state: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(n)}
        step_ticks = 2**20//divisor
        dt = mp.mpf(step_ticks)/2**20
        rhs = PilotForce(n,precision,cap_bytes,dt,(4 if method_name == 'CM' else 5)*steps,2*steps+1)
        method = cm_step if method_name == 'CM' else ho_step
        accepted_ticks = 0
        failure = ''
        for index in range(steps):
            try:
                candidate = method(state,mp.mpf(accepted_ticks)/2**20,dt,rhs,mp.mpf(1))
                if not all(mp.isfinite(v) for values in candidate.values() for v in values):
                    raise ArithmeticError('Non-finite candidate refused')
            except (ArithmeticError,ValueError) as error:
                failure = str(error)
                break
            state = candidate
            accepted_ticks = (index+1)*step_ticks
        return report(state,rhs,base,cache_bytes,total,divisor,method_name,accepted_ticks,failure)


def report(state: Spectrum, rhs: PilotForce, base: JsonObject, cache_bytes: int, total: int,
           divisor: int, method: str, ticks: int, failure: str) -> JsonObject:
    result: JsonObject = {
        'status':'diagnostic-stopped' if failure else 'diagnostic-completed',
        'case':'similarity-mms-v2','grid':rhs.n,'precision':rhs.precision,'method':method,
        'divisor':divisor,'target':'1/256','initial_condition':'exact rest',
        'accepted_ticks':ticks,'remaining_ticks':8192-ticks,'quantum_exponent':-20,
        'steps_completed':ticks//(2**20//divisor),'rhs_evaluations':rhs.evaluations,
        'maximum_rhs_evaluations':rhs.maximum_evaluations,'reference_assignments':0,
        'preflight':base,'force_cache_bytes':cache_bytes,'total_reserved_bytes':total,
        'force_sampling':rhs.reports,'failure':failure,
        'state':{str(k):[[mp.nstr(v.real,rhs.precision),mp.nstr(v.imag,rhs.precision)] for v in values]
                 for k,values in state.items()},
        'accepted_pde_windows':0,'limitations':['Spatial/force sampling unresolved',
        'No refinement-family qualification','Retained-band reference tracking only',
        'Allocating Python diagnostic, not a bounded production allocator',
        'No rigorous slab-wide error enclosure']}
    try:
        result['tracking_difference'] = tracking(state,rhs.n,mp.mpf(ticks)/2**20)
    except (ArithmeticError,ValueError) as error:
        result['reference_failure'] = str(error)
    return result


if __name__ == '__main__':
    print(json.dumps(pilot(4,8192,80,'CM',1024**3),indent=2))
