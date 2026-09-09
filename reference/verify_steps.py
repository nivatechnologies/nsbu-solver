"""Full-step/two-half-step direct-DFT arithmetic fixtures from exact rest."""
from dataclasses import dataclass
import json
from mpmath import mp, mpf
from reference.dft import Spectrum, nonlinear, preflight, retained
from reference.steps import RightHandSide, cm_step, ho_step
from reference.tuples import triple
from tools.json_types import JsonObject


@dataclass(frozen=True)
class Study:
    precision: int
    method: str
    preflight: JsonObject
    full_step: Spectrum
    two_half_steps: Spectrum
    raw_local_discrepancy: mpf


def force(n: int, time: mpf) -> Spectrum:
    result: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(n)}
    result[(0,0,0)] = (mp.mpc(1),mp.mpc(-2),mp.mpc(3))
    for mode,component,amplitude in (((0,1,0),0,1),((0,0,1),1,2),((1,0,0),2,3)):
        values = [mp.mpc(0)]*3
        values[component] = amplitude*(1+mp.sin(7*time))/(2*mp.j)
        result[mode] = triple(values)
        result[triple(-v for v in mode)] = triple(mp.conj(v) for v in values)
    return result


def encode(state: Spectrum, precision: int) -> list[JsonObject]:
    return [{'mode':k,'value':[[mp.nstr(v.real,precision),mp.nstr(v.imag,precision)] for v in state[k]]}
            for k in sorted(state)]


def right_hand_side(n: int) -> RightHandSide:
    def rhs(current: Spectrum, time: mpf) -> Spectrum:
        product = nonlinear(current,n)
        prescribed = force(n,time)
        return {k:triple(a+b for a,b in zip(product[k],prescribed[k])) for k in current}
    return rhs


def study(n: int, precision: int) -> list[Study]:
    with mp.workdps(precision):
        reservation = preflight(n,precision,4*1024**3)
        state: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(n)}
        rhs = right_hand_side(n)
        dt = mp.mpf(1)/100
        results: list[Study] = []
        for name,method in (('CM',cm_step),('HO',ho_step)):
            full = method(state,mp.mpf(0),dt,rhs,mp.mpf(1))
            middle = method(state,mp.mpf(0),dt/2,rhs,mp.mpf(1))
            fine = method(middle,dt/2,dt/2,rhs,mp.mpf(1))
            discrepancy = max(abs(a-b) for k in state for a,b in zip(full[k],fine[k]))
            results.append(Study(precision,name,reservation,full,fine,discrepancy))
        return results


def compare(low: Study, high: Study) -> list[JsonObject]:
    results: list[JsonObject] = []
    with mp.workdps(120):
        paths = (('full_step',low.full_step,high.full_step),
                 ('two_half_steps',low.two_half_steps,high.two_half_steps))
        for path,left,right in paths:
            error = max(abs(a-b) for k in left for a,b in zip(left[k],right[k]))
            if error >= mp.mpf('1e-60'):
                raise ArithmeticError('DFT step arithmetic refinement failed')
            results.append({'method':low.method,'path':path,
                            'max_full_band_component_change':mp.nstr(error,12)})
    return results


def study_report(item: Study) -> JsonObject:
    return {'precision':item.precision,'method':item.method,'preflight':item.preflight,
            'full_step':encode(item.full_step,item.precision),
            'two_half_steps':encode(item.two_half_steps,item.precision),
            'raw_local_discrepancy':mp.nstr(item.raw_local_discrepancy,20)}


def run(n: int = 4) -> JsonObject:
    low = study(n,80)
    high = study(n,120)
    comparisons = [row for left,right in zip(low,high) for row in compare(left,right)]
    return {'status':'passed','scope':f'N={n} smooth forced semidiscrete arithmetic fixture',
            'command':f'python -m reference.verify_steps (grid {n})',
            'initial_state':'exact rest at t=0',
            'problem':'unit periodic cube; nu=1; mean force (1,-2,3); cyclic sines with amplitudes (1,2,3)*(1+sin(7t))',
            'dt':'1/100','absolute_comparison_tolerance':'1e-60',
            'studies':[study_report(s) for s in low+high],'comparisons':comparisons,
            'reference_assignments':0,'limitations':['Not similarity-mms-v2',
            'No binary64 production comparison','No accepted PDE convergence window']}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
