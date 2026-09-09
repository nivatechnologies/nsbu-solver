"""Full-step/two-half-step DFT arithmetic fixture from exact rest at 80/120 digits."""
import json
from mpmath import mp
from reference.dft import Spectrum, nonlinear, preflight, retained
from reference.steps import cm_step, ho_step


def force(n: int, time: mp.mpf) -> Spectrum:
    result = {k:(mp.mpc(0),)*3 for k in retained(n)}
    result[(0,0,0)] = (mp.mpc(1),mp.mpc(-2),mp.mpc(3))
    for mode,component,amplitude in (((0,1,0),0,1),((0,0,1),1,2),((1,0,0),2,3)):
        values = [mp.mpc(0)]*3
        values[component] = amplitude*(1+mp.sin(7*time))/(2*mp.j)
        result[mode] = tuple(values)
        result[tuple(-v for v in mode)] = tuple(mp.conj(v) for v in values)
    return result


def encode(state: Spectrum, precision: int) -> list[dict]:
    return [{'mode':k,'value':[[mp.nstr(v.real,precision),mp.nstr(v.imag,precision)] for v in state[k]]}
            for k in sorted(state)]


def run() -> dict:
    n = 4
    studies = []
    for precision in (80,120):
        with mp.workdps(precision):
            reservation = preflight(n,precision,1024**3)
            state = {k:(mp.mpc(0),)*3 for k in retained(n)}
            def rhs(current: Spectrum, time: mp.mpf) -> Spectrum:
                product = nonlinear(current,n)
                prescribed = force(n,time)
                return {k:tuple(a+b for a,b in zip(product[k],prescribed[k])) for k in current}
            dt = mp.mpf(1)/100
            for name,method in (('CM',cm_step),('HO',ho_step)):
                full = method(state,mp.mpf(0),dt,rhs,mp.mpf(1))
                middle = method(state,mp.mpf(0),dt/2,rhs,mp.mpf(1))
                fine = method(middle,dt/2,dt/2,rhs,mp.mpf(1))
                discrepancy = max(abs(a-b) for k in state for a,b in zip(full[k],fine[k]))
                studies.append({'precision':precision,'method':name,'preflight':reservation,
                                'full_step':encode(full,precision),'two_half_steps':encode(fine,precision),
                                'raw_local_discrepancy':mp.nstr(discrepancy,20)})
    comparisons = []
    with mp.workdps(120):
        for low,high in zip(studies[:2],studies[2:]):
            for path in ('full_step','two_half_steps'):
                errors = [abs(mp.mpf(a)-mp.mpf(b)) for lo,hi in zip(low[path],high[path])
                          for lv,hv in zip(lo['value'],hi['value']) for a,b in zip(lv,hv)]
                error = max(errors)
                if error >= mp.mpf('1e-60'):
                    raise ArithmeticError('DFT step arithmetic refinement failed')
                comparisons.append({'method':low['method'],'path':path,'max_full_band_component_change':mp.nstr(error,12)})
    return {'status':'passed','scope':'N=4 smooth forced semidiscrete arithmetic fixture',
            'command':'python -m reference.verify_steps','initial_state':'exact rest at t=0',
            'problem':'unit periodic cube; nu=1; mean force (1,-2,3); cyclic sines with amplitudes (1,2,3)*(1+sin(7t))',
            'dt':'1/100','scaled_comparison_tolerance':'1e-60','studies':studies,'comparisons':comparisons,
            'reference_assignments':0,'limitations':['Not similarity-mms-v2','N=8,12 studies not yet executed',
            'No binary64 production comparison','No accepted PDE convergence window','Quality gates not yet measured']}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
