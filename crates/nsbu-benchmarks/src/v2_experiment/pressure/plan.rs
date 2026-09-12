//! Bounded admission for independent full-band V2 pressure diagnostics.
use super::{PressureFamilyWorkspace, PressureRefinementSample};
use crate::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyError, FamilyPlan},
};
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace, derivatives::DerivativeWorkspace, local::TensorErrors,
        physical::PhysicalComparisonWorkspace,
    },
    domain::{Domain, Layout},
    SolverError,
};

/// Work charged by a pressure report attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PressureFamilyWork {
    /// Number of charged attempts.
    pub attempts: usize,
    /// Independent V2 provider work units.
    pub provider_work_units: usize,
    /// Scalar forward/inverse transforms across provider, products and comparisons.
    pub scalar_transforms: usize,
    /// Conservative coefficient and sampled-point visits.
    pub weighted_visits: usize,
}
/// Complete consumer and joint-family reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressureFamilyBounds {
    /// Consumer-owned storage.
    pub storage_bytes: usize,
    /// Family plus consumer storage.
    pub joint_storage_bytes: usize,
    /// Maximum charged attempts.
    pub maximum_attempts: usize,
    /// Worst-case work over the allowance.
    pub work: PressureFamilyWork,
}
/// Immutable pressure policy borrowed from an admitted V2 family.
#[derive(Debug, Clone, Copy)]
pub struct PressureFamilyPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) source: Domain,
    pub(super) diagnostic: Domain,
    pub(super) force: ForceSettings,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 2],
    pub(super) provider_limits: nsbu_solver::integrators::forcing::ForceLimits,
    pub(super) bounds: PressureFamilyBounds,
    pub(super) per_attempt: PressureFamilyWork,
}
impl<'a> PressureFamilyPlan<'a> {
    /// Admit full doubled-band pressure, independent force and finite work.
    pub fn new(
        family: FamilyPlan<'a>,
        samples: Layout,
        floors: [f64; 2],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < family.times.as_slice().len() {
            return Err(SolverError::ResourceLimit.into());
        }
        for floor in floors {
            TensorErrors::<1>::new(samples.real_len(), floor)?;
        }
        let source = family.branches[2].resources().domain();
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        if samples
            .dimensions()
            .iter()
            .zip(diagnostic.layout().dimensions())
            .any(|(a, b)| *a < b)
        {
            return Err(SolverError::InvalidDomain.into());
        }
        // Pressure has always sampled force on this finest doubled domain.  Only
        // the already configured trajectory worker count is inherited here.
        let force = ForceSettings {
            samples: diagnostic.layout(),
            workers: family.settings().force.workers,
        };
        let provider_limits = force.limits(diagnostic)?;
        let storage_bytes = storage(source, diagnostic, samples, provider_limits)?;
        let joint_storage_bytes = add(storage_bytes, family.bounds().storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = work(source, diagnostic, samples, provider_limits)?;
        let work = PressureFamilyWork {
            attempts: maximum_attempts,
            provider_work_units: mul(per_attempt.provider_work_units, maximum_attempts)?,
            scalar_transforms: mul(per_attempt.scalar_transforms, maximum_attempts)?,
            weighted_visits: mul(per_attempt.weighted_visits, maximum_attempts)?,
        };
        Ok(Self {
            family,
            source,
            diagnostic,
            force,
            samples,
            floors,
            provider_limits,
            bounds: PressureFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work,
            },
            per_attempt,
        })
    }
    /// Complete consumer and joint reservation.
    pub fn bounds(self) -> PressureFamilyBounds {
        self.bounds
    }
    /// Finest source velocity domain.
    pub fn source_domain(self) -> Domain {
        self.source
    }
    /// Full doubled force/pressure domain.
    pub fn force_layout(self) -> Layout {
        self.force.samples
    }
    /// Effective persistent-worker count; zero selects the serial provider.
    pub fn force_workers(self) -> usize {
        self.force.workers
    }
    /// Physical comparison sample layout.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Pressure and gradient floors.
    pub fn relative_floors(self) -> [f64; 2] {
        self.floors
    }
    pub(crate) fn family_plan(self) -> FamilyPlan<'a> {
        self.family
    }
    pub(crate) fn diagnostic_domain(self) -> Domain {
        self.diagnostic
    }
    pub(crate) fn force_settings(self) -> ForceSettings {
        self.force
    }
    pub(crate) fn provider_limits(self) -> nsbu_solver::integrators::forcing::ForceLimits {
        self.provider_limits
    }
    pub(crate) fn per_attempt_work(self) -> PressureFamilyWork {
        self.per_attempt
    }
}
fn storage(
    source: Domain,
    diagnostic: Domain,
    samples: Layout,
    provider: nsbu_solver::integrators::forcing::ForceLimits,
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
    // Sixty-four 64-byte allocator allowances cover the consumer's own buffers,
    // conservative and physical workspaces; element/header reservations are separate.
    add(
        add(bytes, 64 * 64)?,
        add(
            provider.storage_bytes,
            add(
                std::mem::size_of::<PressureFamilyWorkspace<'_>>(),
                std::mem::size_of::<PressureRefinementSample>(),
            )?,
        )?,
    )
}
fn work(
    source: Domain,
    diagnostic: Domain,
    samples: Layout,
    provider: nsbu_solver::integrators::forcing::ForceLimits,
) -> Result<PressureFamilyWork, SolverError> {
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
        add(mul(samples.real_len(), 4 * 40 + 10 * 5 * 2)?, 1024)?,
    )?;
    Ok(PressureFamilyWork {
        attempts: 1,
        provider_work_units: provider.work_units,
        scalar_transforms: add(provider.scalar_transforms, 10 * 9 + 5 * (2 + 6))?,
        weighted_visits: visits,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
