//! Aggregate memory and finite work for independent actual-state pressure production.
use super::{PressureFamilyWorkspace, PressureRefinementSample};
use crate::{
    smooth::CyclicSine,
    smooth_experiment::{FamilyError, FamilyPlan},
};
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace, local::TensorErrors,
        physical::PhysicalComparisonWorkspace,
    },
    domain::{Domain, Layout},
    integrators::forcing::PrescribedForce,
    SolverError,
};

/// Full worst-case charges, including malformed calls; transform and visit units are distinct.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PressureFamilyWork {
    /// Total attempted complete reports.
    pub attempts: usize,
    /// Independent force work, evaluated once per attempted report.
    pub provider_work_units: usize,
    /// Ten conservative assemblies plus complete pressure/gradient physical comparisons.
    pub scalar_transforms: usize,
    /// Conservative weighted coefficient/sample visits excluding FFT internals; not FLOPs.
    pub weighted_visits: usize,
}
/// All independent diagnostic storage and finite execution allowances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressureFamilyBounds {
    /// Owned diagnostic elements/headers and one returned complete record.
    pub storage_bytes: usize,
    /// Six family owners plus this consumer; other consumers need separate admission.
    pub joint_storage_bytes: usize,
    /// Maximum report attempts, at least the number of tested clocks.
    pub maximum_attempts: usize,
    /// Maximum work over the entire attempt allowance.
    pub work: PressureFamilyWork,
}
/// Immutable plan; no pressure samples or state buffers are allocated during admission.
#[derive(Debug, Clone, Copy)]
pub struct PressureFamilyPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) source: Domain,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 2],
    pub(super) bounds: PressureFamilyBounds,
    pub(super) per_attempt: PressureFamilyWork,
}
impl<'a> PressureFamilyPlan<'a> {
    /// Preflight common full doubled-band pressure, physical sampling, and all six family owners.
    /// Floors are pressure then pressure gradient; caller artifacts/allocator overhead are separate.
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
        let source = family.branches[2].plan.resources().domain();
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let storage_bytes = storage(source, diagnostic, samples)?;
        let joint_storage_bytes = add(storage_bytes, family.bounds().storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = work(source, diagnostic, samples)?;
        let work = PressureFamilyWork {
            attempts: maximum_attempts,
            provider_work_units: mul(per_attempt.provider_work_units, maximum_attempts)?,
            scalar_transforms: mul(per_attempt.scalar_transforms, maximum_attempts)?,
            weighted_visits: mul(per_attempt.weighted_visits, maximum_attempts)?,
        };
        Ok(Self {
            family,
            source,
            samples,
            floors,
            per_attempt,
            bounds: PressureFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work,
            },
        })
    }
    /// Complete joint reservation and maximum work before allocating either owner or diagnostics.
    pub fn bounds(self) -> PressureFamilyBounds {
        self.bounds
    }
    /// Actual pressure sample grid; it must retain the complete doubled velocity band.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed positive floors for pressure and its gradient.
    pub fn relative_floors(self) -> [f64; 2] {
        self.floors
    }
}
fn storage(source: Domain, diagnostic: Domain, samples: Layout) -> Result<usize, SolverError> {
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
        bytes,
        add(
            std::mem::size_of::<PressureFamilyWorkspace<'_>>(),
            std::mem::size_of::<PressureRefinementSample>(),
        )?,
    )
}
fn work(
    source: Domain,
    diagnostic: Domain,
    samples: Layout,
) -> Result<PressureFamilyWork, SolverError> {
    let provider = CyclicSine::new(diagnostic)?
        .limits()
        .ok_or(SolverError::UnknownProviderCost)?;
    // Ten assemblies retain full quadratic spectra. Separate allowances include transfers,
    // product formation, spectrum validation, scalar sampling and complete magnitude reduction.
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
            mul(diagnostic.layout().half_len(), 6 * 40)?,
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
