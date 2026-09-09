"""Arbitrary-precision CM and independent HO tableau fixtures, design section 5.

This is a allocating reference implementation, not the bounded runtime API.
"""
from collections.abc import Callable
from mpmath import mp, mpf
from reference.dft import Spectrum, Mode
from reference.tuples import triple, quadruple

Coefficients = dict[Mode, mpf]

RightHandSide = Callable[[Spectrum, mpf], Spectrum]


def coefficient_guard_digits(z: mpf) -> int:
    if not mp.isfinite(z) or z > 0 or z < -mp.mpf('1e309'):
        raise ValueError('Reference coefficients require finite -1e309 <= z <= 0')
    return 20 + int(mp.log10(max(abs(z),mp.mpf(1))))


def cm_weights(z: mpf) -> tuple[mpf, mpf, mpf, mpf]:
    """Hypergeometric oracle with guard digits for cancellation in combined weights."""
    with mp.workdps(mp.dps+coefficient_guard_digits(z)):
        p1,p2,p3 = (mp.hyp1f1(1,j+1,z)/mp.factorial(j) for j in (1,2,3))
        half = mp.hyp1f1(1,2,z/2)/2
        result = half,p1-3*p2+4*p3,2*p2-4*p3,-p2+4*p3
    return quadruple(+v for v in result)


def combine(base: Spectrum, exponent: Coefficients, terms: list[tuple[Coefficients,Spectrum]]) -> Spectrum:
    return {k:triple(exponent[k]*u[c]+sum((w[k]*v[k][c] for w,v in terms), mp.mpc(0)) for c in range(3))
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
    c = combine(a,half,[(q,{k:triple(2*b-a for a,b in zip(n1[k],nb[k])) for k in state})])
    nc = rhs(c,time+dt)
    source = [(n1,1),(na,2),(nb,2),(nc,3)]
    return combine(state,e,[({k:dt*w[index] for k,w in weights.items()},v) for v,index in source])


def ho_phi(z: mpf, order: int, max_terms: int = 512) -> mpf:
    """Independent negative-argument recurrence, with a bounded Taylor junction."""
    coefficient_guard_digits(z)
    if order not in (1,2,3) or max_terms < 1:
        raise ValueError('Require phi order 1..3 and a positive series cap')
    if z < -1:
        value = (mp.exp(z)-1)/z
        for j in range(2,order+1):
            value = (value-1/mp.factorial(j-1))/z
        return value
    term = 1/mp.factorial(order)
    total = term
    for j in range(1,max_terms):
        term *= z/(j+order)
        total += term
        if abs(term) <= mp.eps*abs(total):
            return total
    raise ArithmeticError('HO reference coefficient sum exhausted')


def ho_tableau_values(z: mpf) -> tuple[list[list[mpf]], list[mpf]]:
    p1,p2,p3 = (ho_phi(z,j) for j in (1,2,3))
    h1,h2,h3 = (ho_phi(z/2,j) for j in (1,2,3))
    a52 = h2/2-p3+p2/4-h3/2
    a54 = h2/4-a52
    rows = [[],[h1/2],[h1/2-h2,h2],[p1-2*p2,p2,p2],
            [h1/2-2*a52-a54,a52,a52,a54]]
    return rows,[p1-3*p2+4*p3,mp.mpf(0),mp.mpf(0),-p2+4*p3,4*p2-8*p3]


def ho_tableau(z: mpf) -> tuple[list[list[mpf]], list[mpf]]:
    with mp.workdps(mp.dps+coefficient_guard_digits(z)):
        rows,weights = ho_tableau_values(z)
    return [[+v for v in row] for row in rows],[+v for v in weights]


def ho_step(state: Spectrum, time: mpf, dt: mpf, rhs: RightHandSide, nu: mpf) -> Spectrum:
    nodes = (mp.mpf(0),mp.mpf('0.5'),mp.mpf('0.5'),mp.mpf(1),mp.mpf('0.5'))
    z = {k:-nu*4*mp.pi**2*sum(m*m for m in k)*dt for k in state}
    tables = {k:ho_tableau(v) for k,v in z.items()}
    sources: list[Spectrum] = []
    for i,node in enumerate(nodes):
        stage = combine(state,{k:mp.exp(node*v) for k,v in z.items()},
                        [({k:dt*tables[k][0][i][j] for k in state},v) for j,v in enumerate(sources)])
        sources.append(rhs(stage,time+node*dt))
    return combine(state,{k:mp.exp(v) for k,v in z.items()},
                   [({k:dt*tables[k][1][j] for k in state},v) for j,v in enumerate(sources)])
