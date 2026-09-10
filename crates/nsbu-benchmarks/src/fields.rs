//! Degree-four implicit jets and manufactured force for the exact viscosity-one v2 case.
pub mod reference;
use crate::{
    jet::Jet,
    root::{self, RootReport},
    scalar,
    time::BenchmarkTime,
    BenchmarkError,
};

/// Pointwise manufactured data. These values never initialize an integrated trajectory.
#[derive(Debug, Clone)]
pub struct Evaluation {
    /// Analytical velocity.
    pub velocity: [f64; 3],
    /// Pressure before spatial mean removal.
    pub pressure_raw: f64,
    /// Prescribed physical force at viscosity one.
    pub force: [f64; 3],
    /// Spatial force derivatives, component then coordinate.
    pub force_gradient: [[f64; 3]; 3],
    /// Sum of absolute differentiated momentum terms; a cancellation diagnostic, not an error bound.
    pub gradient_term_magnitudes: [[f64; 3]; 3],
    /// Temporal, advective, viscous, and pressure-gradient terms per component.
    pub momentum_terms: [[f64; 4]; 3],
    /// Velocity divergence diagnostic.
    pub divergence: f64,
    /// Floating scalar root evidence; absent for exact flat regions.
    pub root: Option<RootReport>,
    /// Formal implicit-equation residual coefficients; floating estimates, not enclosures.
    pub root_residual: [f64; 70],
}

/// Solve the implicit equation through degree four with three formal Newton steps.
pub fn implicit_root(
    z: Jet,
    time: BenchmarkTime,
) -> Result<(Jet, Jet, RootReport), BenchmarkError> {
    let report = root::solve(z.value(), time.remaining(), 128)?;
    let tau = Jet::variable(-time.remaining(), 3)?.scale(-1.0)?;
    let one = Jet::constant(1.0)?;
    let zz = z.times(z)?;
    let mut q = Jet::constant(report.value)?;
    for _ in 0..3 {
        let residual = root_residual(q, zz, tau)?;
        let slope = one.minus(zz.times(q.powf(-0.75)?)?.scale(0.25)?)?;
        q = q.minus(residual.quotient(slope)?)?;
    }
    let residual = root_residual(q, zz, tau)?;
    Ok((q, residual, report))
}

