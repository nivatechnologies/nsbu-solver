//! Bounded admission for complete physical comparisons of an exact-v2 family.
mod work;
use super::QUANTITIES;
use crate::v2_experiment::{FamilyError, FamilyPlan};
use nsbu_solver::{
    diagnostics::{local::TensorErrors, physical::PhysicalComparisonWorkspace},
    domain::Layout,
    SolverError,
};
use work::{mul, work};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Work charged by physical measurement attempts.
pub struct PhysicalFamilyWork {
    /// Number of attempts charged.
    pub attempts: usize,
    /// Complete scalar inverse transforms charged.
    pub scalar_transforms: usize,
    /// Conservative weighted visits charged.
    pub weighted_visits: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Storage and work reservation for a physical V2 consumer.
pub struct PhysicalFamilyBounds {
    /// Consumer-owned storage reservation.
    pub storage_bytes: usize,
    /// Family plus consumer storage reservation.
    pub joint_storage_bytes: usize,
    /// Maximum attempts admitted.
    pub maximum_attempts: usize,
    /// Complete worst-case work reservation.
    pub work: PhysicalFamilyWork,
}

#[derive(Debug, Clone, Copy)]
/// Immutable physical policy borrowed from a V2 family admission.
pub struct PhysicalFamilyPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) bounds: PhysicalFamilyBounds,
    pub(super) per_attempt: PhysicalFamilyWork,
}

impl<'a> PhysicalFamilyPlan<'a> {
    /// Admit storage, positive floors and complete finite work before allocation.
    pub fn new(
        family: FamilyPlan<'a>,
        samples: Layout,
        floors: [f64; 4],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < family.times.as_slice().len() {
            return Err(SolverError::ResourceLimit.into());
        }
        for floor in floors {
            TensorErrors::<1>::new(samples.real_len(), floor)?;
        }
        let domain = family.branches[2].resources().domain();
        let storage_bytes = PhysicalComparisonWorkspace::reservation(domain, domain, samples)?
            // Two FFT/derivative streams and four reduction arrays use fewer than 32
            // allocations; reserve 64 bytes of allocator metadata for each slot.
            .checked_add(32 * 64)
            .ok_or(SolverError::SizeOverflow)?
            .checked_add(std::mem::size_of::<super::PhysicalFamilyWorkspace<'_>>())
            .and_then(|n| n.checked_add(std::mem::size_of::<super::PhysicalRefinementSample>()))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = storage_bytes
            .checked_add(family.bounds.storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = work(domain.layout(), samples)?;
        let work = PhysicalFamilyWork {
            attempts: maximum_attempts,
            scalar_transforms: mul(per_attempt.scalar_transforms, maximum_attempts)?,
            weighted_visits: mul(per_attempt.weighted_visits, maximum_attempts)?,
        };
        Ok(Self {
            family,
            samples,
            floors,
            bounds: PhysicalFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work,
            },
            per_attempt,
        })
    }
    /// Return the complete storage and work reservation.
    pub fn bounds(self) -> PhysicalFamilyBounds {
        self.bounds
    }
    /// Return the fixed diagnostic sample layout.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Return floors in [`super::QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
}
