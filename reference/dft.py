"""Separable full-complex direct DFT sums; no FFT library or production kernels."""
from itertools import product
from mpmath import mp, mpc
from reference.tuples import triple
from tools.json_types import JsonObject

Mode = tuple[int, int, int]
Vector = tuple[mpc, mpc, mpc]
Spectrum = dict[Mode, Vector]


def grid(n: int) -> tuple[Mode, ...]:
    return tuple(triple(p) for p in product(range(n), repeat=3))


def retained(n: int) -> tuple[Mode, ...]:
    if n not in (4,8,12):
        raise ValueError('Reference fixtures support N=4,8,12 only')
    return tuple(triple(p) for p in product(range(1-n//2,n//2),repeat=3))


def transform(values: list[mpc], n: int, inverse: bool = False) -> list[mpc]:
    """Forward normalization 1/N³; inverse is the unnormalized Fourier sum."""
    if n not in (4,6,8,12,18) or len(values) != n**3:
        raise ValueError('Invalid direct-DFT grid or payload')
    sign = 1 if inverse else -1
    roots = [mp.exp(sign*2*mp.pi*mp.j*j/n) for j in range(n)]
    source = values[:]
    for axis in range(3):
        stride = (n*n,n,1)[axis]
        output = [mp.mpc(0)]*len(source)
        for point in grid(n):
            index = (point[0]*n+point[1])*n+point[2]
            base = index-point[axis]*stride
            output[index] = sum((source[base+j*stride]*roots[(j*point[axis])%n] for j in range(n)), mp.mpc(0))
        source = output
    return source if inverse else [v/n**3 for v in source]


def inverse_vector(state: Spectrum, n: int) -> list[Vector]:
    arrays: list[list[mpc]] = []
    for component in range(3):
        values = [mp.mpc(0)]*(n**3)
        for mode,vector in state.items():
            i,j,k = (m%n for m in mode)
            values[(i*n+j)*n+k] = vector[component]
        arrays.append(transform(values,n,True))
    return [triple(v) for v in zip(*arrays)]


def forward_vector(values: list[Vector], n: int, base: int) -> Spectrum:
    arrays = [transform([v[c] for v in values],n) for c in range(3)]
    result: Spectrum = {}
    for mode in retained(base):
        i,j,k = (m%n for m in mode)
        index = (i*n+j)*n+k
        result[mode] = triple(a[index] for a in arrays)
    return result


def cross(a: Vector, b: Vector) -> Vector:
    return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])


def project(mode: Mode, vector: Vector) -> Vector:
    squared = sum(k*k for k in mode)
    if squared == 0:
        return vector
    longitudinal = sum((k*v for k,v in zip(mode,vector)), mp.mpc(0))/squared
    return triple(v-k*longitudinal for k,v in zip(mode,vector))


def curl(state: Spectrum) -> Spectrum:
    return {k: triple(2*mp.pi*mp.j*v for v in cross(triple(mp.mpc(m) for m in k),u)) for k,u in state.items()}


def nonlinear(state: Spectrum, n: int) -> Spectrum:
    """3/2 padded rotational product, cropped and projected onto full retained band."""
    padded = 3*n//2
    velocity = inverse_vector(state,padded)
    vorticity = inverse_vector(curl(state),padded)
    result = forward_vector([cross(u,w) for u,w in zip(velocity,vorticity)],padded,n)
    return {k:project(k,v) for k,v in result.items()}


def convolution(state: Spectrum) -> Spectrum:
    """Independent coefficient-pair convolution, with no DFT or grid product."""
    result = {k:[mp.mpc(0)]*3 for k in state}
    for p,u in state.items():
        for q,v in state.items():
            target = triple(a+b for a,b in zip(p,q))
            if target not in result:
                continue
            # Conservative form: -i (u_p dot k_q) u_q, projected afterwards.
            factor = -2*mp.pi*mp.j*sum(a*b for a,b in zip(u,q))
            for component in range(3):
                result[target][component] += factor*v[component]
    return {k:project(k,triple(v)) for k,v in result.items()}


def preflight(n: int, precision: int, cap_bytes: int) -> JsonObject:
    """Conservative fixture reservation, not a measured Python allocator bound."""
    retained(n)
    if precision not in (80,120):
        raise ValueError('Fixture precision must be 80 or 120 digits')
    allocations = {'retained_vectors':64*n**3*3*4096,
                   'padded_vectors_and_dft_temporaries':24*(3*n//2)**3*3*4096,
                   'metadata_and_interpreter_reservation':64*1024**2}
    total = sum(allocations.values())
    if total > cap_bytes:
        raise ValueError('Reference fixture exceeds memory reservation cap')
    return {'n':n,'precision':precision,'allocations':allocations,'reserved_bytes':total,
            'cap_bytes':cap_bytes,'classification':'conservative diagnostic reservation; no hard allocator guarantee'}
