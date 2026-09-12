use crate::{
    regions::CoveragePlan,
    v2_experiment::{reference::regional::RegionalTrackingPlan, FamilyError, FamilyPlan},
};
use nsbu_solver::SolverError;

/// Work charged for one complete core/annulus coverage attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoverageFamilyWork {
    /// Attempt count, including a refused measurement.
    pub attempts: usize,
    /// Exact repeated Simpson node evaluations for both nominal regions.
    pub geometry_evaluations: usize,
    /// Bounded branch/quantity sampled-metadata inspections.
    pub metadata_visits: usize,
}

/// Family and coverage-consumer reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageFamilyBounds {
    /// Consumer storage, including pending and last report.
    pub storage_bytes: usize,
    /// Joint family, regional-tracking, and consumer storage reservation.
    pub joint_storage_bytes: usize,
    /// Maximum work across all admitted attempts.
    pub work: CoverageFamilyWork,
}

/// Immutable three-setting nominal-coverage policy over an accepted V2 family.
#[derive(Debug, Clone, Copy)]
pub struct CoverageFamilyPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) coverage: [CoveragePlan; 3],
    pub(super) panels: [usize; 3],
    pub(super) regional_samples: nsbu_solver::domain::Layout,
    pub(super) regional_floors: [f64; 4],
    pub(super) bounds: CoverageFamilyBounds,
    pub(super) per_attempt: CoverageFamilyWork,
}
impl<'a> CoverageFamilyPlan<'a> {
    /// Admit three nested panel settings and family-plus-consumer storage before allocation.
    pub fn new(
        family: FamilyPlan<'a>,
        regional: RegionalTrackingPlan<'a>,
        panels: [usize; 3],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if regional.tracking_plan().family_plan().identity() != family.identity()
            || maximum_attempts < family.times().as_slice().len()
            || !(panels[0] < panels[1]
                && panels[1] < panels[2]
                && panels[1].is_multiple_of(panels[0])
                && panels[2].is_multiple_of(panels[1]))
        {
            return Err(FamilyError::InvalidFamily);
        }
        let coverage = panels.map(|panels| {
            let cap = panels
                .checked_mul(3)
                .and_then(|n| n.checked_add(2))
                .ok_or(FamilyError::from(SolverError::SizeOverflow))?;
            CoveragePlan::new(panels, cap).map_err(|_| FamilyError::InvalidFamily)
        });
        let coverage = [coverage[0]?, coverage[1]?, coverage[2]?];
        let per_attempt = work(panels)?;
        let storage_bytes = std::mem::size_of::<super::CoverageFamilyWorkspace<'_>>()
            .checked_add(2 * std::mem::size_of::<super::CoverageFamilySample>())
            .and_then(|n| n.checked_add(4096))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = regional
            .bounds()
            .joint_storage_bytes
            .checked_add(storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            family,
            coverage,
            panels,
            regional_samples: regional.tracking_plan().sample_layout(),
            regional_floors: regional.tracking_plan().relative_floors(),
            bounds: CoverageFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                work: scale(per_attempt, maximum_attempts)?,
            },
            per_attempt,
        })
    }
    /// Family, regional tracking, and this consumer reservation.
    pub fn bounds(self) -> CoverageFamilyBounds {
        self.bounds
    }
    /// Immutable accepted-family policy bound to this coverage schedule.
    pub fn family_plan(self) -> FamilyPlan<'a> {
        self.family
    }
    /// Nested coarse Simpson panel counts; each measurement also evaluates its doubled grid.
    pub fn panels(self) -> [usize; 3] {
        self.panels
    }
}
fn work(panels: [usize; 3]) -> Result<CoverageFamilyWork, SolverError> {
    let geometry_evaluations = panels
        .into_iter()
        .try_fold(0usize, |total, panels| {
            panels
                .checked_mul(3)
                .and_then(|n| n.checked_add(2))
                .and_then(|n| total.checked_add(n))
                .ok_or(SolverError::SizeOverflow)
        })?
        .checked_mul(2)
        .ok_or(SolverError::SizeOverflow)?;
    Ok(CoverageFamilyWork {
        attempts: 1,
        geometry_evaluations,
        metadata_visits: 24,
    })
}
fn scale(value: CoverageFamilyWork, attempts: usize) -> Result<CoverageFamilyWork, SolverError> {
    Ok(CoverageFamilyWork {
        attempts,
        geometry_evaluations: value
            .geometry_evaluations
            .checked_mul(attempts)
            .ok_or(SolverError::SizeOverflow)?,
        metadata_visits: value
            .metadata_visits
            .checked_mul(attempts)
            .ok_or(SolverError::SizeOverflow)?,
    })
}
