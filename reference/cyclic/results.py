"""Strict readers for current-grid arithmetic reports; imported numbers remain diagnostic data."""
from collections.abc import Mapping
from mpmath import mp, mpf
from reference.cyclic.bits import binary64, integer, mode_at, vector
from reference.dft import Spectrum, retained
from reference.tuples import triple
from tools.json_types import Json, array_value, object_value, string_value


def require(root: Mapping[str, Json], key: str, expected: Json) -> None:
    """Require a present property of the exact scalar type, including explicit nulls."""
    if key not in root or type(root[key]) is not type(expected) or root[key] != expected:
        raise ValueError(f'Arithmetic report identity mismatch: {key}')


def decimal(value: Json) -> mpf:
    """Bound literal conversion and refuse nonfinite high-precision coefficients."""
    text = string_value(value)
    if len(text) > 256:
        raise ValueError('Coefficient decimal exceeds 256 characters')
    number = mp.mpf(text)
    if not mp.isfinite(number):
        raise ValueError('Nonfinite reference coefficient')
    return number


def reference_state(root: Mapping[str, Json], n: int, method: str,
                    precision: int, fixed_sha256: str | None) -> Spectrum:
    """Require the complete declared fine-update schedule and every full-band signed mode."""
    expected: dict[str, Json] = {
        'case': 'CyclicSine' if fixed_sha256 is None else 'fixed-stage-arithmetic-fixture',
        'nominal_comparison_case': 'CyclicSine', 'grid': n, 'method': method,
        'precision': precision, 'endpoint': '1/512', 'macro_step': '1/4096',
        'macro_steps': 8, 'fine_steps_committed': 16,
        'rhs_calls': 96 if method == 'CM' else 120, 'reference_assignments': 0,
        'fixed_force_sha256': fixed_sha256, 'status': 'diagnostic-completed',
        'accepted_pde_windows': 0,
    }
    for key, value in expected.items():
        require(root, key, value)
    rows = array_value(root.get('state'))
    modes = retained(n)
    if len(rows) != len(modes):
        raise ValueError('Reference state is missing full-band coefficients')
    output: Spectrum = {}
    for mode, item in zip(modes, rows):
        row = object_value(item)
        if triple(integer(v) for v in array_value(row.get('mode'))) != mode:
            raise ValueError('Reference modes are missing, duplicated or reordered')
        pairs = [array_value(v) for v in array_value(row.get('value'))]
        if any(len(pair) != 2 for pair in pairs):
            raise ValueError('Reference complex value requires two decimals')
        output[mode] = triple(mp.mpc(decimal(pair[0]), decimal(pair[1])) for pair in pairs)
    return output


def rust_state(root: Mapping[str, Json], n: int, method: str) -> Spectrum:
    """Expand raw binary64 storage without projecting, symmetrizing or discarding high modes."""
    require(root, 'grid', n)
    require(root, 'method', 'CoxMatthews' if method == 'CM' else 'HochbruckOstermann')
    require(root, 'tick_exponent', -16)
    require(root, 'elapsed_ticks', 128)
    require(root, 'macro_step_ticks', 16)
    require(root, 'macro_steps', 8)
    require(root, 'fine_steps_committed', 16)
    require(root, 'rhs_calls', 96 if method == 'CM' else 120)
    require(root, 'reference_assignments', 0)
    require(root, 'accepted_pde_windows', 0)
    rows = array_value(root.get('state'))
    if len(rows) != n*n*(n//2+1):
        raise ValueError('Rust state shape mismatch')
    raw: Spectrum = {}
    for index, item in enumerate(rows):
        row = object_value(item)
        mode = triple(integer(v) for v in array_value(row.get('mode')))
        if mode != mode_at(index, n):
            raise ValueError('Rust modes are missing, duplicated or reordered')
        values = triple(mp.mpc(binary64(a), binary64(b)) for a, b in vector(row.get('bits')))
        if n//2 in tuple(abs(k) for k in mode) and any(v != 0 for v in values):
            raise ValueError('Rust excluded Nyquist coefficient is nonzero')
        raw[mode] = values
    return {k: (triple(mp.conj(v) for v in raw[triple(-m for m in k)]) if k[2] < 0 else raw[k])
            for k in retained(n)}


def reality_defect(state: Spectrum) -> mpf:
    """Report stored-plane asymmetry without resetting either member of a conjugate pair."""
    return max(abs(a-mp.conj(b)) for k, values in state.items()
               for a, b in zip(values, state[triple(-m for m in k)]))
