//! Checked admission ledger for the offline analytical reference observer.
//!
//! Every entry is either an exact library reservation, a live-array term derived
//! exactly from the observer's own field allocations, or an explicitly labelled
//! conservative allowance; the total is a conservative admission upper bound,
//! not a measured allocator peak. `ObserverLedger` is re-derived field by field
//! from the actual construction inputs before any allocation, so a forged,
//! shrunken or cross-profile ledger cannot bypass the cap.
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_benchmarks::fields::reference::ReferenceEvaluation;
use nsbu_solver::{
    diagnostics::{conservative::ConservativeWorkspace, derivatives::DerivativeWorkspace},
    domain::{Domain, Layout},
    spectral::{FftBackend, FftCatalog},
    SolverError,
};

pub const MANIFEST_HEADER_ALLOWANCE: usize = 1024 * 1024;
pub const SNAPSHOT_PREFIX_ALLOWANCE: usize = 1024;
pub const ALLOCATOR_PAGE_ALLOWANCE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverLedger {
    pub backend: FftBackend,
    pub catalog_bytes: usize,
    pub force_storage_bytes: usize,
    pub conservative_bytes: usize,
    pub velocity_sampler_bytes: usize,
    pub pressure_sampler_bytes: usize,
    pub observer_array_bytes: usize,
    pub comparison_array_bytes: usize,
    pub reference_cache_bytes: usize,
    pub snapshot_state_bytes: usize,
    pub header_allowance_bytes: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkLedger {
    pub velocity_points: usize,
    pub pressure_points: usize,
    pub reference_evaluations: usize,
    pub reference_root_iterations: usize,
    pub classification_root_budget: usize,
    pub observer_scalar_transforms: usize,
    pub conservative_scalar_transforms: usize,
    pub provider_scalar_transforms: usize,
    pub provider_work_units: usize,
    pub weighted_visits: usize,
}

/// Inputs that fully determine both ledgers; kept as data so `preflight` is pure
/// arithmetic and the run path can re-derive identically from construction inputs.
#[derive(Debug, Clone, Copy)]
pub struct AdmissionInputs {
    pub source: Domain,
    pub velocity_samples: Layout,
    pub pressure_samples: Layout,
    pub force_samples: Layout,
    pub workers: usize,
    pub backend: FftBackend,
    pub root_budget: usize,
    pub identity_len: usize,
}

const VELOCITY_TRANSFORMS: usize = 45;
const PRESSURE_TRANSFORMS: usize = 5;
const VELOCITY_QUANTITIES: usize = 4;
const PRESSURE_QUANTITIES: usize = 2;

fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}

fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}

/// Complete memory ledger before any observer allocation. Exact library
/// reservations are taken from the same public reservation functions the run
/// path uses; the live-array terms are derived from the exact field allocations
/// in `ReferenceObserver::new`; only snapshot and header terms are allowances.
pub fn preflight(inputs: AdmissionInputs) -> Result<(ObserverLedger, WorkLedger), SolverError> {
    let inputs = inputs.normalize()?;
    let exact = exact_reservations(inputs)?;
    let observer_array_bytes = observer_array_bytes(inputs)?;
    let comparison_array_bytes = comparison_array_bytes(inputs)?;
    let reference_cache_bytes = reference_cache_bytes(inputs)?;
    let snapshot_state_bytes = snapshot_state_bytes(inputs)?;
    let header_allowance_bytes = add(MANIFEST_HEADER_ALLOWANCE, ALLOCATOR_PAGE_ALLOWANCE)?;
    let total_bytes = [
        exact.catalog_bytes,
        exact.force_storage_bytes,
        exact.conservative_bytes,
        exact.velocity_sampler_bytes,
        exact.pressure_sampler_bytes,
        observer_array_bytes,
        comparison_array_bytes,
        reference_cache_bytes,
        snapshot_state_bytes,
        header_allowance_bytes,
    ]
    .into_iter()
    .try_fold(0_usize, add)?;
    let ledger = ObserverLedger {
        backend: inputs.backend,
        catalog_bytes: exact.catalog_bytes,
        force_storage_bytes: exact.force_storage_bytes,
        conservative_bytes: exact.conservative_bytes,
        velocity_sampler_bytes: exact.velocity_sampler_bytes,
        pressure_sampler_bytes: exact.pressure_sampler_bytes,
        observer_array_bytes,
        comparison_array_bytes,
        reference_cache_bytes,
        snapshot_state_bytes,
        header_allowance_bytes,
        total_bytes,
    };
    let work = work_ledger(inputs, exact.provider_work_units, exact.provider_scalar_transforms)?;
    Ok((ledger, work))
}

/// Exact library reservations and the provider work they admit.
struct ExactReservations {
    catalog_bytes: usize,
    force_storage_bytes: usize,
    conservative_bytes: usize,
    velocity_sampler_bytes: usize,
    pressure_sampler_bytes: usize,
    provider_work_units: usize,
    provider_scalar_transforms: usize,
}

fn exact_reservations(inputs: AdmissionInputs) -> Result<ExactReservations, SolverError> {
    let diagnostic = ConservativeWorkspace::diagnostic_domain(inputs.source)?;
    let force_limits = ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        inputs.force_samples,
        inputs.workers,
        inputs.backend,
    )?;
    Ok(ExactReservations {
        catalog_bytes: FftCatalog::reservation(inputs.backend)?,
        force_storage_bytes: force_limits.storage_bytes,
        conservative_bytes: ConservativeWorkspace::reservation_with_fft_backend(
            inputs.source,
            inputs.backend,
        )?,
        velocity_sampler_bytes: DerivativeWorkspace::reservation_with_shared_backend(
            inputs.source,
            inputs.velocity_samples,
            inputs.backend,
        )?,
        pressure_sampler_bytes: DerivativeWorkspace::reservation_with_shared_backend(
            diagnostic,
            inputs.pressure_samples,
            inputs.backend,
        )?,
        provider_work_units: force_limits.work_units,
        provider_scalar_transforms: force_limits.scalar_transforms,
    })
}

