use super::{ProbeRegionalSample, ProbeRegionalWorkspace};
use crate::v2_experiment::{
    probes::reference::{ProbeReferencePlan, ProbeReferenceWork},
    reference::{self, regional::RegionalTrackingWork},
    FamilyError,
};
use nsbu_solver::{domain::Domain, SolverError};

/// Consumer storage and complete simultaneous owner admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeRegionalBounds {
    /// Independent analytical/regional scratch, report and allocator allowance.
    pub storage_bytes: usize,
    /// Probe family, reference producer and this consumer counted once each.
    pub joint_storage_bytes: usize,
    /// Maximum complete attempts, including a failed charged attempt.
    pub maximum_attempts: usize,
    /// Work for independently recomputing global values before regional reduction.
    pub tracking_work: ProbeReferenceWork,
    /// Work for the 24 regional classifiers per attempt.
    pub regional_work: RegionalTrackingWork,
}

/// Immutable regional policy bound to one admitted probe-reference producer.
#[derive(Debug, Clone, Copy)]
pub struct ProbeRegionalPlan<'a> {
    pub(super) reference: ProbeReferencePlan<'a>,
    pub(super) sources: [Domain; 6],
    pub(super) root_budget: usize,
    pub(super) bounds: ProbeRegionalBounds,
    pub(super) per_tracking: ProbeReferenceWork,
    pub(super) per_regional: RegionalTrackingWork,
}

impl<'a> ProbeRegionalPlan<'a> {
    /// Admit the producer, independent scratch, all classifiers and retained report.
    pub fn new(
        reference: ProbeReferencePlan<'a>,
        root_budget: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if root_budget != 128 {
            return Err(FamilyError::InvalidFamily);
        }
        let attempts = reference.bounds().maximum_attempts;
        let samples = reference.sample_layout();
        let probes = reference.probe_plan();
        let sources = probes.branches.map(|branch| branch.resources().domain());
        let unique = [sources[0], sources[1], sources[2]];
        let storage_bytes = reference::kernel_reservation(unique, samples)?
            .checked_add(std::mem::size_of::<ProbeRegionalWorkspace<'_>>())
            .and_then(|n| n.checked_add(std::mem::size_of::<ProbeRegionalSample>()))
            .and_then(|n| n.checked_add(4096))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = reference
            .bounds()
            .joint_storage_bytes
            .checked_add(storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let tracking = reference::tracking_work(unique.map(Domain::layout), samples)?;
        let per_tracking = tracking_work(tracking);
        let per_regional = regional_work(samples.real_len(), root_budget)?;
        Ok(Self {
            reference,
            sources,
            root_budget,
            bounds: ProbeRegionalBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts: attempts,
                tracking_work: scale_tracking(per_tracking, attempts)?,
                regional_work: scale_regional(per_regional, attempts)?,
            },
            per_tracking,
            per_regional,
        })
    }
    /// Complete simultaneous storage and finite-work bounds.
    pub fn bounds(self) -> ProbeRegionalBounds {
        self.bounds
    }
    /// Bound upstream analytical producer policy.
    pub fn reference_plan(self) -> ProbeReferencePlan<'a> {
        self.reference
    }
    /// Fixed full classifier root allowance.
    pub fn root_budget(self) -> usize {
        self.root_budget
    }
}

fn tracking_work(
    work: crate::v2_experiment::reference::ReferenceTrackingWork,
) -> ProbeReferenceWork {
    ProbeReferenceWork {
        attempts: 1,
        reference_evaluations: work.reference_evaluations,
        root_iterations: work.root_iterations,
        scalar_transforms: work.scalar_transforms,
        weighted_visits: work.weighted_visits,
        binding_checks: 192,
    }
}
fn regional_work(points: usize, roots: usize) -> Result<RegionalTrackingWork, SolverError> {
    let classifications = points.checked_mul(24).ok_or(SolverError::SizeOverflow)?;
    Ok(RegionalTrackingWork {
        attempts: 1,
        classifications,
        root_iterations: classifications
            .checked_mul(roots)
            .ok_or(SolverError::SizeOverflow)?,
        magnitude_visits: classifications,
    })
}
fn scale_tracking(w: ProbeReferenceWork, n: usize) -> Result<ProbeReferenceWork, SolverError> {
    Ok(ProbeReferenceWork {
        attempts: n,
        reference_evaluations: mul(w.reference_evaluations, n)?,
        root_iterations: mul(w.root_iterations, n)?,
        scalar_transforms: mul(w.scalar_transforms, n)?,
        weighted_visits: mul(w.weighted_visits, n)?,
        binding_checks: mul(w.binding_checks, n)?,
    })
}
fn scale_regional(w: RegionalTrackingWork, n: usize) -> Result<RegionalTrackingWork, SolverError> {
    Ok(RegionalTrackingWork {
        attempts: n,
        classifications: mul(w.classifications, n)?,
        root_iterations: mul(w.root_iterations, n)?,
        magnitude_visits: mul(w.magnitude_visits, n)?,
    })
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
