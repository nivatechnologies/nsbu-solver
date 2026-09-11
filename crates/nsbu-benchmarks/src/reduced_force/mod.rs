//! Experimental exact-v2 force values using axisymmetric potentials in three variables.
//!
//! The full Cartesian PDE and prescribed problem remain unchanged. This evaluator changes
//! floating arithmetic order and supplies no force-gradient or numerical-accuracy certificate.
//! It is not used by the default force providers. The degree-four Cartesian evaluator remains
//! an independent comparison path; same-input high-precision comparisons are still required.
pub(crate) mod axial;
mod jet;
mod potentials;
mod transport;
use crate::{root::RootReport, scalar, time::BenchmarkTime, BenchmarkError};
use jet::Jet;
/// Force-value arithmetic and conditioning diagnostics; no integrated state is carried here.
#[derive(Debug, Clone)]
pub struct ForceSample {
    /// Cartesian prescribed-force values at viscosity one.
    pub force: [f64; 3],
    /// Temporal, advective, viscous and pressure-gradient terms, component first.
    pub momentum_terms: [[f64; 4]; 3],
    /// Reference velocity from the same potentials, for evaluator comparisons only.
    pub velocity: [f64; 3],
    /// Raw kinematic pressure, before the separately required periodic mean removal.
    pub pressure_raw: f64,
    /// Safeguarded scalar root report; absent at rest and outside support.
    pub root: Option<RootReport>,
    /// Complete degree-three implicit residual in (w,z,t); estimates, not enclosures.
    pub root_residual: [f64; 20],
}
/// Evaluate all physical force terms using total degree three in w=x*x+y*y,z,t.
/// Every call has bounded scalar/formal iterations and fixed stack storage.
pub fn evaluate(point: [f64; 3], time: BenchmarkTime) -> Result<ForceSample, BenchmarkError> {
    let point = scalar::periodic(point)?;
    let Some(point) = active_point(point, time) else {
        return Ok(zero());
    };
    let z = Jet::variable(point[2], 1)?;
    let (q, residual, report) = potentials::implicit(z, time)?;
    rooted(point, time, (q, residual, report))
}

pub(crate) fn active_point(point: [f64; 3], time: BenchmarkTime) -> Option<[f64; 3]> {
    if point.iter().map(|v| v * v).sum::<f64>() >= 441.0 / 2500.0 || time.elapsed() == 0.0 {
        None
    } else {
        Some(point)
    }
}

fn rooted(
    point: [f64; 3],
    time: BenchmarkTime,
    (q, residual, report): (Jet, Jet, RootReport),
) -> Result<ForceSample, BenchmarkError> {
    let [x, y, z] = point;
    let w = Jet::variable(x * x + y * y, 0)?;
    let z = Jet::variable(z, 1)?;
    let t = Jet::variable(time.elapsed(), 2)?;
    let [a, b, p] = potentials::evaluate(w, z, t, q)?;
    let (velocity, momentum_terms) = transport::evaluate([x, y], w, a, b, p)?;
    let force = momentum_terms.map(|terms| ((terms[0] + terms[1]) + terms[2]) + terms[3]);
    if force
        .iter()
        .chain(velocity.iter())
        .chain(momentum_terms.iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err(BenchmarkError::ArithmeticResolution);
    }
    Ok(ForceSample {
        force,
        momentum_terms,
        velocity,
        pressure_raw: p.value(),
        root: Some(report),
        root_residual: *residual.coefficients(),
    })
}

fn zero() -> ForceSample {
    ForceSample {
        force: [0.0; 3],
        momentum_terms: [[0.0; 4]; 3],
        velocity: [0.0; 3],
        pressure_raw: 0.0,
        root: None,
        root_residual: [0.0; 20],
    }
}
