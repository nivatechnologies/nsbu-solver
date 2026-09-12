//! Joint admission for off-stage pressure comparisons at every probe clock.
use super::{ProbePressureSample, ProbePressureWorkspace};
use crate::{
    runtime_force::ForceSettings,
    v2_experiment::{probes::ProbePlan, FamilyError},
};
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace, derivatives::DerivativeWorkspace, local::TensorErrors,
        physical::PhysicalComparisonWorkspace,
    },
    domain::{Domain, Layout},
    integrators::forcing::ForceLimits,
    SolverError,
};

/// Charged work for one or more complete pressure attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbePressureWork {
    /// Whole report attempts charged before binding or physics.
    pub attempts: usize,
    /// Original-force provider work reserved and charged once per attempt.
    pub provider_work_units: usize,
    /// Provider, ten pressure-construction, and ten comparison transform allowance.
    pub scalar_transforms: usize,
    /// Conservative coefficient, sampled-point, reduction, and comparison visits.
    pub weighted_visits: usize,
    /// Conservative complete producer, field, clock, domain, identity, and origin checks.
    pub binding_checks: usize,
}
/// Complete consumer and joint probe-family reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbePressureBounds {
    /// Consumer-owned buffers, workspaces, provider, reports, and allocator allowance.
    pub storage_bytes: usize,
    /// Probe-family joint owner plus this consumer, counted exactly once each.
    pub joint_storage_bytes: usize,
    /// Finite whole-report attempt allowance; failed attempts consume one.
    pub maximum_attempts: usize,
    /// Checked maximum work across the complete attempt allowance.
    pub work: ProbePressureWork,
}
/// Immutable pressure policy bound to one exact probe manifest.
#[derive(Debug, Clone, Copy)]
pub struct ProbePressurePlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) sources: [Domain; 6],
    pub(super) source: Domain,
    pub(super) diagnostic: Domain,
    pub(super) force: ForceSettings,
    pub(super) provider_limits: ForceLimits,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 2],
    pub(super) bounds: ProbePressureBounds,
    pub(super) per_attempt: ProbePressureWork,
}
impl<'a> ProbePressurePlan<'a> {
    /// Admit the complete manifest, two pressure buffers, one 2N force provider, and work before allocation.
    pub fn new(
        probes: ProbePlan<'a>,
        samples: Layout,
        floors: [f64; 2],
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
        let source = sources[2];
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        if samples
            .dimensions()
            .iter()
            .zip(diagnostic.layout().dimensions())
            .any(|(a, b)| *a < b)
        {
            return Err(SolverError::InvalidDomain.into());
        }
        let force = ForceSettings {
            samples: diagnostic.layout(),
            workers: probes.family.settings().force.workers,
        };
        let provider_limits = force.limits(diagnostic)?;
        let storage_bytes = storage(source, diagnostic, samples, provider_limits)?;
        let joint_storage_bytes = add(probes.bounds().joint_storage_bytes, storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = attempt_work(source, diagnostic, samples, provider_limits)?;
        let work = scale(per_attempt, maximum_attempts)?;
        Ok(Self {
            probes,
            sources,
            source,
            diagnostic,
            force,
            provider_limits,
            samples,
            floors,
            bounds: ProbePressureBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work,
            },
            per_attempt,
        })
    }
    /// Complete consumer and simultaneous probe-owner bounds.
    pub fn bounds(self) -> ProbePressureBounds {
        self.bounds
    }
    /// Exact reconstructed probe manifest bound by this consumer.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
    /// Finest retained family domain used as the common velocity source.
    pub fn source_domain(self) -> Domain {
        self.source
    }
    /// Doubled finest-domain layout used for both force sampling and pressure coefficients.
    pub fn force_layout(self) -> Layout {
        self.force.samples
    }
    /// Worker count inherited from the family force settings.
    pub fn force_workers(self) -> usize {
        self.force.workers
    }
    /// Caller-selected lattice for physical pressure comparisons.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Positive finite floors in scalar-pressure, pressure-gradient order.
    pub fn relative_floors(self) -> [f64; 2] {
        self.floors
    }
}
fn storage(
    source: Domain,
    diagnostic: Domain,
    samples: Layout,
    provider: ForceLimits,
) -> Result<usize, SolverError> {
    let elements = add(
        mul(source.layout().half_len(), 3)?,
        mul(diagnostic.layout().half_len(), 8)?,
    )?;
    let bytes = add(
        mul(elements, 16)?,
        ConservativeWorkspace::reservation(source)?,
    )?;
    let bytes = add(
        bytes,
        PhysicalComparisonWorkspace::reservation(diagnostic, diagnostic, samples)?,
    )?;
    add(
        add(bytes, 64 * 64)?,
        add(
            provider.storage_bytes,
            add(
                std::mem::size_of::<ProbePressureWorkspace<'_>>(),
                std::mem::size_of::<ProbePressureSample>(),
            )?,
        )?,
    )
}
fn attempt_work(
    source: Domain,
    diagnostic: Domain,
    samples: Layout,
    provider: ForceLimits,
) -> Result<ProbePressureWork, SolverError> {
    let assembly = add(
        add(
            mul(source.layout().half_len(), 64)?,
            mul(diagnostic.layout().half_len(), 64)?,
        )?,
        mul(diagnostic.layout().real_len(), 6)?,
    )?;
    let visits = add(
        add(
            mul(assembly, 10)?,
            mul(
                DerivativeWorkspace::coefficient_visits(diagnostic.layout())?,
                40,
            )?,
        )?,
        add(mul(samples.real_len(), 260)?, 1024)?,
    )?;
    Ok(ProbePressureWork {
        attempts: 1,
        provider_work_units: provider.work_units,
        scalar_transforms: add(provider.scalar_transforms, 130)?,
        weighted_visits: visits,
        binding_checks: 128,
    })
}
fn scale(w: ProbePressureWork, n: usize) -> Result<ProbePressureWork, SolverError> {
    Ok(ProbePressureWork {
        attempts: n,
        provider_work_units: mul(w.provider_work_units, n)?,
        scalar_transforms: mul(w.scalar_transforms, n)?,
        weighted_visits: mul(w.weighted_visits, n)?,
        binding_checks: mul(w.binding_checks, n)?,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
