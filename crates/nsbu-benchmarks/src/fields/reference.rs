//! Analytical velocity derivatives and pressure for read-only v2 physical comparisons.
//!
//! This uses the Rust jet evaluator and unchanged mathematical fields. The independent
//! Python high-precision evaluator supplies accuracy fixtures; this module alone does
//! not establish independent arithmetic or current-grid reference qualification.
use super::{implicit_root, velocity_pressure};
use crate::{jet::Jet, root::RootReport, scalar, time::BenchmarkTime, BenchmarkError};

/// Pointwise derivatives in fixed physical coordinates, before any region masking.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceEvaluation {
    /// Analytical velocity, never an integrated-state initializer.
    pub velocity: [f64; 3],
    /// Velocity component followed by spatial coordinate.
    pub gradient: [[f64; 3]; 3],
    /// Velocity component followed by two ordered spatial coordinates.
    /// Both mixed entries are retained; Frobenius norms count both entries.
    pub hessian: [[[f64; 3]; 3]; 3],
    /// Curl computed from the analytical velocity gradient.
    pub vorticity: [f64; 3],
    /// Analytical pressure before subtracting its independently measured spatial mean.
    pub pressure_raw: f64,
    /// Spatial pressure gradient, independent of the mean-pressure gauge.
    pub pressure_gradient: [f64; 3],
    /// Floating root evidence; absent for exact rest and exterior flat regions.
    pub root: Option<RootReport>,
}

/// Evaluate physical derivatives without finite differences or any evolving-state input.
/// A global pressure comparison must separately remove the analytical spatial mean;
/// local pressure means must never be removed independently to suppress collar errors.
pub fn evaluate(
    point: [f64; 3],
    time: BenchmarkTime,
) -> Result<ReferenceEvaluation, BenchmarkError> {
    let point = scalar::periodic(point)?;
    let mut result = ReferenceEvaluation {
        velocity: [0.0; 3],
        gradient: [[0.0; 3]; 3],
        hessian: [[[0.0; 3]; 3]; 3],
        vorticity: [0.0; 3],
        pressure_raw: 0.0,
        pressure_gradient: [0.0; 3],
        root: None,
    };
    if time.elapsed() == 0.0 || point.iter().map(|v| v * v).sum::<f64>() >= 441.0 / 2500.0 {
        return Ok(result);
    }
    let x = Jet::variable(point[0], 0)?;
    let y = Jet::variable(point[1], 1)?;
    let z = Jet::variable(point[2], 2)?;
    let t = Jet::variable(time.elapsed(), 3)?;
    let (q, _, root) = implicit_root(z, time)?;
    let (velocity, pressure) = velocity_pressure(x, y, z, t, q)?;
    result.root = Some(root);
    result.velocity = velocity.map(Jet::value);
    result.pressure_raw = pressure.value();
    derivatives(&mut result, velocity, pressure)?;
    Ok(result)
}

fn derivatives(
    result: &mut ReferenceEvaluation,
    velocity: [Jet; 3],
    pressure: Jet,
) -> Result<(), BenchmarkError> {
    for (component, value) in velocity.into_iter().enumerate() {
        result.pressure_gradient[component] = pressure.derivative(component)?.value();
        for axis in 0..3 {
            let first = value.derivative(axis)?;
            result.gradient[component][axis] = first.value();
            for second in 0..3 {
                result.hessian[component][axis][second] = first.derivative(second)?.value();
            }
        }
    }
    let g = result.gradient;
    result.vorticity = [g[2][1] - g[1][2], g[0][2] - g[2][0], g[1][0] - g[0][1]];
    Ok(())
}
