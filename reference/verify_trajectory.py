"""Independent smooth multi-mode from-rest trajectories and temporal refinements."""
from collections.abc import Callable
import json
from mpmath import mp, mpf
from reference.dft import Spectrum, convolution, nonlinear, preflight, retained
from reference.steps import RightHandSide, cm_step, ho_step
from reference.tuples import triple
from tools.json_types import JsonObject

Method = Callable[[Spectrum,mpf,mpf,RightHandSide,mpf],Spectrum]


def spatial_profile() -> Spectrum:
    field: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(4)}
    field[(0,0,0)] = (mp.mpc(1),mp.mpc(-2),mp.mpc(3))
    for mode,component,amplitude in (((0,1,0),0,1),((0,0,1),1,2),((1,0,0),2,3)):
        values = [mp.mpc(0)]*3
        values[component] = amplitude/(2*mp.j)
        field[mode] = triple(values)
        field[triple(-v for v in mode)] = triple(mp.conj(v) for v in values)
    return field


def prescribed_force(profile: Spectrum, quadratic: Spectrum, time: mpf) -> Spectrum:
    """Continuum MMS source; quadratic coefficients use independent convolution."""
    g = mp.sin(13*time)
    derivative = 13*mp.cos(13*time)
    return {k:triple((derivative+4*mp.pi**2*sum(m*m for m in k)*g)*v[c]-g*g*quadratic[k][c]
                    for c in range(3)) for k,v in profile.items()}


def errors(state: Spectrum, profile: Spectrum, time: mpf) -> tuple[mpf,mpf]:
    g = mp.sin(13*time)
    squared = {k:sum((abs(a-g*b)**2 for a,b in zip(v,profile[k])),mp.mpf(0)) for k,v in state.items()}
    l2 = mp.sqrt(sum(squared.values(),mp.mpf(0)))
    h1 = mp.sqrt(sum(((1+4*mp.pi**2*sum(m*m for m in k))*v for k,v in squared.items()),mp.mpf(0)))
    return l2,h1


def trajectory(method: Method, divisor: int, precision: int) -> tuple[Spectrum,JsonObject]:
    reservation = preflight(4,precision,1024**3)
    if divisor < 32 or divisor % 32:
        raise ValueError('Timestep divisor must be a positive multiple of 32')
    with mp.workdps(precision):
        profile = spatial_profile()
        quadratic = convolution(profile)
        state: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in profile}
        dt = mp.mpf(1)/divisor
        def rhs(current: Spectrum, time: mpf) -> Spectrum:
            rotational = nonlinear(current,4)
            force = prescribed_force(profile,quadratic,time)
            return {k:triple(a+b for a,b in zip(rotational[k],force[k])) for k in current}
        for tick in range(divisor//32):
            state = method(state,mp.mpf(tick)/divisor,dt,rhs,mp.mpf(1))
        l2,h1 = errors(state,profile,mp.mpf(1)/32)
        return state,{'divisor':divisor,'precision':precision,'preflight':reservation,
                     'l2_error':mp.nstr(l2,precision),'h1_error':mp.nstr(h1,precision),
                     'steps':divisor//32,'reference_assignments':0}


def run() -> JsonObject:
    reports: list[JsonObject] = []
    for name,method in (('CM',cm_step),('HO',ho_step)):
        rows: list[JsonObject] = []
        norms: list[mpf] = []
        low_states: list[Spectrum] = []
        with mp.workdps(80):
            for divisor in (128,256,512,1024):
                state,report = trajectory(method,divisor,80)
                rows.append(report)
                norms.append(errors(state,spatial_profile(),mp.mpf(1)/32)[1])
                low_states.append(state)
            orders = [mp.log(coarse/fine)/mp.log(2) for coarse,fine in zip(norms,norms[1:])]
            if orders[-1] < mp.mpf('3.5'):
                raise ArithmeticError('Smooth temporal order has not reached the required regime')
        high,high_report = trajectory(method,1024,120)
        with mp.workdps(120):
            change = max(abs(a-b) for k in high for a,b in zip(low_states[-1][k],high[k]))
            if change >= mp.mpf('1e-60'):
                raise ArithmeticError('Short trajectory arithmetic refinement failed')
            reports.append({'method':name,'refinements':rows,'high_precision':high_report,
                            'h1_observed_orders':[mp.nstr(v,20) for v in orders],
                            'max_full_band_arithmetic_change':mp.nstr(change,20)})
    return {'status':'passed','case':'smooth-cyclic-mms-v1','grid':4,'nu':'1','target':'1/32',
            'time_amplitude':'sin(13t)','initial_condition':'exact rest','reports':reports,
            'reference_assignments':0,'scope':'small-grid smooth reference trajectory verification',
            'limitations':['Not a concentrating benchmark window','Not production Rust arithmetic']}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
