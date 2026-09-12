//! Checked per-attempt physical sampling and extrema-scan work.
use super::{PhysicalFamilyWork, QUANTITIES};
use nsbu_solver::{domain::Layout, SolverError};

pub(super) fn work(source: Layout, samples: Layout) -> Result<PhysicalFamilyWork, SolverError> {
    work_for_pairs(source, samples, 5)
}

pub(crate) fn work_for_pairs(
    source: Layout,
    samples: Layout,
    pairs: usize,
) -> Result<PhysicalFamilyWork, SolverError> {
    let scalar_transforms = mul(
        pairs,
        QUANTITIES
            .into_iter()
            .map(|quantity| quantity.scalar_transforms())
            .sum::<usize>(),
    )?;
    let reductions = mul(10, mul(pairs, QUANTITIES.len())?)?;
    let extrema = mul(3, mul(pairs, QUANTITIES.len())?)?;
    let per_sample = mul(4, scalar_transforms)?
        .checked_add(reductions)
        .and_then(|n| n.checked_add(extrema))
        .ok_or(SolverError::SizeOverflow)?;
    let weighted_visits = mul(source.half_len(), mul(scalar_transforms, 6)?)?
        .checked_add(mul(
            samples.real_len(),
            // Sampling plus three explicit extrema scans for every pair/quantity.
            per_sample,
        )?)
        .and_then(|n| n.checked_add(1024))
        .ok_or(SolverError::SizeOverflow)?;
    Ok(PhysicalFamilyWork {
        attempts: 1,
        scalar_transforms,
        weighted_visits,
    })
}

pub(crate) fn mul(left: usize, right: usize) -> Result<usize, SolverError> {
    left.checked_mul(right).ok_or(SolverError::SizeOverflow)
}
