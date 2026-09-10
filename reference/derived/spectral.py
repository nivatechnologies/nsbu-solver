"""Full complex direct sums for physical derivatives and conservative mean-zero pressure."""
from collections.abc import Iterator
from itertools import combinations_with_replacement, product
from mpmath import mp, mpc, mpf
from reference.dft import Mode, Spectrum, curl, inverse_vector, retained, transform
from reference.tuples import triple

Scalar = dict[Mode, mpc]
Orders = tuple[int, int, int]


def modes(n: int) -> tuple[Mode, ...]:
    """Complete strict diagnostic band on twice an admitted retained grid."""
    if n not in (8, 16, 24):
        raise ValueError('Derived diagnostic grid must be 8,16,24')
    return tuple(triple(k) for k in product(range(1-n//2, n//2), repeat=3))


def index(mode: Mode, n: int) -> int:
    """Lexicographic unshifted full-complex storage coordinate."""
    x, y, z = (k % n for k in mode)
    return (x*n+y)*n+z


def inverse(values: Scalar, n: int, orders: Orders = (0, 0, 0)) -> list[mpc]:
    """Full complex derivative samples; imaginary defects remain visible, not repaired."""
    if any(k < 0 for k in orders) or sum(orders) > 2:
        raise ValueError('Derivative total order must be zero, one or two')
    if n not in (8,16,24):
        raise ValueError('Unsupported derived sample grid')
    dense = [mp.mpc(0)]*n**3
    for mode, value in values.items():
        if any(abs(k) >= n//2 for k in mode) or not mp.isfinite(value):
            raise ValueError('Nonfinite or out-of-band derivative coefficient')
        multiplier = mp.mpc(1)
        for frequency, order in zip(mode, orders):
            multiplier *= (2*mp.pi*mp.j*frequency)**order
        dense[index(mode,n)] = value*multiplier
    return transform(dense,n,True)


def pressure(state: Spectrum, force: Spectrum, n: int) -> tuple[Scalar, mpf]:
    """Independently form p_hat=-sum(k_i k_j (u_i u_j)_hat)/|k|²-i k.f_hat/|k|².

    Nine scalar direct DFTs form the complete quadratic band. The excluded
    Nyquist product magnitude is reported separately. No projected force,
    integrator-stage RHS or analytical pressure is substituted.
    """
    if tuple(state) != retained(n) or tuple(force) != modes(2*n):
        raise ValueError('Pressure requires complete ordered state and doubled force bands')
    if any(not mp.isfinite(v) for field in (state,force) for vector in field.values() for v in vector):
        raise ValueError('Pressure inputs must be finite')
    m = 2*n
    velocity = inverse_vector(state,m)
    result = {k:mp.mpc(0) for k in force}
    excluded = mp.mpf(0)
    for row,column in combinations_with_replacement(range(3),2):
        values = transform([u[row]*u[column] for u in velocity],m)
        add_product(result,values,row,column,m)
        excluded = max(excluded, nyquist_maximum(values,m))
    for k, f in force.items():
        squared = sum(v*v for v in k)
        if squared:
            result[k] -= mp.j*sum((v*a for v,a in zip(k,f)),mp.mpc(0))/(2*mp.pi*squared)
    return result, excluded


def add_product(result: Scalar, values: list[mpc], row: int, column: int, n: int) -> None:
    """Accumulate one complete Fourier tensor product with its ordered multiplicity."""
    multiplicity = 1 if row==column else 2
    for k in result:
        squared = sum(v*v for v in k)
        if squared:
            result[k] -= multiplicity*k[row]*k[column]*values[index(k,n)]/squared


def nyquist_maximum(values: list[mpc], n: int) -> mpf:
    """Preserve product leakage on the explicitly excluded Nyquist planes."""
    return max(abs(values[(x*n+y)*n+z]) for x,y,z in product(range(n),repeat=3)
               if n//2 in (x,y,z))


def velocity_orders() -> tuple[Orders, ...]:
    """Scalar, gradient and full ordered Hessian multi-indices for one velocity component."""
    return ((0,0,0), *(triple(int(j==a) for j in range(3)) for a in range(3)),
            *(triple(int(j==a)+int(j==b) for j in range(3)) for a,b in product(range(3),repeat=2)))


def inventory() -> tuple[tuple[str,int,Orders], ...]:
    """Exact 46-row order, including both occurrences of every mixed Hessian entry."""
    rows = [('velocity',c,o) for c in range(3) for o in velocity_orders()]
    rows.extend(('vorticity',c,(0,0,0)) for c in range(3))
    rows.extend(('pressure',0,o) for o in velocity_orders()[:4])
    return tuple(rows)


def velocity_entries(values: Scalar, n: int) -> Iterator[list[mpc]]:
    """Stream one complete component; keep only three mixed entries that occur twice."""
    cache: dict[Orders,list[mpc]] = {}
    for orders in velocity_orders():
        if orders in cache:
            yield cache[orders]
        else:
            sampled = inverse(values,n,orders)
            if sum(order==1 for order in orders)==2:
                cache[orders] = sampled
            yield sampled


def fields(state: Spectrum, p: Scalar, n: int) -> Iterator[list[mpc]]:
    """Yield complete physical entries sequentially, retaining modes and imaginary parts."""
    for component in range(3):
        yield from velocity_entries({k:u[component] for k,u in state.items()},2*n)
    omega = curl(state)
    for component in range(3):
        yield inverse({k:u[component] for k,u in omega.items()},2*n)
    for orders in velocity_orders()[:4]:
        yield inverse(p,2*n,orders)
