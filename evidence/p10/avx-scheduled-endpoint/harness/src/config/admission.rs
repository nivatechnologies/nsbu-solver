//! Complete memory/work/disk admission for the selected profile.
#[cfg(not(capture_offline))]
use crate::observer::ReducedObserver;
use crate::{artifact, cache::CachedReducedForce, schedule};
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
use super::*;

#[derive(Clone, Copy)]
pub(crate) struct Geometry {
    pub(crate) domain: Domain,
    pub(crate) samples: Layout,
    pub(crate) observer_samples: Layout,
    pub(crate) backend: FftBackend,
}

#[derive(Clone, Copy)]
pub(crate) struct Reservations {
    pub(crate) catalog: usize,
    pub(crate) force: ForceLimits,
    pub(crate) rhs: usize,
    pub(crate) attempt: usize,
    pub(crate) observer: usize,
}

pub(crate) struct Admission {
    pub(crate) resources: ResourcePlan,
    pub(crate) reservations: Reservations,
    pub(crate) disk: usize,
    pub(crate) integration_work: usize,
    pub(crate) observer_work: usize,
}

pub fn preflight() -> Result<ResourcePlan, SolverError> {
    require_execution_ready()?;
    schedule::validate()?;
    let admission = admit()?;
    report(&admission);
    Ok(admission.resources)
}

pub fn require_execution_ready() -> Result<(), SolverError> {
    Ok(())
}

pub fn require_run_authorized() -> Result<(), SolverError> {
    #[cfg(feature = "n512-m512-piecewise-cadv33")]
    return require_n512_gate(
        std::env::var("NSBU_RUN_N512_M512_ENDPOINT_CAPTURE")
            .ok()
            .as_deref(),
    );
    #[cfg(feature = "n512-m512-temporal-h32")]
    return require_temporal_gate(
        std::env::var("NSBU_RUN_N512_TEMPORAL_CAPTURE")
            .ok()
            .as_deref(),
        std::env::var("NSBU_N512_TEMPORAL_REVIEWED_HOST")
            .ok()
            .as_deref(),
    );
    #[cfg(feature = "n512-m512-temporal-h16")]
    return require_temporal_gate(
        std::env::var("NSBU_RUN_N512_TEMPORAL_CAPTURE")
            .ok()
            .as_deref(),
        std::env::var("NSBU_N512_TEMPORAL_REVIEWED_HOST")
            .ok()
            .as_deref(),
    );
    #[cfg(feature = "n256-m512-piecewise-cadv33")]
    return require_n256_gate(
        std::env::var("NSBU_RUN_N256_M512_ENDPOINT_CAPTURE")
            .ok()
            .as_deref(),
    );
    #[cfg(not(any(
        feature = "n512-m512-piecewise-cadv33",
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16",
        feature = "n256-m512-piecewise-cadv33"
    )))]
    Ok(())
}

#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub(crate) fn require_n512_gate(value: Option<&str>) -> Result<(), SolverError> {
    if value == Some("1") {
        Ok(())
    } else {
        Err(SolverError::InvalidPayload)
    }
}

