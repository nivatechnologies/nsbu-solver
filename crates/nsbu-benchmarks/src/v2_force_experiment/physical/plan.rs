//! Joint resource and complete-work admission for physical force-grid comparisons.
use super::{ForcePhysicalError, ForcePhysicalSample, ForcePhysicalWorkspace};
use crate::{
    v2_experiment::physical::plan::work::{mul, work_for_pairs},
    v2_force_experiment::ForceFamilyPlan,
};
use nsbu_solver::{
    diagnostics::{local::TensorErrors, physical::PhysicalComparisonWorkspace},
    domain::Layout,
    SolverError,
};

/// Charged physical sampling work, including failed attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ForcePhysicalWork {
    /// Whole report attempts.
    pub attempts: usize,
    /// Complete inverse transforms for all quantities and both pairs.
    pub scalar_transforms: usize,
    /// Conservative source, sampling, reduction, and extrema visits.
    pub weighted_visits: usize,
}

/// Consumer storage and finite total work admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForcePhysicalBounds {
    /// Consumer workspace, allocation metadata, and one retained report.
    pub storage_bytes: usize,
    /// Force family plus consumer simultaneous storage.
    pub joint_storage_bytes: usize,
    /// Maximum whole attempts.
    pub maximum_attempts: usize,
    /// Complete maximum-attempt work allowance.
    pub work: ForcePhysicalWork,
}

/// Immutable physical policy bound to one exact force-family identity and manifest.
#[derive(Debug, Clone, Copy)]
pub struct ForcePhysicalPlan<'a> {
    pub(super) family: ForceFamilyPlan<'a>,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) bounds: ForcePhysicalBounds,
    pub(super) per_attempt: ForcePhysicalWork,
}
impl<'a> ForcePhysicalPlan<'a> {
    /// Validate all floors, work, and joint storage before consumer allocation.
    pub fn new(
        family: ForceFamilyPlan<'a>,
        samples: Layout,
        floors: [f64; 4],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, ForcePhysicalError> {
        if maximum_attempts < family.times().as_slice().len() {
            return Err(SolverError::ResourceLimit.into());
        }
        for floor in floors {
            TensorErrors::<1>::new(samples.real_len(), floor)?;
        }
        let domain = family
            .branch_plan(0)
            .ok_or(ForcePhysicalError::InvalidFamily)?
            .resources()
            .domain();
        let storage_bytes = PhysicalComparisonWorkspace::reservation(domain, domain, samples)?
            // The two derivative streams and four vectors allocate fewer than 32
            // blocks; 64 bytes per slot is a conservative allocator-metadata allowance.
            .checked_add(32 * 64)
            .and_then(|n| n.checked_add(std::mem::size_of::<ForcePhysicalWorkspace<'_>>()))
            .and_then(|n| n.checked_add(std::mem::size_of::<ForcePhysicalSample>()))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = family
            .bounds()
            .storage_bytes
            .checked_add(storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let physical = work_for_pairs(domain.layout(), samples, 2)?;
        let per_attempt = ForcePhysicalWork {
            attempts: 1,
            scalar_transforms: physical.scalar_transforms,
            weighted_visits: physical.weighted_visits,
        };
        let work = ForcePhysicalWork {
            attempts: maximum_attempts,
            scalar_transforms: mul(per_attempt.scalar_transforms, maximum_attempts)?,
            weighted_visits: mul(per_attempt.weighted_visits, maximum_attempts)?,
        };
        Ok(Self {
            family,
            samples,
            floors,
            bounds: ForcePhysicalBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work,
            },
            per_attempt,
        })
    }
    /// Complete consumer and joint-family reservation.
    pub fn bounds(self) -> ForcePhysicalBounds {
        self.bounds
    }
    /// Bound exact force-family admission.
    pub fn family_plan(self) -> ForceFamilyPlan<'a> {
        self.family
    }
    /// Common physical sample lattice.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Floors in [`super::FORCE_PHYSICAL_QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
}
