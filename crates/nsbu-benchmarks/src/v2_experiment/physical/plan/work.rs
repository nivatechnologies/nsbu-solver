//! Checked per-attempt physical sampling and extrema-scan work.
use super::{PhysicalFamilyWork, QUANTITIES};
use nsbu_solver::{diagnostics::derivatives::DerivativeWorkspace, domain::Layout, SolverError};

pub(super) fn work(source: Layout, samples: Layout) -> Result<PhysicalFamilyWork, SolverError> {
    let scalar_transforms = 5 * QUANTITIES
        .into_iter()
        .map(|quantity| quantity.scalar_transforms())
        .sum::<usize>();
    let weighted_visits = mul(
        DerivativeWorkspace::coefficient_visits(source)?,
        scalar_transforms,
    )?
    .checked_add(mul(
        samples.real_len(),
        // Sampling plus three explicit extrema scans for every pair/quantity.
        4 * scalar_transforms + 10 * 5 * QUANTITIES.len() + 3 * 5 * QUANTITIES.len(),
    )?)
    .and_then(|n| n.checked_add(1024))
    .ok_or(SolverError::SizeOverflow)?;
    Ok(PhysicalFamilyWork {
        attempts: 1,
        scalar_transforms,
        weighted_visits,
    })
}

pub(super) fn mul(left: usize, right: usize) -> Result<usize, SolverError> {
    left.checked_mul(right).ok_or(SolverError::SizeOverflow)
}
