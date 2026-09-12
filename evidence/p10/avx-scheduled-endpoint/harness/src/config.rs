//! Immutable endpoint profile and its complete memory/work/disk admission.
use crate::{artifact, cache::CachedReducedForce, observer::ReducedObserver, schedule};
use nsbu_benchmarks::CASE_SHA256;
use nsbu_solver::{
    diagnostics::conservative::ConservativeWorkspace,
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan},
    integrators::{
        attempt::AttemptWorkspace, indicator::Tolerances, method::Method, rhs::SpectralRhs,
    },
    spectral::{FftBackend, FftCatalog},
    SolverError,
};

pub const N: usize = 192;
pub const M: usize = 384;
pub const WORKERS: usize = 32;
pub const CAP: usize = 103_079_215_104;
pub const ADVECTIVE_LIMIT: f64 = 0.45;
const HISTORY_BYTES: usize = schedule::MAXIMUM_ATTEMPTS * 4096;
const OVERHEAD: usize = artifact::BUFFER_BYTES + HISTORY_BYTES + 64 * 1024;

pub fn preflight() -> Result<ResourcePlan, SolverError> {
    schedule::validate()?;
    let domain = domain()?;
    let samples = Layout::new([M; 3])?;
    let observer_samples = Layout::new([2 * M; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let force_limits = CachedReducedForce::preflight(domain, samples, WORKERS, backend)?;
    let rhs_bytes = SpectralRhs::<CachedReducedForce>::reservation_with_fft_backend(
        domain,
        force_limits,
        backend,
    )?;
    let attempt_bytes = AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews)?;
    let observer_bytes = ReducedObserver::preflight(domain, observer_samples, WORKERS, backend)?;
    let diagnostics = attempt_bytes
        .checked_add(observer_bytes)
        .ok_or(SolverError::SizeOverflow)?;
    let resources = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: catalog_bytes,
            force: rhs_bytes,
            diagnostics,
            overhead: OVERHEAD,
        },
        CAP,
        Epoch(0),
    )?;
    report(
        resources,
        force_limits.work_units,
        catalog_bytes,
        rhs_bytes,
        attempt_bytes,
        observer_bytes,
    )?;
    Ok(resources)
}

fn report(
    resources: ResourcePlan,
    force_work: usize,
    catalog_bytes: usize,
    rhs_bytes: usize,
    attempt_bytes: usize,
    observer_bytes: usize,
) -> Result<(), SolverError> {
    let snapshot_bytes = domain()?
        .layout()
        .half_len()
        .checked_mul(3 * 16)
        .ok_or(SolverError::SizeOverflow)?;
    let disk_bytes = artifact::disk_preflight(snapshot_bytes, schedule::FINE.len())?;
    let integration_work = force_work
        .checked_mul(12 * schedule::MAXIMUM_ATTEMPTS)
        .ok_or(SolverError::SizeOverflow)?;
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain()?)?;
    let observer_force = nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        Layout::new([2 * M; 3])?,
        WORKERS,
        FftBackend::RustFft6_4_1AvxFma,
    )?;
    let observer_work = observer_force
        .work_units
        .checked_mul(8)
        .ok_or(SolverError::SizeOverflow)?;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} schema=p10-avx-scheduled-endpoint-v2 backend=rustfft-6.4.1-avx-avx2-fma provider=parallel-reduced-attempt-cache retained={N} sampled={M} workers={WORKERS} method=cox-matthews step={} maximum_attempts={} endpoint={} advective_limit={ADVECTIVE_LIMIT} observer_nodes={:?} observer_sampled={} nested_middle={:?} nested_coarse={:?} catalog_bytes={catalog_bytes} rhs_bytes={rhs_bytes} attempt_bytes={attempt_bytes} observer_bytes={observer_bytes} overhead={OVERHEAD} total={} cap={CAP} disk_preflight_bytes={disk_bytes} disk_cap_bytes={} integration_work_bound={integration_work} observer_work_bound={observer_work} archive_profile=unsupported qualification=experimental",
        env!("RUN_SOURCE"),
        schedule::STEP,
        schedule::MAXIMUM_ATTEMPTS,
        schedule::ENDPOINT,
        schedule::FINE,
        2 * M,
        schedule::MIDDLE,
        schedule::COARSE,
        resources.total(),
        artifact::DISK_CAP_BYTES,
    );
    Ok(())
}

pub fn tolerances() -> Tolerances {
    Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    }
}

pub fn domain() -> Result<Domain, SolverError> {
    Domain::new([N; 3], [1.0; 3], 1.0)
}

pub fn identity() -> String {
    format!(
        "source={};case={CASE_SHA256};backend=rustfft-6.4.1-avx-avx2-fma;provider=parallel-reduced-attempt-cache;n={N};m={M};workers={WORKERS};method=cox-matthews;step={};endpoint={};advective_limit={ADVECTIVE_LIMIT};cap={CAP};schema=p10-avx-scheduled-endpoint-v2;resume=unsupported",
        env!("RUN_SOURCE"),
        schedule::STEP,
        schedule::ENDPOINT,
    )
}
