"""Reproducible 80/120-digit scalar-versus-jet evidence; no PDE integration."""
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
from mpmath import mp, mpf
from reference.evaluator import Evaluation, evaluate
from reference.tuples import triple, quadruple
from tools.json_types import JsonObject
from reference.scalar import fields, rational

SAMPLES = (
    ('origin', ('0','0','0','1/256')),
    ('near-axis', ('1/1000000','-1/1000000','1/10','1/1024')),
    ('interior', ('1/32','-1/64','1/10','1/1024')),
    ('inner-edge', ('3/10','0','0','1/256')),
    ('collar', ('7/20','0','1/20','1/256')),
    ('outer-edge', ('21/50','0','0','1/256')),
    ('rest', ('1/32','1/64','1/10','0')),
    ('ramp-end', ('1/32','1/64','1/10','1/512')),
    ('late', ('1/10000','0','1/10000','999999/128000000')),
)


def scalar_force(point: tuple[mpf, mpf, mpf, mpf]) -> tuple[mpf, ...]:
    """Differentiate explicit scalar velocities, without calling jet operations."""
    velocity, _ = fields(*point)
    result: list[mpf] = []
    for component in range(3):
        def u(*coordinates: mpf) -> mpf:
            return fields(*quadruple(coordinates))[0][component]
        derivatives = [mp.diff(u,point,tuple(int(j == axis) for j in range(4)),
                               direction=1 if point[3] == 0 and axis == 3 else 0)
                       for axis in range(4)]
        laplacian = sum(mp.diff(u,point,tuple(2*int(j == axis) for j in range(4)))
                        for axis in range(3))
        def p(*coordinates: mpf) -> mpf:
            return fields(*quadruple(coordinates))[1]
        pressure = mp.diff(p,point,
                           tuple(int(j == component) for j in range(4)))
        result.append(derivatives[3]+sum(velocity[j]*derivatives[j] for j in range(3))
                      -laplacian+pressure)
    return triple(result)


@dataclass(frozen=True)
class FieldSample:
    label: str
    coordinates: tuple[str, str, str, str]
    evaluated: Evaluation
    scaled_oracle_error: mpf


def sample(label: str, coordinates: tuple[str, str, str, str]) -> FieldSample:
    point = quadruple(rational(c) for c in coordinates)
    jet = evaluate(*point)
    oracle = scalar_force(point)
    scale = max(mp.mpf(1), *(abs(f) for f in oracle))
    error = max(abs(a-b) for a,b in zip(oracle,jet.force))/scale
    if error >= mp.mpf('1e-60'):
        raise ArithmeticError(f'{label}: force disagreement {error}')
    return FieldSample(label,coordinates,jet,error)


def sample_report(item: FieldSample, precision: int) -> JsonObject:
    jet = item.evaluated
    return {'sample':item.label,'coordinates':item.coordinates,
            'velocity':[mp.nstr(v,precision) for v in jet.velocity],
            'force':[mp.nstr(f,precision) for f in jet.force],
            'force_oracle_scaled_error':mp.nstr(item.scaled_oracle_error,12),
            'divergence':mp.nstr(jet.divergence,12),
            'root_iterations':jet.root_iterations,
            'force_gradient':[[mp.nstr(v,precision) for v in row] for row in jet.force_gradient],
            'max_root_jet_residual':mp.nstr(max(abs(c) for c in jet.root_residual_coefficients),12),
            'assembly_conditioning':[mp.nstr(sum(abs(v) for v in terms)/max(abs(f),mp.mpf('1e-60')),12)
                                     for terms,f in zip(jet.momentum_terms,jet.force)]}


def compare(low: FieldSample, high: FieldSample) -> JsonObject:
    with mp.workdps(120):
        scale = max(mp.mpf(1),*(abs(f) for f in high.evaluated.force))
        error = max(abs(a-b) for a,b in zip(low.evaluated.force,high.evaluated.force))/scale
        if error >= mp.mpf('1e-60'):
            raise ArithmeticError('Precision refinement failed')
        return {'sample':low.label,'force_scaled_change':mp.nstr(error,12)}


def run() -> JsonObject:
    results: dict[str,list[JsonObject]] = {}
    samples: list[list[FieldSample]] = []
    for precision in (80,120):
        with mp.workdps(precision):
            values = [sample(label,coordinates) for label,coordinates in SAMPLES]
            results[str(precision)] = [sample_report(s,precision) for s in values]
            samples.append(values)
    changes = [compare(low,high) for low,high in zip(samples[0],samples[1])]
    case = Path('benchmarks/similarity-mms-v2.json')
    return {'status':'passed','scope':'pointwise reference evaluation only',
            'command':'python -m reference.verify_fields','case_sha256':hashlib.sha256(case.read_bytes()).hexdigest(),
            'scaled_error_tolerance':'1e-60','precisions':results,'precision_comparison':changes,
            'limitations':['Empirical arithmetic comparisons, not enclosures','Pressure is raw; no periodic mean quadrature',
                           'No Fourier force-sampling qualification','No PDE trajectory']}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
