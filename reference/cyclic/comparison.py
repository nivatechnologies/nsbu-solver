"""Separate prescribed-force effects, direct-DFT precision and actual Rust arithmetic."""
from dataclasses import dataclass
import hashlib
from pathlib import Path
from mpmath import mp
from reference.cyclic.bits import FixedForcing
from reference.cyclic.profile import projected_force
from reference.cyclic.results import reality_defect, reference_state, rust_state
from reference.cyclic.schema import decode_fixture
from reference.cyclic.study import maximum_difference
from reference.dft import Spectrum, retained
from tools.json_types import Json, JsonObject, object_value

MAXIMUM_INPUT_BYTES = 16*1024**2
RESERVATION_BYTES = 2*1024**3


@dataclass(frozen=True)
class Inputs:
    """All six independent artifacts are explicit; directory names do not establish identity."""
    rust: Path
    force: Path
    independent80: Path
    independent120: Path
    fixed80: Path
    fixed120: Path


def bounded_bytes(path: Path) -> bytes:
    """Bound each read before JSON parsing, including an extra byte to detect truncation."""
    with path.open('rb') as source:
        raw = source.read(MAXIMUM_INPUT_BYTES+1)
    if len(raw) > MAXIMUM_INPUT_BYTES:
        raise ValueError('Arithmetic input exceeds 16 MiB')
    return raw


def difference(left: Spectrum, right: Spectrum) -> JsonObject:
    """Complete unit-cube Fourier L2/H1 differences; no common-band crop or alignment."""
    maximum = maximum_difference(left, right)
    l2 = mp.mpf(0)
    h1 = mp.mpf(0)
    for mode in left:
        squared = sum((abs(a-b)**2 for a, b in zip(left[mode], right[mode])), mp.mpf(0))
        l2 += squared
        h1 += (1+4*mp.pi**2*sum(k*k for k in mode))*squared
    return {'maximum_coefficient': mp.nstr(maximum, 40),
            'l2': mp.nstr(mp.sqrt(l2), 40), 'h1': mp.nstr(mp.sqrt(h1), 40),
            'coefficient_differences': coefficient_differences(left,right)}


def coefficient_differences(left: Spectrum, right: Spectrum) -> list[Json]:
    """Persist every signed complex discrepancy after complete-band finite admission."""
    rows: list[Json] = []
    for mode in left:
        delta = [a-b for a, b in zip(left[mode],right[mode])]
        rows.append({'mode':list(mode),'value':[[mp.nstr(v.real,40),mp.nstr(v.imag,40)] for v in delta]})
    return rows


def compare(inputs: Inputs, n: int, method: str, cap_bytes: int) -> JsonObject:
    """Admit all input/parser/state storage before I/O; compare under a 120-digit context."""
    retained(n)
    if method not in ('CM','HO') or cap_bytes < RESERVATION_BYTES:
        raise ValueError('Arithmetic comparison profile or reservation refused')
    paths = {'rust': inputs.rust, 'force': inputs.force,
             'independent80': inputs.independent80, 'independent120': inputs.independent120,
             'fixed80': inputs.fixed80, 'fixed120': inputs.fixed120}
    raw = {name: bounded_bytes(path) for name, path in paths.items()}
    hashes = {name: hashlib.sha256(value).hexdigest() for name, value in raw.items()}
    with mp.workdps(120):
        force = FixedForcing(raw['force'], n, MAXIMUM_INPUT_BYTES)
        rust = rust_state(object_value(decode_fixture(raw['rust'])), n, method)
        states = {name: reference_state(object_value(decode_fixture(raw[name])), n, method,
                  precision, force.sha256 if fixed else None)
                  for name, precision, fixed in (('independent80',80,False),('independent120',120,False),
                                                ('fixed80',80,True),('fixed120',120,True))}
        comparisons = {
            'independent_precision_80_120': difference(states['independent80'],states['independent120']),
            'fixed_precision_80_120': difference(states['fixed80'],states['fixed120']),
            'rust_vs_fixed_120': difference(rust,states['fixed120']),
            'fixed_vs_independent_120': difference(states['fixed120'],states['independent120']),
            'rust_vs_independent_120': difference(rust,states['independent120']),
        }
        force_difference = max(maximum_difference(force.evaluate(mp.mpf(tick)/65536),
                               projected_force(n,mp.mpf(tick)/65536)) for tick in range(0,129,4))
        return {'status':'diagnostic-completed','case':'CyclicSine','grid':n,'method':method,
                'endpoint':'1/512','comparisons':comparisons,'input_sha256':hashes,
                'stage_projected_force_maximum':mp.nstr(force_difference,40),
                'rust_reality_defect':mp.nstr(reality_defect(rust),40),
                'reservation_bytes':RESERVATION_BYTES,'cap_bytes':cap_bytes,
                'reservation_scope':'conservative Python planning; no hard allocator guarantee',
                'accepted_pde_windows':0,'artifact_origin':'externally supplied diagnostic numbers',
                'scope':'same-grid velocity arithmetic and force effects; no trajectory qualification'}
