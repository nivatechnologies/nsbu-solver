use crate::v2_experiment::{
    probes::ProbePlan,
    reference::{self, ReferenceTrackingWork},
    FamilyError,
};
use nsbu_solver::{
    diagnostics::local::TensorErrors,
    domain::{Domain, Layout},
    SolverError,
};

/// Charged analytical, transform, reduction and publication-binding work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeReferenceWork {
    /// Whole report attempts, including a failed admitted attempt.
    pub attempts: usize,
    /// Pointwise binary64 analytical reference evaluations.
    pub reference_evaluations: usize,
    /// Conservative implicit-root iterations.
    pub root_iterations: usize,
    /// Actual reconstructed-field inverse transforms.
    pub scalar_transforms: usize,
    /// Conservative coefficient, sample and reduction visits.
    pub weighted_visits: usize,
    /// Conservative producer, publication, field, clock, domain and origin checks.
    pub binding_checks: usize,
}

/// Consumer storage and complete simultaneous probe-family admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeReferenceBounds {
    /// Consumer scratch, allocator allowance and one retained report.
    pub storage_bytes: usize,
    /// Probe owners and scratch plus this consumer.
    pub joint_storage_bytes: usize,
    /// Maximum whole report attempts.
    pub maximum_attempts: usize,
    /// Complete maximum-attempt work allowance.
    pub work: ProbeReferenceWork,
}

/// Immutable analytical tracking policy bound to one complete probe manifest.
#[derive(Debug, Clone, Copy)]
pub struct ProbeReferencePlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) sources: [Domain; 6],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) bounds: ProbeReferenceBounds,
    pub(super) per_attempt: ProbeReferenceWork,
}

impl<'a> ProbeReferencePlan<'a> {
    /// Validate floors, attempts, complete work and joint storage before allocation.
    pub fn new(
        probes: ProbePlan<'a>,
        samples: Layout,
        floors: [f64; 4],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < probes.tested_times().as_slice().len() {
            return Err(FamilyError::InvalidFamily);
        }
        for floor in floors {
            TensorErrors::<1>::new(samples.real_len(), floor)?;
        }
        let sources = probes.branches.map(|branch| branch.resources().domain());
        let unique = [sources[0], sources[1], sources[2]];
        let storage_bytes = reference::kernel_reservation(unique, samples)?
            .checked_add(std::mem::size_of::<super::ProbeReferenceWorkspace<'_>>())
            .and_then(|n| n.checked_add(std::mem::size_of::<super::ProbeReferenceSample>()))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = probes
            .bounds()
            .joint_storage_bytes
            .checked_add(storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let tracking = reference::tracking_work(unique.map(Domain::layout), samples)?;
        let per_attempt = from_tracking(tracking);
        Ok(Self {
            probes,
            sources,
            samples,
            floors,
            bounds: ProbeReferenceBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work: scale(per_attempt, maximum_attempts)?,
            },
            per_attempt,
        })
    }

    /// Complete consumer and simultaneous probe-family bounds.
    pub fn bounds(self) -> ProbeReferenceBounds {
        self.bounds
    }
    /// Exact probe-family and manifest admission bound to reports.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
    /// Unshifted physical sample lattice.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Velocity, gradient, Hessian and vorticity relative floors.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
}

fn from_tracking(work: ReferenceTrackingWork) -> ProbeReferenceWork {
    ProbeReferenceWork {
        attempts: work.attempts,
        reference_evaluations: work.reference_evaluations,
        root_iterations: work.root_iterations,
        scalar_transforms: work.scalar_transforms,
        weighted_visits: work.weighted_visits,
        binding_checks: 128,
    }
}

fn scale(work: ProbeReferenceWork, count: usize) -> Result<ProbeReferenceWork, SolverError> {
    Ok(ProbeReferenceWork {
        attempts: count,
        reference_evaluations: mul(work.reference_evaluations, count)?,
        root_iterations: mul(work.root_iterations, count)?,
        scalar_transforms: mul(work.scalar_transforms, count)?,
        weighted_visits: mul(work.weighted_visits, count)?,
        binding_checks: mul(work.binding_checks, count)?,
    })
}

fn mul(value: usize, count: usize) -> Result<usize, SolverError> {
    value.checked_mul(count).ok_or(SolverError::SizeOverflow)
}
