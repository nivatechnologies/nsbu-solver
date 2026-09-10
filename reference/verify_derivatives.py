"""Independent 80/120-digit v2 physical-derivative fixtures; no PDE integration."""
import hashlib
import json
from pathlib import Path
from mpmath import mp, mpf
from reference.evaluator import field_jets
from reference.jets import Jet
from reference.scalar import rational
from reference.tuples import quadruple
from reference.verify_fields import SAMPLES
from tools.json_types import JsonObject

FIELDS = (tuple(f'velocity_{i}' for i in range(3))
          + tuple(f'gradient_{i}_{j}' for i in range(3) for j in range(3))
          + tuple(f'hessian_{i}_{j}_{k}' for i in range(3) for j in range(3) for k in range(3))
          + tuple(f'vorticity_{i}' for i in range(3))
          + ('pressure_raw',)
          + tuple(f'pressure_gradient_{i}' for i in range(3)))


def evaluate(coordinates: tuple[str, str, str, str]) -> tuple[mpf, ...]:
    """Retain ordered tensor entries from the independently implemented Python jets."""
    point = quadruple(Jet.variable(rational(value), axis)
                      for axis, value in enumerate(coordinates))
    velocity, pressure, _ = field_jets(*point)
    gradient = [[value.derivative(axis) for axis in range(3)] for value in velocity]
    values = [value.value for value in velocity]
    values.extend(value.value for row in gradient for value in row)
    values.extend(value.derivative(axis).value
                  for row in gradient for value in row for axis in range(3))
    values.extend((gradient[2][1].value-gradient[1][2].value,
                   gradient[0][2].value-gradient[2][0].value,
                   gradient[1][0].value-gradient[0][1].value))
    values.append(pressure.value)
    values.extend(pressure.derivative(axis).value for axis in range(3))
    return tuple(values)


def precision_change(low: tuple[mpf, ...], high: tuple[mpf, ...]) -> str:
    """Require componentwise subordinate precision change, keeping exact zeros explicit."""
    with mp.workdps(120):
        if not all(mp.isfinite(value) for value in low + high):
            raise ArithmeticError('Nonfinite physical derivative evidence')
        error = max(abs(a-b)/max(mp.mpf(1), abs(b)) for a, b in zip(low, high, strict=True))
        if not mp.isfinite(error) or error >= mp.mpf('1e-60'):
            raise ArithmeticError('Physical derivative precision refinement failed')
        return mp.nstr(error, 30)


def run() -> JsonObject:
    """Fixed nine-point, two-precision execution with no user-controlled unbounded work."""
    results: list[JsonObject] = []
    for label, coordinates in SAMPLES:
        with mp.workdps(80):
            low = evaluate(coordinates)
            words80 = [mp.nstr(value, 80) for value in low]
        with mp.workdps(120):
            high = evaluate(coordinates)
            words120 = [mp.nstr(value, 120) for value in high]
        results.append({'sample': label, 'coordinates': coordinates,
                        'values_80': words80, 'values_120': words120,
                        'maximum_componentwise_scaled_precision_change': precision_change(low, high)})
    case = Path('benchmarks/similarity-mms-v2.json')
    return {'status': 'passed', 'scope': 'pointwise physical derivatives only',
            'command': 'python -m reference.verify_derivatives',
            'case_sha256': hashlib.sha256(case.read_bytes()).hexdigest(),
            'fields': FIELDS, 'samples': results, 'precision_change_tolerance': '1e-60',
            'limitations': ['Empirical arithmetic comparison, not an enclosure',
                           'Raw pressure has no periodic mean quadrature',
                           'No sampled-grid or PDE trajectory qualification']}


if __name__ == '__main__':
    print(json.dumps(run(), indent=2))
