//! Joint admission counts the V2 family once and all three physical consumers.
mod reservation;
use super::{SamplingSample, SamplingWorkspace};
use crate::v2_experiment::{physical::PhysicalFamilyPlan, FamilyError, FamilyPlan};
use nsbu_solver::{domain::Layout, SolverError};
use reservation::reservation;

/// Worst-case work charged for complete sampling attempts, including refusals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SamplingWork {
    /// Complete aggregate attempts.
    pub attempts: usize,
    /// Scalar inverse transforms across all physical children.
    pub scalar_transforms: usize,
    /// Conservative coefficient/sample/report visits.
    pub weighted_visits: usize,
}

/// Joint allocation and finite traversal reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingBounds {
    /// Three incremental consumers plus fixed aggregate/report storage.
    pub storage_bytes: usize,
    /// One V2 family plus every simultaneous consumer.
    pub joint_storage_bytes: usize,
    /// Complete work over the admitted attempts.
    pub work: SamplingWork,
}

/// Immutable nested physical sampling policy for one already admitted V2 family.
#[derive(Debug, Clone, Copy)]
pub struct SamplingPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) physical: [PhysicalFamilyPlan<'a>; 3],
    pub(super) bounds: SamplingBounds,
    pub(super) per_attempt: SamplingWork,
}
impl<'a> SamplingPlan<'a> {
    /// Admit strict componentwise sample refinement, fixed floors and complete work.
    /// No trajectory, FFT workspace or physical field buffer is allocated here.
    pub fn new(
        family: FamilyPlan<'a>,
        samples: [Layout; 3],
        floors: [f64; 4],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if samples.windows(2).any(|pair| {
            pair[0]
                .dimensions()
                .into_iter()
                .zip(pair[1].dimensions())
                .any(|(a, b)| a >= b || !b.is_multiple_of(a))
        }) {
            return Err(FamilyError::InvalidFamily);
        }
        let children = samples.map(|layout| {
            PhysicalFamilyPlan::new(family, layout, floors, maximum_attempts, joint_cap)
        });
        let [a, b, c] = children;
        let physical = [a?, b?, c?];
        let bounds = reservation(family, physical, maximum_attempts)?;
        if bounds.joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            family,
            physical,
            bounds,
            per_attempt: SamplingWork {
                attempts: 1,
                scalar_transforms: bounds.work.scalar_transforms / maximum_attempts,
                weighted_visits: bounds.work.weighted_visits / maximum_attempts,
            },
        })
    }
    /// Complete simultaneous-owner reservation and finite work.
    pub fn bounds(self) -> SamplingBounds {
        self.bounds
    }
    /// Exact admitted strictly nested sample layouts.
    pub fn sample_layouts(self) -> [Layout; 3] {
        self.physical.map(|child| child.sample_layout())
    }
    /// Fixed positive floors shared by every lattice and pair.
    pub fn relative_floors(self) -> [f64; 4] {
        self.physical[0].relative_floors()
    }
}
