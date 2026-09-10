"""Continuum CyclicSine coefficients; convolution is independent of the Rust force formula."""
from mpmath import mp, mpf, mpc
from reference.dft import Mode, Spectrum, project, retained
from reference.tuples import triple


def profile(n: int, time: mpf) -> tuple[Spectrum, Spectrum]:
    """Return analytical velocity and its time derivative without accessing an evolved state."""
    if not mp.isfinite(time) or time < 0:
        raise ValueError('CyclicSine time must be finite and nonnegative')
    zero = (mp.mpc(0), mp.mpc(0), mp.mpc(0))
    state: Spectrum = {k: zero for k in retained(n)}
    derivative: Spectrum = {k: zero for k in state}
    for axis, frequency in enumerate((13, 17, 19)):
        direction = (axis + 1) % 3
        for sign in (-1, 1):
            mode = triple(sign if j == direction else 0 for j in range(3))
            values = [mp.mpc(0)] * 3
            values[axis] = sign * mp.sin(frequency * time) / (2 * mp.j)
            state[mode] = triple(values)
            values[axis] = sign * frequency * mp.cos(frequency * time) / (2 * mp.j)
            derivative[mode] = triple(values)
    return state, derivative


def projected_force(n: int, time: mpf) -> Spectrum:
    """Construct P(u_t + (u dot grad)u - Delta u) by sparse continuum convolution.

    The domain is the unit cube and viscosity is one. This supplies the projected
    velocity equation only; it is not a physical-pressure reference or a sampled
    forcing-tail certificate. No production FFT, force evaluator or state is used.
    """
    state, derivative = profile(n, time)
    result: dict[Mode, list[mpc]] = {
        k: [derivative[k][c] + 4 * mp.pi**2 * sum(v*v for v in k) * u[c]
            for c in range(3)] for k, u in state.items()
    }
    sparse = [(k, u) for k, u in state.items() if any(u)]
    for p, u in sparse:
        for q, v in sparse:
            target = triple(a+b for a, b in zip(p, q))
            if target in result:
                factor = 2 * mp.pi * mp.j * sum((a*b for a, b in zip(u, q)), mp.mpc(0))
                for c in range(3):
                    result[target][c] += factor * v[c]
    return {k: project(k, triple(v)) for k, v in result.items()}
