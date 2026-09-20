//! Pinned per-point analytical cache for one exact clock and sample lattice.
//!
//! The velocity-side cache reuses the reviewed independent evaluator output
//! (`ReferenceEvaluation`) verbatim, exactly as the reviewed v2 analytical tracking
//! caches one evaluation per clock/grid point. The pressure-side cache stores only
//! the four pressure components (`[raw, d/dx, d/dy, d/dz]`) so the doubled-grid
//! pressure lattice stays representable within the admitted memory ledger.
use nsbu_benchmarks::{
    fields::reference::{self, ReferenceEvaluation},
    time::BenchmarkTime,
};
use nsbu_solver::{domain::Layout, SolverError};

/// Zero-valued analytical record shared by cache construction refusals.
pub(crate) fn zero_reference() -> ReferenceEvaluation {
    ReferenceEvaluation {
        velocity: [0.0; 3],
        gradient: [[0.0; 3]; 3],
        hessian: [[[0.0; 3]; 3]; 3],
        vorticity: [0.0; 3],
        pressure_raw: 0.0,
        pressure_gradient: [0.0; 3],
        root: None,
    }
}

pub(crate) fn zero_pressure_row() -> [f64; 4] {
    [0.0; 4]
}

/// Unit-periodic unaligned sample point in x-major, z-fastest order.
pub(crate) fn point(layout: Layout, index: usize) -> [f64; 3] {
    let [nx, ny, nz] = layout.dimensions();
    [
        (index / (ny * nz)) as f64 / nx as f64,
        ((index / nz) % ny) as f64 / ny as f64,
        (index % nz) as f64 / nz as f64,
    ]
}

/// Allocate without evaluating anything; only the admitted reservation reaches here.
pub(crate) fn filled<T: Clone>(count: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(count, value);
    Ok(values)
}

/// Evaluate the reviewed reference once per velocity-lattice point at the exact clock.
/// Points are the unchanged unaligned lattice positions; nothing is recentered,
/// aligned or phase-shifted, and no integrated state is read as an initializer.
pub(crate) fn velocity_cache(
    samples: Layout,
    time: BenchmarkTime,
) -> Result<Vec<ReferenceEvaluation>, SolverError> {
    let mut cache = filled(samples.real_len(), zero_reference())?;
    for (index, output) in cache.iter_mut().enumerate() {
        *output = reference::evaluate(point(samples, index), time)
            .map_err(|_| SolverError::InvalidPayload)?;
    }
    Ok(cache)
}

/// Evaluate the analytical pressure row once per doubled-grid pressure sample.
pub(crate) fn pressure_cache(
    samples: Layout,
    time: BenchmarkTime,
) -> Result<Vec<[f64; 4]>, SolverError> {
    let mut cache = filled(samples.real_len(), zero_pressure_row())?;
    for (index, output) in cache.iter_mut().enumerate() {
        let value = reference::evaluate(point(samples, index), time)
            .map_err(|_| SolverError::InvalidPayload)?;
        *output = [
            value.pressure_raw,
            value.pressure_gradient[0],
            value.pressure_gradient[1],
            value.pressure_gradient[2],
        ];
    }
    Ok(cache)
}

/// Ordered empirical mean over the sample lattice; the gauge witness is explicit.
pub(crate) fn lattice_mean(values: &[[f64; 4]], column: usize) -> Result<f64, SolverError> {
    if values.is_empty() {
        return Err(SolverError::InvalidPayload);
    }
    let mut sum = 0_f64;
    for row in values {
        let value = row[column];
        if !value.is_finite() {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        sum += value;
    }
    let mean = sum / values.len() as f64;
    if mean.is_finite() {
        Ok(mean)
    } else {
        Err(SolverError::ArithmeticResolutionLimited)
    }
}

/// Coefficient SHA-256 over borrowed components, used to prove read-only observation.
pub(crate) fn coefficient_sha256(coefficients: [&[nsbu_solver::Complex64]; 3]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for component in coefficients {
        for value in component {
            hash.update(value.re.to_bits().to_le_bytes());
            hash.update(value.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}
