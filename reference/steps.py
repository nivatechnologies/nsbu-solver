"""Arbitrary-precision CM and independent HO tableau fixtures, design section 5.

This is a allocating reference implementation, not the bounded runtime API.
"""
from collections.abc import Callable
from mpmath import mp, mpf
from reference.dft import Spectrum

RightHandSide = Callable[[Spectrum, mpf], Spectrum]


def cm_weights(z: mpf) -> tuple[mpf, mpf, mpf, mpf]:
    """Independent integral-definition phi evaluations via hypergeometric series."""
    phi = [mp.hyp1f1(1,j+1,z)/mp.factorial(j) for j in (1,2,3)]
    p1,p2,p3 = phi
    half = mp.hyp1f1(1,2,z/2)/2
    return half,p1-3*p2+4*p3,2*p2-4*p3,-p2+4*p3


def combine(base: Spectrum, exponent: dict, terms: list[tuple[dict,Spectrum]]) -> Spectrum:
    return {k:tuple(exponent[k]*u[c]+sum(w[k]*v[k][c] for w,v in terms) for c in range(3))
            for k,u in base.items()}


def cm_step(state: Spectrum, time: mpf, dt: mpf, rhs: RightHandSide, nu: mpf) -> Spectrum:
    z = {k:-nu*4*mp.pi**2*sum(m*m for m in k)*dt for k in state}
    weights = {k:cm_weights(v) for k,v in z.items()}
    e = {k:mp.exp(v) for k,v in z.items()}
    half = {k:mp.exp(v/2) for k,v in z.items()}
    q = {k:dt*w[0] for k,w in weights.items()}
    n1 = rhs(state,time)
    a = combine(state,half,[(q,n1)])
    na = rhs(a,time+dt/2)
    b = combine(state,half,[(q,na)])
    nb = rhs(b,time+dt/2)
    c = combine(a,half,[(q,{k:tuple(2*b-a for a,b in zip(n1[k],nb[k])) for k in state})])
    nc = rhs(c,time+dt)
    source = [(n1,1),(na,2),(nb,2),(nc,3)]
    return combine(state,e,[({k:dt*w[index] for k,w in weights.items()},v) for v,index in source])


def ho_phi(z: mpf, order: int) -> mpf:
    """Separate coefficient construction by Taylor sum, with bounded work."""
    term = 1/mp.factorial(order)
    total = term
    for j in range(1,4096):
        term *= z/(j+order)
        total += term
        if abs(term) <= mp.eps*abs(total):
            return total
    raise ArithmeticError('HO reference coefficient sum exhausted')


def ho_tableau(z: mpf) -> tuple[list[list[mpf]], list[mpf]]:
    p1,p2,p3 = (ho_phi(z,j) for j in (1,2,3))
    h1,h2,h3 = (ho_phi(z/2,j) for j in (1,2,3))
    a52 = h2/2-p3+p2/4-h3/2
    a54 = h2/4-a52
    rows = [[],[h1/2],[h1/2-h2,h2],[p1-2*p2,p2,p2],
            [h1/2-2*a52-a54,a52,a52,a54]]
    return rows,[p1-3*p2+4*p3,mp.mpf(0),mp.mpf(0),-p2+4*p3,4*p2-8*p3]


def ho_step(state: Spectrum, time: mpf, dt: mpf, rhs: RightHandSide, nu: mpf) -> Spectrum:
    nodes = (mp.mpf(0),mp.mpf('0.5'),mp.mpf('0.5'),mp.mpf(1),mp.mpf('0.5'))
    z = {k:-nu*4*mp.pi**2*sum(m*m for m in k)*dt for k in state}
    tables = {k:ho_tableau(v) for k,v in z.items()}
    sources = []
    for i,node in enumerate(nodes):
        stage = combine(state,{k:mp.exp(node*v) for k,v in z.items()},
                        [({k:dt*tables[k][0][i][j] for k in state},v) for j,v in enumerate(sources)])
        sources.append(rhs(stage,time+node*dt))
    return combine(state,{k:mp.exp(v) for k,v in z.items()},
                   [({k:dt*tables[k][1][j] for k in state},v) for j,v in enumerate(sources)])