/// Smooth step composition, with exact flat branches and stable logistic tails.
pub fn smooth_step(s: Jet) -> Result<Jet, BenchmarkError> {
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

fn root_residual(q: Jet, zz: Jet, tau: Jet) -> Result<Jet, BenchmarkError> {
    q.minus(zz.times(q.powf(0.25)?)?)?.minus(tau)
}

fn velocity_pressure(
    x: Jet,
    y: Jet,
    z: Jet,
    t: Jet,
    q: Jet,
) -> Result<([Jet; 3], Jet), BenchmarkError> {
    let xy = x.times(x)?.plus(y.times(y)?)?;
    let cutoff = cutoff(xy.plus(z.times(z)?)?, t)?;
    let radial = xy.quotient(q)?.scale(0.5)?;
    let (potential, swirl) = potentials(z, q, radial, cutoff)?;
    Ok((
        curl_potential(x, y, potential, swirl)?,
        pressure(q, radial, cutoff)?,
    ))
}

fn cutoff(radius: Jet, t: Jet) -> Result<Jet, BenchmarkError> {
    let shape = Jet::constant(441.0 / 2500.0)?
        .minus(radius)?
        .scale(625.0 / 54.0)?;
    smooth_step(shape)?.times(smooth_step(t.scale(512.0)?)?)
}

fn potentials(z: Jet, q: Jet, radial: Jet, cutoff: Jet) -> Result<(Jet, Jet), BenchmarkError> {
    let gaussian = radial.scale(-1.0)?.exp()?;
    let eta = z.times(q.powf(-3.0 / 8.0)?)?;
    let g = q
        .powf(-5.0 / 8.0)?
        .times(eta.plus(Jet::constant(1.0 / 32.0)?)?)?
        .times(gaussian)?
        .scale(0.5)?;
    let swirl = cutoff
        .times(q.powf(-9.0 / 8.0)?)?
        .times(gaussian)?
        .scale(0.25)?;
    Ok((cutoff.times(g)?, swirl))
}

fn curl_potential(x: Jet, y: Jet, potential: Jet, swirl: Jet) -> Result<[Jet; 3], BenchmarkError> {
    let dz = potential.derivative(2)?;
    Ok([
        x.times(dz)?.plus(y.times(swirl)?)?.scale(-1.0)?,
        y.times(dz)?.scale(-1.0)?.plus(x.times(swirl)?)?,
        potential
            .scale(2.0)?
            .plus(x.times(potential.derivative(0)?)?)?
            .plus(y.times(potential.derivative(1)?)?)?,
    ])
}

fn pressure(q: Jet, radial: Jet, cutoff: Jet) -> Result<Jet, BenchmarkError> {
    cutoff
        .times(cutoff)?
        .times(q.powf(-5.0 / 4.0)?)?
        .times(radial.scale(-2.0)?.exp()?)?
        .scale(-1.0 / 32.0)
}

/// Evaluate force and its first spatial derivatives without finite differences.
pub fn evaluate(point: [f64; 3], time: BenchmarkTime) -> Result<Evaluation, BenchmarkError> {
    let point = scalar::periodic(point)?;
    let mut result = Evaluation {
        velocity: [0.0; 3],
        pressure_raw: 0.0,
        force: [0.0; 3],
        force_gradient: [[0.0; 3]; 3],
        gradient_term_magnitudes: [[0.0; 3]; 3],
        momentum_terms: [[0.0; 4]; 3],
        divergence: 0.0,
        root: None,
        root_residual: [0.0; 70],
    };
    if point.iter().map(|v| v * v).sum::<f64>() >= 441.0 / 2500.0 || time.elapsed() == 0.0 {
        return Ok(result);
    }
    let x = Jet::variable(point[0], 0)?;
    let y = Jet::variable(point[1], 1)?;
    let z = Jet::variable(point[2], 2)?;
    let t = Jet::variable(time.elapsed(), 3)?;
    let (q, residual, report) = implicit_root(z, time)?;
    let (velocity, pressure) = velocity_pressure(x, y, z, t, q)?;
    result.velocity = velocity.map(Jet::value);
    result.pressure_raw = pressure.value();
    result.root = Some(report);
    result.root_residual = *residual.coefficients();
    assemble(&mut result, velocity, pressure)?;
    Ok(result)
}

fn assemble(
    result: &mut Evaluation,
    velocity: [Jet; 3],
    pressure: Jet,
) -> Result<(), BenchmarkError> {
    for i in 0..3 {
        let temporal = velocity[i].derivative(3)?;
        let (advective, viscous) =
            transport_terms(velocity, i, &mut result.gradient_term_magnitudes[i])?;
        let gradient = pressure.derivative(i)?;
        result.momentum_terms[i] = [
            temporal.value(),
            advective.value(),
            viscous.value(),
            gradient.value(),
        ];
        let force = temporal.plus(advective)?.plus(viscous)?.plus(gradient)?;
        result.force[i] = force.value();
        for j in 0..3 {
            result.force_gradient[i][j] = force.derivative(j)?.value();
            for term in [temporal, gradient] {
                result.gradient_term_magnitudes[i][j] += term.derivative(j)?.value().abs();
            }
        }
        result.divergence += velocity[i].derivative(i)?.value();
    }
    Ok(())
}

fn transport_terms(
    velocity: [Jet; 3],
    i: usize,
    magnitudes: &mut [f64; 3],
) -> Result<(Jet, Jet), BenchmarkError> {
    let mut advective = Jet::constant(0.0)?;
    let mut viscous = Jet::constant(0.0)?;
    for (j, component) in velocity.iter().enumerate() {
        let derivative = velocity[i].derivative(j)?;
        let product = component.times(derivative)?;
        let diffusion = derivative.derivative(j)?;
        advective = advective.plus(product)?;
        viscous = viscous.minus(diffusion)?;
        for (k, magnitude) in magnitudes.iter_mut().enumerate() {
            *magnitude +=
                product.derivative(k)?.value().abs() + diffusion.derivative(k)?.value().abs();
        }
    }
    Ok((advective, viscous))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_assembly_retains_component_signs_and_term_magnitudes() {
        let x = Jet::variable(2.0, 0).unwrap();
        let y = Jet::variable(3.0, 1).unwrap();
        let t = Jet::variable(1.0, 3).unwrap();
        let velocity = [
            x.times(t).unwrap(),
            y.powf(2.0).unwrap(),
            Jet::constant(0.0).unwrap(),
        ];
        let pressure = x.powf(2.0).unwrap().plus(y.powf(3.0).unwrap()).unwrap();
        let time =
            BenchmarkTime::new(nsbu_solver::domain::TickClock::from_rest(-10, 8).unwrap()).unwrap();
        let mut result = evaluate([0.0; 3], time).unwrap();
        assemble(&mut result, velocity, pressure).unwrap();
        assert_eq!(result.divergence, 7.0);
        assert_eq!(result.force, [8.0, 79.0, 0.0]);
        assert_eq!(
            result.force_gradient,
            [[4.0, 0.0, 0.0], [0.0, 72.0, 0.0], [0.0; 3]]
        );
        assert_eq!(result.gradient_term_magnitudes, result.force_gradient);
        assert_eq!(
            result.momentum_terms,
            [[2.0, 2.0, 0.0, 4.0], [0.0, 54.0, -2.0, 27.0], [0.0; 4]]
        );
    }
    #[test]
    fn transport_magnitude_includes_diffusion_gradient() {
        let x = Jet::variable(1.0, 0).unwrap();
        let velocity = [
            x.powf(3.0).unwrap(),
            Jet::constant(0.0).unwrap(),
            Jet::constant(0.0).unwrap(),
        ];
        let mut magnitudes = [0.0; 3];
        let (advection, diffusion) = transport_terms(velocity, 0, &mut magnitudes).unwrap();
        assert_eq!(advection.value(), 3.0);
        assert_eq!(diffusion.value(), -6.0);
        assert_eq!(magnitudes, [21.0, 0.0, 0.0]);
    }
}
