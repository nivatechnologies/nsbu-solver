//! Checked simultaneous-owner and finite-work aggregation.
use super::{
    FamilyPlan, PhysicalFamilyPlan, SamplingBounds, SamplingSample, SamplingWork, SamplingWorkspace,
};
use nsbu_solver::SolverError;

pub(super) fn reservation(
    family: FamilyPlan<'_>,
    physical: [PhysicalFamilyPlan<'_>; 3],
    attempts: usize,
) -> Result<SamplingBounds, SolverError> {
    let reports = std::mem::size_of::<SamplingSample>()
        .checked_mul(4)
        .ok_or(SolverError::SizeOverflow)?;
    let mut storage = std::mem::size_of::<SamplingWorkspace<'_>>()
        .checked_add(reports)
        .ok_or(SolverError::SizeOverflow)?;
    let mut work = SamplingWork {
        attempts,
        ..SamplingWork::default()
    };
    for child in physical {
        let bounds = child.bounds();
        storage = storage
            .checked_add(bounds.storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        work.scalar_transforms = work
            .scalar_transforms
            .checked_add(bounds.work.scalar_transforms)
            .ok_or(SolverError::SizeOverflow)?;
        work.weighted_visits = work
            .weighted_visits
            .checked_add(bounds.work.weighted_visits)
            .ok_or(SolverError::SizeOverflow)?;
    }
    work.weighted_visits = work
        .weighted_visits
        .checked_add(
            attempts
                .checked_mul(2048)
                .ok_or(SolverError::SizeOverflow)?,
        )
        .ok_or(SolverError::SizeOverflow)?;
    Ok(SamplingBounds {
        storage_bytes: storage,
        joint_storage_bytes: storage
            .checked_add(family.bounds().storage_bytes)
            .ok_or(SolverError::SizeOverflow)?,
        work,
    })
}
