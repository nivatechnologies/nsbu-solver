"""Independent force sampling refinements; differences are empirical, never tail proofs."""
import json
from mpmath import mp, mpf
from reference.dft import Spectrum, Vector, forward_vector, grid, preflight
from reference.evaluator import evaluate
from reference.scalar import rational
from reference.tuples import triple
from tools.json_types import JsonObject


def sample_force(n: int, time: mpf, precision: int, cap_bytes: int) -> tuple[Spectrum,JsonObject]:
    reservation = preflight(n,precision,cap_bytes)
    if not mp.isfinite(time) or not 0 <= time < rational('1/128'):
        raise ValueError('Force sampling requires 0 <= time < T_star')
    with mp.workdps(precision):
        samples: list[Vector] = []
        max_conditioning = mp.mpf(0)
        root_work = 0
        for point in grid(n):
            coordinates = triple(mp.mpf(j if j < n//2 else j-n)/n for j in point)
            value = evaluate(*coordinates,time)
            samples.append(triple(mp.mpc(v) for v in value.force))
            root_work += value.root_iterations
            for terms,force in zip(value.momentum_terms,value.force):
                condition = sum(abs(v) for v in terms)/max(abs(force),mp.mpf('1e-60'))
                max_conditioning = max(max_conditioning,condition)
        return forward_vector(samples,n,n),{'grid':n,'precision':precision,'time':mp.nstr(time,precision),
                  'preflight':reservation,'field_evaluations':n**3,'maximum_field_evaluations':n**3,
                  'scalar_root_iterations':root_work,'max_assembly_conditioning':mp.nstr(max_conditioning,20)}


def difference(coarse: Spectrum, fine: Spectrum) -> JsonObject:
    zero = (mp.mpc(0),mp.mpc(0),mp.mpc(0))
    squared = {k:sum((abs(a-b)**2 for a,b in zip(vector,coarse.get(k,zero))),mp.mpf(0))
               for k,vector in fine.items()}
    l2 = mp.sqrt(sum(squared.values(),mp.mpf(0)))
    h1 = mp.sqrt(sum(((1+4*mp.pi**2*sum(m*m for m in k))*value for k,value in squared.items()),mp.mpf(0)))
    maximum = max(abs(a-b) for k,vector in fine.items() for a,b in zip(vector,coarse.get(k,zero)))
    return {'full_fine_band_l2':mp.nstr(l2,20),'full_fine_band_h1':mp.nstr(h1,20),
            'max_component_change':mp.nstr(maximum,20)}


def run() -> JsonObject:
    reports: list[JsonObject] = []
    for time_text in ('1/1024','1/256'):
        low: list[Spectrum] = []
        sampling: list[JsonObject] = []
        with mp.workdps(80):
            time = rational(time_text)
            for n in (4,8,12):
                field,report = sample_force(n,time,80,4*1024**3)
                low.append(field)
                sampling.append(report)
            spatial = [difference(a,b) for a,b in zip(low,low[1:])]
        with mp.workdps(120):
            high,high_report = sample_force(12,rational(time_text),120,4*1024**3)
            arithmetic = difference(low[-1],high)
            if mp.mpf(str(arithmetic['max_component_change'])) >= mp.mpf('1e-60'):
                raise ArithmeticError('Force sampling arithmetic refinement failed')
        reports.append({'time':time_text,'sampling':sampling,'high_precision':high_report,
                        'spatial_differences':spatial,'arithmetic_difference':arithmetic})
    return {'status':'diagnostic-completed','case':'similarity-mms-v2','reports':reports,
            'spatial_qualification':'unresolved; no tolerance protocol passed',
            'limitations':['Small sampling grids are not a force-resolution certificate',
                           'No integrated trajectory','Empirical sampled comparisons, not enclosures']}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