fn observer_array_bytes(inputs: AdmissionInputs) -> Result<usize, SolverError> {
    let doubled = ConservativeWorkspace::diagnostic_domain(inputs.source)?.layout().half_len();
    add(
        mul(doubled, 7 * std::mem::size_of::<nsbu_solver::Complex64>())?,
        mul(
            std::mem::size_of::<ParallelReducedV2Force>()
                + 4 * std::mem::size_of::<DerivativeWorkspace>()
                + 12,
            ALLOCATOR_PAGE_ALLOWANCE,
        )?,
    )
}

/// Live f64 arrays, derived exactly from the `ReferenceObserver` fields:
/// `actual`, `scratch` (peak-magnitude accumulator) and one buffer sized to
/// the larger lattice each, plus the per-grid error and reference magnitude
/// arrays. The previous `3v + 2p` allowance undercounted the N512 live
/// arrays by exactly 16 106 127 360 bytes; this term now matches them.
fn comparison_array_bytes(inputs: AdmissionInputs) -> Result<usize, SolverError> {
    let velocity_points = inputs.velocity_samples.real_len();
    let pressure_points = inputs.pressure_samples.real_len();
    let largest = velocity_points.max(pressure_points);
    mul(
        add(
            add(mul(2, largest)?, mul(2, velocity_points)?)?,
            mul(2, pressure_points)?,
        )?,
        std::mem::size_of::<f64>(),
    )
}

fn reference_cache_bytes(inputs: AdmissionInputs) -> Result<usize, SolverError> {
    add(
        mul(
            inputs.velocity_samples.real_len(),
            std::mem::size_of::<ReferenceEvaluation>(),
        )?,
        mul(inputs.pressure_samples.real_len(), 4 * std::mem::size_of::<f64>())?,
    )
}

fn snapshot_state_bytes(inputs: AdmissionInputs) -> Result<usize, SolverError> {
    add(
        mul(
            inputs.source.layout().half_len(),
            3 * std::mem::size_of::<nsbu_solver::Complex64>(),
        )?,
        add(inputs.identity_len, SNAPSHOT_PREFIX_ALLOWANCE)?,
    )
}

/// Exact per-attempt work charges: one cached reference evaluation per lattice
/// point, the reviewed 128-iteration root allowance per evaluation, the declared
/// classification budget for the global-plus-regional collectors, and the
/// sequential transform/visit inventory for both comparison grids.
fn work_ledger(
    inputs: AdmissionInputs,
    provider_work_units: usize,
    provider_scalar_transforms: usize,
) -> Result<WorkLedger, SolverError> {
    let diagnostic = ConservativeWorkspace::diagnostic_domain(inputs.source)?;
    let velocity_points = inputs.velocity_samples.real_len();
    let pressure_points = inputs.pressure_samples.real_len();
    let evaluations = add(velocity_points, pressure_points)?;
    let classifications = add(
        mul(velocity_points, VELOCITY_QUANTITIES)?,
        mul(pressure_points, PRESSURE_QUANTITIES)?,
    )?;
    let visits = add(
        mul(
            VELOCITY_TRANSFORMS,
            DerivativeWorkspace::coefficient_visits(inputs.source.layout())?,
        )?,
        mul(
            PRESSURE_TRANSFORMS,
            DerivativeWorkspace::coefficient_visits(diagnostic.layout())?,
        )?,
    )?;
    let point_visits = add(
        add(mul(velocity_points, 42 + 1)?, mul(pressure_points, 4 + 1)?)?,
        evaluations,
    )?;
    Ok(WorkLedger {
        velocity_points,
        pressure_points,
        reference_evaluations: evaluations,
        reference_root_iterations: mul(evaluations, 128)?,
        classification_root_budget: mul(classifications, inputs.root_budget)?,
        observer_scalar_transforms: VELOCITY_TRANSFORMS + PRESSURE_TRANSFORMS,
        conservative_scalar_transforms: 9,
        provider_scalar_transforms,
        provider_work_units,
        weighted_visits: add(add(visits, point_visits)?, ALLOCATOR_PAGE_ALLOWANCE)?,
    })
}

impl AdmissionInputs {
    /// Reject grids that cannot retain the bands they must sample and reject
    /// degenerate worker counts or root budgets before any reservation is trusted.
    /// Finer-grained provider/sample-domain admission stays with the reviewed
    /// library reservations invoked by `preflight`.
    pub fn normalize(self) -> Result<Self, SolverError> {
        let diagnostic = ConservativeWorkspace::diagnostic_domain(self.source)?;
        let covers = |domain: Domain, samples: Layout| {
            domain
                .layout()
                .dimensions()
                .into_iter()
                .zip(samples.dimensions())
                .all(|(source, sample)| source <= sample)
        };
        if !covers(self.source, self.velocity_samples)
            || !covers(self.source, self.force_samples)
            || !covers(diagnostic, self.pressure_samples)
        {
            return Err(SolverError::InvalidDomain);
        }
        if self.workers == 0 || !(1..=128).contains(&self.root_budget) {
            return Err(SolverError::InvalidPayload);
        }
        Ok(self)
    }
}