#[cfg(any(
    feature = "n512-m512-temporal-h32",
    feature = "n512-m512-temporal-h16"
))]
pub(crate) fn require_temporal_gate(opt_in: Option<&str>, host: Option<&str>) -> Result<(), SolverError> {
    if opt_in != Some("1") {
        return Err(SolverError::InvalidPayload);
    }
    match host {
        Some("baccus") | Some("sulaco") => Ok(()),
        _ => Err(SolverError::InvalidPayload),
    }
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub(crate) fn require_n256_gate(value: Option<&str>) -> Result<(), SolverError> {
    if value == Some("1") {
        Ok(())
    } else {
        Err(SolverError::InvalidPayload)
    }
}

pub(crate) fn admit() -> Result<Admission, SolverError> {
    let geometry = geometry()?;
    admit_geometry(geometry, CAP)
}

pub(crate) fn geometry() -> Result<Geometry, SolverError> {
    let domain = domain()?;
    geometry_for(domain)
}

fn geometry_for(domain: Domain) -> Result<Geometry, SolverError> {
    let samples = Layout::new([M; 3])?;
    let observer_samples = Layout::new([OBSERVER_M; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    Ok(Geometry {
        domain,
        samples,
        observer_samples,
        backend,
    })
}

pub(crate) fn admit_geometry(geometry: Geometry, cap: usize) -> Result<Admission, SolverError> {
    let reservations = reservations(geometry)?;
    let resources = resources(geometry.domain, reservations, cap)?;
    work_admission(geometry, reservations, resources)
}

pub(crate) fn reservations(geometry: Geometry) -> Result<Reservations, SolverError> {
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

#[cfg(not(feature = "n384-prep"))]
pub(crate) fn execution_reservations(geometry: Geometry) -> Result<(usize, ForceLimits, usize), SolverError> {
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

#[cfg(all(feature = "n384-prep", not(feature = "n512-m512-parallel-capture")))]
pub(crate) fn execution_reservations(geometry: Geometry) -> Result<(usize, ForceLimits, usize), SolverError> {
    let catalog = FftCatalog::reservation(geometry.backend)?;
    let force = CachedReducedForce::preflight(
        geometry.domain,
        geometry.samples,
        WORKERS,
        geometry.backend,
        true,
    )?;
    let rhs = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        geometry.domain,
        force,
        geometry.backend,
    )?;
    Ok((catalog, force, rhs))
}

#[cfg(feature = "n512-m512-parallel-capture")]
pub(crate) fn execution_reservations(geometry: Geometry) -> Result<(usize, ForceLimits, usize), SolverError> {
    let catalog = FftCatalog::reservation(geometry.backend)?;
    let force = CachedReducedForce::preflight(
        geometry.domain,
        geometry.samples,
        WORKERS,
        geometry.backend,
        true,
        Some(FFT_WORKERS),
    )?;
    let rhs = SpectralRhs::<CachedReducedForce>::reservation_with_parallel_w3_fft_backend(
        geometry.domain,
        force,
        geometry.backend,
        FFT_WORKERS,
    )?;
    Ok((catalog, force, rhs))
}

#[cfg(capture_offline)]
pub(crate) fn diagnostic_reservations(geometry: Geometry) -> Result<(usize, usize), SolverError> {
    let attempt = AttemptWorkspace::reservation_with_method(geometry.domain, Method::CoxMatthews)?;
    Ok((attempt, 0))
}

#[cfg(not(capture_offline))]
pub(crate) fn diagnostic_reservations(geometry: Geometry) -> Result<(usize, usize), SolverError> {
    let attempt = AttemptWorkspace::reservation_with_method(geometry.domain, Method::CoxMatthews)?;
    let observer = ReducedObserver::preflight(
        geometry.domain,
        geometry.observer_samples,
        WORKERS,
        geometry.backend,
    )?;
    Ok((attempt, observer))
}

pub(crate) fn resources(domain: Domain, sizes: Reservations, cap: usize) -> Result<ResourcePlan, SolverError> {
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
        cap,
        Epoch(0),
    )
}

pub(crate) fn work_admission(
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

pub(crate) fn disk_bound(domain: Domain) -> Result<usize, SolverError> {
    let snapshot = domain
        .layout()
        .half_len()
        .checked_mul(3 * 16)
        .ok_or(SolverError::SizeOverflow)?;
    #[cfg(all(
        feature = "n384-prep",
        not(feature = "n512-m512-temporal-h16"),
        not(feature = "n256-m512-piecewise-cadv33")
    ))]
    return crate::step_artifact::disk_preflight(snapshot, schedule::MAXIMUM_ATTEMPTS);
    #[cfg(feature = "n512-m512-temporal-h16")]
    return crate::step_artifact::disk_preflight(snapshot, schedule::MAXIMUM_ATTEMPTS);
    #[cfg(feature = "n256-m512-piecewise-cadv33")]
    return crate::step_artifact::disk_preflight(snapshot, schedule::MAXIMUM_ATTEMPTS);
    #[cfg(not(feature = "n384-prep"))]
    artifact::disk_preflight(snapshot, schedule::FINE.len())
}

pub(crate) fn observer_work(geometry: Geometry) -> Result<usize, SolverError> {
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
        "preflight source={} case_sha256={CASE_SHA256} schema={PREFLIGHT_SCHEMA} profile={PROFILE} backend=rustfft-6.4.1-avx-avx2-fma execution={EXECUTION} provider={PROVIDER} retained={N} sampled={M} workers={WORKERS} method=cox-matthews schedule={} maximum_attempts={} endpoint={} advective_limit={ADVECTIVE_LIMIT} observer_nodes={:?} observer_sampled={} nested_middle={:?} nested_coarse={:?} catalog_bytes={} rhs_bytes={} attempt_bytes={} observer_bytes={} overhead={OVERHEAD} total={} cap={CAP} disk_preflight_bytes={} disk_cap_bytes={} integration_work_bound={} observer_work_bound={} profile_identity={} archive_profile=unsupported qualification=experimental",
        env!("RUN_SOURCE"),
        schedule::IDENTITY,
        schedule::MAXIMUM_ATTEMPTS,
        schedule::ENDPOINT,
        schedule::FINE,
        OBSERVER_M,
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
        identity(),
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
