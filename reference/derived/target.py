"""Independent unprojected smooth target force, assembled on the complete diagnostic band."""
from mpmath import mp, mpf
from reference.cyclic.profile import profile
from reference.derived.spectral import modes
from reference.dft import Spectrum
from reference.tuples import triple


def force(n: int, time: mpf) -> Spectrum:
    """Unit-cube viscosity-one u_t+(u.grad)u-Delta u; target pressure is exactly zero."""
    state,derivative=profile(n,time)
    result={k:[mp.mpc(0)]*3 for k in modes(2*n)}
    for k,u in state.items():
        result[k]=[derivative[k][c]+4*mp.pi**2*sum(v*v for v in k)*u[c] for c in range(3)]
    sparse=[(k,u) for k,u in state.items() if any(u)]
    for p,u in sparse:
        for q,v in sparse:
            k=triple(a+b for a,b in zip(p,q))
            factor=2*mp.pi*mp.j*sum((a*b for a,b in zip(u,q)),mp.mpc(0))
            for c in range(3):
                result[k][c] += factor*v[c]
    return {k:triple(v) for k,v in result.items()}
