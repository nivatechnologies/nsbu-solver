"""Separate axial/radial quadrature and 80/120-digit pressure-gauge refinements."""
from fractions import Fraction
import hashlib
from pathlib import Path
from mpmath import mp
from reference.pressure_gauge import CAP_BYTES, MeanPlan, PressureMean, evaluate
from tools.json_types import Json, JsonObject

CASE=Path(__file__).resolve().parents[1]/'benchmarks/similarity-mms-v2.json'
CASE_SHA256='e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e'


def plans(time: Fraction, panels: int, cap_bytes: int) -> tuple[MeanPlan,...]:
    """Admit every profile before evaluating any of them; only one runs at a time."""
    geometry=((panels,panels),(2*panels,2*panels),(4*panels,4*panels),
              (2*panels,4*panels),(4*panels,2*panels))
    return tuple(MeanPlan(time,nz,nr,dps,cap_bytes=cap_bytes) for dps in (80,120) for nz,nr in geometry)


def preflight(time: Fraction, panels: int, cap_bytes: int=CAP_BYTES) -> JsonObject:
    """Bind the frozen bytes and the complete finite sequence, without quadrature work."""
    if hashlib.sha256(CASE.read_bytes()).hexdigest()!=CASE_SHA256:
        raise ValueError('Frozen exact-v2 case hash mismatch')
    admitted=plans(time,panels,cap_bytes)
    return {'status':'pressure-mean-preflight','case_sha256':CASE_SHA256,
            'profiles':[p.preflight() for p in admitted],
            'sequential_profiles':len(admitted),'peak_reserved_bytes':max(p.reserved_bytes for p in admitted),
            'pressure_evaluations':sum(p.evaluations for p in admitted),
            'maximum_root_iterations':sum((p.axial_panels+1)*p.root_iterations for p in admitted),
            'classification':'EmpiricalQuadrature','accepted_pde_windows':0}


def changes(means: tuple[PressureMean,...]) -> JsonObject:
    """Keep arithmetic separation distinct from all joint and one-direction grid changes."""
    with mp.workdps(120):
        fine=means[7].value
        differences=[abs(means[a].value-means[b].value) for a,b in ((5,6),(6,7),(8,7),(9,7))]
        arithmetic=[abs(means[i].value-means[i+5].value) for i in range(5)]
        scale=max(differences[1:])
        return {'coarse_to_middle':mp.nstr(differences[0],100),'middle_to_fine':mp.nstr(differences[1],100),
                'axial_only_to_fine':mp.nstr(differences[2],100),'radial_only_to_fine':mp.nstr(differences[3],100),
                'precision_differences':[mp.nstr(v,100) for v in arithmetic],
                'maximum_precision_to_finest_quadrature_ratio':mp.nstr(max(arithmetic)/scale,100) if scale else None,
                'finest_mean':mp.nstr(fine,120),
                'relative_finest_quadrature_change':mp.nstr(scale/abs(fine),100) if fine else None}


def study(time: Fraction, panels: int, cap_bytes: int=CAP_BYTES) -> JsonObject:
    """Return all ten actual estimates; no configured tolerance silently promotes them to acceptance."""
    result=preflight(time,panels,cap_bytes)
    means=tuple(evaluate(plan) for plan in plans(time,panels,cap_bytes))
    rows: list[Json]=[{'axial_panels':m.plan.axial_panels,'radial_squared_panels':m.plan.radial_panels,
                      'precision':m.plan.precision,'mean':mp.nstr(m.value,120)} for m in means]
    result.update({'status':'pressure-mean-diagnostic-complete','estimates':rows,'changes':changes(means),
                   'limitations':['Empirical refinements, not rigorous quadrature enclosures',
                                  'One global unit-cube gauge; regional errors must not refit it',
                                  'Independent reference only; no state assignment or PDE qualification']})
    return result
