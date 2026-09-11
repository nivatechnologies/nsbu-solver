//! Separate admission for repeated exact-v2 regional classification and retained reports.
use super::{RegionalTrackingSample, RegionalTrackingWorkspace};
use crate::v2_experiment::{
    reference::{ReferenceTrackingPlan, ReferenceTrackingWork},
    FamilyError,
};
use nsbu_solver::SolverError;

/// Added sampled-region work, separate from analytical tracking and integration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegionalTrackingWork {
    /// Complete regional report attempts.
    pub attempts: usize,
    /// Point classifications across six branches and four quantities.
    pub classifications: usize,
    /// Conservative classification root iterations.
    pub root_iterations: usize,
    /// Magnitude visits supplied to the regional collectors.
    pub magnitude_visits: usize,
}

/// Complete tracking and added regional storage/work bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionalTrackingBounds {
    /// Added retained-report and workspace reservation.
    pub regional_storage_bytes: usize,
    /// Family, analytical tracking and regional reservation.
    pub joint_storage_bytes: usize,
    /// Unchanged underlying analytical tracking allowance.
    pub tracking_work: ReferenceTrackingWork,
    /// Added repeated regional classification allowance.
    pub regional_work: RegionalTrackingWork,
}

/// Immutable regional policy over a separately admitted analytical tracking plan.
#[derive(Debug, Clone, Copy)]
pub struct RegionalTrackingPlan<'a> {
    pub(super) tracking: ReferenceTrackingPlan<'a>,
    pub(super) bounds: RegionalTrackingBounds,
    pub(super) per_attempt: RegionalTrackingWork,
    pub(super) root_budget: usize,
}
impl<'a> RegionalTrackingPlan<'a> {
    /// Admit all 24 repeated classifiers and report retention before evaluation.
    pub fn new(
        tracking: ReferenceTrackingPlan<'a>,
        root_budget: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if root_budget != 128 {
            return Err(FamilyError::InvalidFamily);
        }
        let attempts = tracking.bounds().work.attempts;
        let per_attempt = work(tracking.sample_layout().real_len(), root_budget)?;
        let reports = std::mem::size_of::<RegionalTrackingSample>()
            .checked_mul(2)
            .ok_or(SolverError::SizeOverflow)?;
        let regional_storage_bytes = std::mem::size_of::<RegionalTrackingWorkspace<'_>>()
            .checked_add(reports)
            .and_then(|n| n.checked_add(4096))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = tracking
            .bounds()
            .joint_storage_bytes
            .checked_add(regional_storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            tracking,
            bounds: RegionalTrackingBounds {
                regional_storage_bytes,
                joint_storage_bytes,
                tracking_work: tracking.bounds().work,
                regional_work: scale(per_attempt, attempts)?,
            },
            per_attempt,
            root_budget,
        })
    }
    /// Joint tracking/regional resource and work bounds.
    pub fn bounds(self) -> RegionalTrackingBounds {
        self.bounds
    }
    /// Fixed classification root allowance, currently the evaluator's full 128 iterations.
    pub fn root_budget(self) -> usize {
        self.root_budget
    }
    /// Underlying analytical tracking policy, unchanged by regional attachment.
    pub fn tracking_plan(self) -> ReferenceTrackingPlan<'a> {
        self.tracking
    }
}

fn work(points: usize, root_budget: usize) -> Result<RegionalTrackingWork, SolverError> {
    let classifications = points.checked_mul(24).ok_or(SolverError::SizeOverflow)?;
    Ok(RegionalTrackingWork {
        attempts: 1,
        classifications,
        root_iterations: classifications
            .checked_mul(root_budget)
            .ok_or(SolverError::SizeOverflow)?,
        magnitude_visits: classifications,
    })
}
fn scale(work: RegionalTrackingWork, attempts: usize) -> Result<RegionalTrackingWork, SolverError> {
    Ok(RegionalTrackingWork {
        attempts,
        classifications: mul(work.classifications, attempts)?,
        root_iterations: mul(work.root_iterations, attempts)?,
        magnitude_visits: mul(work.magnitude_visits, attempts)?,
    })
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
