//! Joint admission for complete physical comparisons at every probe-manifest clock.
use super::{ProbePhysicalSample, ProbePhysicalWorkspace};
use crate::v2_experiment::{
    physical::plan::work::{mul, work_for_pairs},
    probes::ProbePlan,
    FamilyError,
};
use nsbu_solver::{
    diagnostics::{local::TensorErrors, physical::PhysicalComparisonWorkspace},
    domain::{Domain, Layout},
    SolverError,
};

/// Charged physical and publication-binding work, including failed attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbePhysicalWork {
    /// Whole report attempts.
    pub attempts: usize,
    /// Complete inverse transforms for all four quantities and five pairs.
    pub scalar_transforms: usize,
    /// Conservative source, sampling, reduction, and extrema visits.
    pub weighted_visits: usize,
    /// Conservative whole producer/sample/field identity, clock, origin, and domain checks.
    pub binding_checks: usize,
}

/// Consumer storage and finite total work admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbePhysicalBounds {
    /// Consumer workspace, allocator allowance, and one retained report.
    pub storage_bytes: usize,
    /// Probe owners and scratch plus this consumer.
    pub joint_storage_bytes: usize,
    /// Maximum whole report attempts.
    pub maximum_attempts: usize,
    /// Complete maximum-attempt work allowance.
    pub work: ProbePhysicalWork,
}

/// Immutable physical policy bound to a complete exact probe manifest.
#[derive(Debug, Clone, Copy)]
pub struct ProbePhysicalPlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) sources: [Domain; 6],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) bounds: ProbePhysicalBounds,
    pub(super) per_attempt: ProbePhysicalWork,
}
impl<'a> ProbePhysicalPlan<'a> {
    /// Validate every floor and complete joint storage/work before allocation.
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
        let finest = sources[2];
        let storage_bytes = PhysicalComparisonWorkspace::reservation(finest, finest, samples)?
            // Two derivative streams and four vectors allocate fewer than 32 blocks.
            .checked_add(32 * 64)
            .and_then(|n| n.checked_add(std::mem::size_of::<ProbePhysicalWorkspace<'_>>()))
            .and_then(|n| n.checked_add(std::mem::size_of::<ProbePhysicalSample>()))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = probes
            .bounds()
            .joint_storage_bytes
            .checked_add(storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let physical = work_for_pairs(finest.layout(), samples, 5)?;
        let per_attempt = ProbePhysicalWork {
            attempts: 1,
            scalar_transforms: physical.scalar_transforms,
            weighted_visits: physical.weighted_visits,
            // Whole logical comparisons, conservatively counting nested origins
            // without claiming bytewise or scalar-word operation counts.
            binding_checks: 128,
        };
        let work = scale(per_attempt, maximum_attempts)?;
        Ok(Self {
            probes,
            sources,
            samples,
            floors,
            bounds: ProbePhysicalBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work,
            },
            per_attempt,
        })
    }
    /// Complete consumer and simultaneous probe-family reservation.
    pub fn bounds(self) -> ProbePhysicalBounds {
        self.bounds
    }
    /// Bound exact probe-family and complete manifest admission.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
    /// Common physical sample lattice.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Floors in [`super::PROBE_PHYSICAL_QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
}

fn scale(work: ProbePhysicalWork, attempts: usize) -> Result<ProbePhysicalWork, SolverError> {
    Ok(ProbePhysicalWork {
        attempts,
        scalar_transforms: mul(work.scalar_transforms, attempts)?,
        weighted_visits: mul(work.weighted_visits, attempts)?,
        binding_checks: mul(work.binding_checks, attempts)?,
    })
}
