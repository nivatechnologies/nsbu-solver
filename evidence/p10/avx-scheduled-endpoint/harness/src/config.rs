//! Immutable endpoint profile and its complete memory/work/disk admission.
use crate::{artifact, cache::CachedReducedForce, observer::ReducedObserver, schedule};
use nsbu_benchmarks::CASE_SHA256;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan},
    integrators::{
        attempt::AttemptWorkspace, forcing::ForceLimits, indicator::Tolerances, method::Method,
        rhs::SpectralRhs,
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

#[derive(Clone, Copy)]
struct Geometry {
    domain: Domain,
    samples: Layout,
    observer_samples: Layout,
    backend: FftBackend,
}

#[derive(Clone, Copy)]
struct Reservations {
    catalog: usize,
    force: ForceLimits,
    rhs: usize,
    attempt: usize,
    observer: usize,
}

struct Admission {
    resources: ResourcePlan,
    reservations: Reservations,
    disk: usize,
    integration_work: usize,
    observer_work: usize,
}

pub fn preflight() -> Result<ResourcePlan, SolverError> {
    schedule::validate()?;
    let admission = admit()?;
    report(&admission);
    Ok(admission.resources)
}

fn admit() -> Result<Admission, SolverError> {
    let geometry = geometry()?;
    admit_geometry(geometry)
}

fn geometry() -> Result<Geometry, SolverError> {
    let domain = domain()?;
    geometry_for(domain)
}

fn geometry_for(domain: Domain) -> Result<Geometry, SolverError> {
    let samples = Layout::new([M; 3])?;
    let observer_samples = Layout::new([2 * M; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    Ok(Geometry {
        domain,
        samples,
        observer_samples,
        backend,
    })
}

fn admit_geometry(geometry: Geometry) -> Result<Admission, SolverError> {
    let reservations = reservations(geometry)?;
    let resources = resources(geometry.domain, reservations)?;
    work_admission(geometry, reservations, resources)
}

fn reservations(geometry: Geometry) -> Result<Reservations, SolverError> {
    let (catalog, force, rhs) = execution_reservations(geometry)?;
    let (attempt, observer) = diagnostic_reservations(geometry)?;
    Ok(Reservations {
        catalog,
        force,
        rhs,
        attempt,
        observer,
    })
}

fn execution_reservations(geometry: Geometry) -> Result<(usize, ForceLimits, usize), SolverError> {
    let catalog = FftCatalog::reservation(geometry.backend)?;
    let force = CachedReducedForce::preflight(
        geometry.domain,
        geometry.samples,
        WORKERS,
        geometry.backend,
    )?;
    let rhs = SpectralRhs::<CachedReducedForce>::reservation_with_fft_backend(
        geometry.domain,
        force,
        geometry.backend,
    )?;
    Ok((catalog, force, rhs))
}

fn diagnostic_reservations(geometry: Geometry) -> Result<(usize, usize), SolverError> {
    let attempt = AttemptWorkspace::reservation_with_method(geometry.domain, Method::CoxMatthews)?;
    let observer = ReducedObserver::preflight(
        geometry.domain,
        geometry.observer_samples,
        WORKERS,
        geometry.backend,
    )?;
    Ok((attempt, observer))
}

fn resources(domain: Domain, sizes: Reservations) -> Result<ResourcePlan, SolverError> {
    let diagnostics = sizes
        .attempt
        .checked_add(sizes.observer)
        .ok_or(SolverError::SizeOverflow)?;
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: sizes.catalog,
            force: sizes.rhs,
            diagnostics,
            overhead: OVERHEAD,
        },
        CAP,
        Epoch(0),
    )
}

fn work_admission(
    geometry: Geometry,
    reservations: Reservations,
    resources: ResourcePlan,
) -> Result<Admission, SolverError> {
    let disk = disk_bound(geometry.domain)?;
    let integration_work = reservations
        .force
        .work_units
        .checked_mul(12 * schedule::MAXIMUM_ATTEMPTS)
        .ok_or(SolverError::SizeOverflow)?;
    let observer_work = observer_work(geometry)?;
    Ok(Admission {
        resources,
        reservations,
        disk,
        integration_work,
        observer_work,
    })
}

fn disk_bound(domain: Domain) -> Result<usize, SolverError> {
    let snapshot = domain
        .layout()
        .half_len()
        .checked_mul(3 * 16)
        .ok_or(SolverError::SizeOverflow)?;
    artifact::disk_preflight(snapshot, schedule::FINE.len())
}

fn observer_work(geometry: Geometry) -> Result<usize, SolverError> {
    let diagnostic =
        nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
            geometry.domain,
        )?;
    let force = nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        geometry.observer_samples,
        WORKERS,
        geometry.backend,
    )?;
    force
        .work_units
        .checked_mul(8)
        .ok_or(SolverError::SizeOverflow)
}

fn report(admission: &Admission) {
    let sizes = admission.reservations;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} schema=p10-avx-scheduled-endpoint-v2 backend=rustfft-6.4.1-avx-avx2-fma provider=parallel-reduced-attempt-cache retained={N} sampled={M} workers={WORKERS} method=cox-matthews step={} maximum_attempts={} endpoint={} advective_limit={ADVECTIVE_LIMIT} observer_nodes={:?} observer_sampled={} nested_middle={:?} nested_coarse={:?} catalog_bytes={} rhs_bytes={} attempt_bytes={} observer_bytes={} overhead={OVERHEAD} total={} cap={CAP} disk_preflight_bytes={} disk_cap_bytes={} integration_work_bound={} observer_work_bound={} archive_profile=unsupported qualification=experimental",
        env!("RUN_SOURCE"),
        schedule::STEP,
        schedule::MAXIMUM_ATTEMPTS,
        schedule::ENDPOINT,
        schedule::FINE,
        2 * M,
        schedule::MIDDLE,
        schedule::COARSE,
        sizes.catalog,
        sizes.rhs,
        sizes.attempt,
        sizes.observer,
        admission.resources.total(),
        admission.disk,
        artifact::DISK_CAP_BYTES,
        admission.integration_work,
        admission.observer_work,
    );
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
