"""Reproducible 80/120-digit scalar-versus-jet evidence; no PDE integration."""
import hashlib
import json
from pathlib import Path
from mpmath import mp, mpf
from reference.evaluator import evaluate
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


def scalar_force(point: tuple[mpf, ...]) -> tuple[mpf, ...]:
    """Differentiate explicit scalar velocities, without calling jet operations."""
    velocity, _ = fields(*point)
    result = []
    for component in range(3):
        def u(*coordinates: mpf) -> mpf:
            return fields(*coordinates)[0][component]
        derivatives = [mp.diff(u,point,tuple(int(j == axis) for j in range(4)),
                               direction=1 if point[3] == 0 and axis == 3 else 0)
                       for axis in range(4)]
        laplacian = sum(mp.diff(u,point,tuple(2*int(j == axis) for j in range(4)))
                        for axis in range(3))
        pressure = mp.diff(lambda *p: fields(*p)[1],point,
                           tuple(int(j == component) for j in range(4)))
        result.append(derivatives[3]+sum(velocity[j]*derivatives[j] for j in range(3))
                      -laplacian+pressure)
    return tuple(result)


def run() -> dict:
    results = {}
    for precision in (80,120):
        rows = []
        with mp.workdps(precision):
            for label, coordinates in SAMPLES:
                point = tuple(rational(c) for c in coordinates)
                jet = evaluate(*point)
                oracle = scalar_force(point)
                scale = max(mp.mpf(1), *(abs(f) for f in oracle))
                error = max(abs(a-b) for a,b in zip(oracle,jet.force))/scale
                if error >= mp.mpf('1e-60'):
                    raise ArithmeticError(f'{label}: force disagreement {error}')
                rows.append({'sample':label,'coordinates':coordinates,
                             'velocity':[mp.nstr(v,precision) for v in jet.velocity],
                             'force':[mp.nstr(f,precision) for f in jet.force],
                             'force_oracle_scaled_error':mp.nstr(error,12),
                             'divergence':mp.nstr(jet.divergence,12),
                             'max_root_jet_residual':mp.nstr(max(abs(c) for c in jet.root_residual_coefficients),12),
                             'assembly_conditioning':[mp.nstr(sum(abs(v) for v in terms)/max(abs(f),mp.mpf('1e-60')),12)
                                                      for terms,f in zip(jet.momentum_terms,jet.force)]})
        results[str(precision)] = rows
    changes = []
    with mp.workdps(120):
        for low,high in zip(results['80'],results['120']):
            scale = max(mp.mpf(1),*(abs(mp.mpf(f)) for f in high['force']))
            error = max(abs(mp.mpf(a)-mp.mpf(b)) for a,b in zip(low['force'],high['force']))/scale
            if error >= mp.mpf('1e-60'):
                raise ArithmeticError('Precision refinement failed')
            changes.append({'sample':low['sample'],'force_scaled_change':mp.nstr(error,12)})
    case = Path('benchmarks/similarity-mms-v2.json')
    return {'status':'passed','scope':'pointwise reference evaluation only',
            'command':'python -m reference.verify_fields','case_sha256':hashlib.sha256(case.read_bytes()).hexdigest(),
            'scaled_error_tolerance':'1e-60','precisions':results,'precision_comparison':changes,
            'limitations':['Empirical arithmetic comparisons, not enclosures','Pressure is raw; no periodic mean quadrature',
                           'No Fourier force-sampling qualification','No PDE trajectory','Quality gates not yet measured']}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
