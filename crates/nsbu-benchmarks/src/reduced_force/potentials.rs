//! Exact-v2 implicit root and cutoff poloidal/swirl/pressure potentials in (w,z,t).
use super::jet::Jet;
use crate::{
    root::{self, RootReport},
    time::BenchmarkTime,
    BenchmarkError,
};
pub(super) fn implicit(
    z: Jet,
    time: BenchmarkTime,
) -> Result<(Jet, Jet, RootReport), BenchmarkError> {
    let report = root::solve(z.value(), time.remaining(), 128)?;
    let tau = Jet::variable(-time.remaining(), 2)?.scale(-1.0)?;
    let one = Jet::constant(1.0)?;
    let zz = z.times(z)?;
    let mut q = Jet::constant(report.value)?;
    // Three fixed formal Newton corrections retain the established scalar iteration policy.
    for _ in 0..3 {
        let residual = residual(q, zz, tau)?;
        let slope = one.minus(zz.times(q.powf(-0.75)?)?.scale(0.25)?)?;
        q = q.minus(residual.quotient(slope)?)?;
    }
    Ok((q, residual(q, zz, tau)?, report))
}
fn residual(q: Jet, zz: Jet, tau: Jet) -> Result<Jet, BenchmarkError> {
    q.minus(zz.times(q.powf(0.25)?)?)?.minus(tau)
}
pub(super) fn evaluate(w: Jet, z: Jet, t: Jet, q: Jet) -> Result<[Jet; 3], BenchmarkError> {
    let cutoff = cutoff(w.plus(z.times(z)?)?, t)?;
    let radial = w.quotient(q)?.scale(0.5)?;
    let gaussian = radial.scale(-1.0)?.exp()?;
    let a = poloidal(z, q, gaussian, cutoff)?;
    let b = cutoff
        .times(q.powf(-9.0 / 8.0)?)?
        .times(gaussian)?
        .scale(0.25)?;
    Ok([a, b, pressure(q, radial, cutoff)?])
}
fn poloidal(z: Jet, q: Jet, gaussian: Jet, cutoff: Jet) -> Result<Jet, BenchmarkError> {
    let eta = z.times(q.powf(-3.0 / 8.0)?)?;
    cutoff
        .times(q.powf(-5.0 / 8.0)?)?
        .times(eta.plus(Jet::constant(1.0 / 32.0)?)?)?
        .times(gaussian)?
        .scale(0.5)
}
fn pressure(q: Jet, radial: Jet, cutoff: Jet) -> Result<Jet, BenchmarkError> {
    cutoff
        .times(cutoff)?
        .times(q.powf(-5.0 / 4.0)?)?
        .times(radial.scale(-2.0)?.exp()?)?
        .scale(-1.0 / 32.0)
}
fn cutoff(radius: Jet, t: Jet) -> Result<Jet, BenchmarkError> {
    let shape = Jet::constant(441.0 / 2500.0)?
        .minus(radius)?
        .scale(625.0 / 54.0)?;
    smooth_step(shape)?.times(smooth_step(t.scale(512.0)?)?)
}
fn smooth_step(s: Jet) -> Result<Jet, BenchmarkError> {
    if s.value() <= 0.0 {
        return Jet::constant(0.0);
    }
    if s.value() >= 1.0 {
        return Jet::constant(1.0);
    }
    let one = Jet::constant(1.0)?;
    let ratio = one.quotient(one.minus(s)?)?.minus(one.quotient(s)?)?;
    if ratio.value() <= 0.0 {
        let exponential = ratio.exp()?;
        exponential.quotient(one.plus(exponential)?)
    } else {
        one.quotient(one.plus(ratio.scale(-1.0)?.exp()?)?)
    }
}
