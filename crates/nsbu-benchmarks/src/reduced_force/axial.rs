//! Per-call cache for reduced degree-three implicit roots, keyed by plane and clock.
use super::jet::Jet;
use super::{rooted, ForceSample};
use crate::{root::RootReport, scalar, time::BenchmarkTime, BenchmarkError};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub(crate) struct AxialRoot {
    z: u64,
    time: [u64; 4],
    q: Jet,
    residual: Jet,
    report: RootReport,
}

impl AxialRoot {
    pub(crate) fn new(z: f64, time: BenchmarkTime) -> Result<Option<Self>, BenchmarkError> {
        let z = scalar::periodic([0.0, 0.0, z])?[2];
        if time.elapsed() == 0.0 || z * z >= 441.0 / 2500.0 {
            return Ok(None);
        }
        let (q, residual, report) = super::potentials::implicit(Jet::variable(z, 1)?, time)?;
        Ok(Some(Self {
            z: z.to_bits(),
            time: key(time),
            q,
            residual,
            report,
        }))
    }

    pub(crate) fn iterations(self) -> usize {
        self.report.iterations
    }
}

/// Use only an exact matching plane cache for an active point; flat values need no root.
pub(crate) fn evaluate(
    point: [f64; 3],
    time: BenchmarkTime,
    root: Option<&AxialRoot>,
) -> Result<ForceSample, BenchmarkError> {
    let point = scalar::periodic(point)?;
    let Some(point) = super::active_point(point, time) else {
        return Ok(super::zero());
    };
    let root = root.ok_or(BenchmarkError::InvalidInput)?;
    if root.z != point[2].to_bits() || root.time != key(time) {
        return Err(BenchmarkError::InvalidInput);
    }
    rooted(point, time, (root.q, root.residual, root.report))
}

fn key(time: BenchmarkTime) -> [u64; 4] {
    let errors = time.rounding_estimates();
    [
        time.elapsed().to_bits(),
        time.remaining().to_bits(),
        errors[0].to_bits(),
        errors[1].to_bits(),
    ]
}
