"""Bounded exact-word readers for complete derived fields and unprojected doubled-band forcing."""
from dataclasses import dataclass
from collections.abc import Mapping
from mpmath import mp, mpf
from reference.cyclic.bits import FixedForcing, binary64, integer, mode_at, sample
from reference.cyclic.results import require, rust_state
from reference.dft import Spectrum, retained
from reference.derived.spectral import inventory, modes
from reference.tuples import triple
from tools.json_types import Json, array_value, object_value, string_value


@dataclass(frozen=True)
class Derived:
    """Checked imported diagnostic values; metadata and checksums do not authenticate execution."""
    state: Spectrum
    force: Spectrum
    fields: tuple[tuple[mpf,...],...]


def raw_force(rows: Json, n: int) -> Spectrum:
    """Retain unprojected full force, including coefficients beyond the velocity band."""
    words = sample(rows,n)
    raw = {mode_at(i,n):triple(mp.mpc(binary64(a),binary64(b)) for a,b in vector)
           for i,vector in enumerate(words)}
    return {k:(triple(mp.conj(v) for v in raw[triple(-j for j in k)]) if k[2]<0 else raw[k])
            for k in modes(n)}


def derived(root: Mapping[str,Json], n: int, method: str) -> Derived:
    """Require exact schedule, geometry and all 46 ordered fields before numerical comparison."""
    retained(n)
    require(root,'schema','NSBU_DERIVED_1')
    require(root,'samples',2*n)
    require(root,'diagnostic_scalar_transforms',55)
    reserved,cap=integer(root.get('reservation_bytes')),integer(root.get('cap_bytes'))
    if reserved<=0 or reserved>cap:
        raise ValueError('Invalid declared Rust diagnostic reservation')
    state=rust_state(root,n,method)
    force=raw_force(root.get('pressure_force'),2*n)
    rows=array_value(root.get('fields'))
    if len(rows)!=46:
        raise ValueError('Complete derivative inventory requires 46 fields')
    fields: list[tuple[mpf,...]]=[]
    for expected,item in zip(inventory(),rows,strict=True):
        row=object_value(item)
        quantity=string_value(row.get('quantity'))
        component=integer(row.get('component'))
        orders=triple(integer(v) for v in array_value(row.get('orders')))
        if (quantity,component,orders)!=expected:
            raise ValueError('Derived fields are missing, duplicated or reordered')
        values=array_value(row.get('bits'))
        if len(values)!=(2*n)**3:
            raise ValueError('Physical sample lattice is incomplete')
        fields.append(tuple(binary64(integer(v)) for v in values))
    return Derived(state,force,tuple(fields))


def bind_endpoint_force(force: Spectrum, fixed: FixedForcing) -> None:
    """Require the independently exported diagnostic force to equal the fixed endpoint input.

    For this bounded smooth profile all prescribed nonzero modes fit the retained
    band. Extra doubled-band force cannot be silently discarded or projected away.
    """
    words=fixed.samples[128]
    raw={mode_at(i,fixed.n):triple(mp.mpc(binary64(a),binary64(b)) for a,b in vector)
         for i,vector in enumerate(words)}
    zero=(mp.mpc(0),mp.mpc(0),mp.mpc(0))
    for k,value in force.items():
        canonical=triple(-v for v in k) if k[2]<0 else k
        expected=raw.get(canonical,zero)
        if k[2]<0:
            expected=triple(mp.conj(v) for v in expected)
        if value!=expected:
            raise ValueError('Diagnostic pressure force differs from the fixed endpoint input')
