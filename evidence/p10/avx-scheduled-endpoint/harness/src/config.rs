//! Immutable endpoint profile and its complete memory/work/disk admission.
#[cfg(not(capture_offline))]
use crate::observer::ReducedObserver;
use crate::{artifact, cache::CachedReducedForce, schedule, timed_rhs::TimedRhs};
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

#[cfg(all(feature = "n256", feature = "n384-prep"))]
compile_error!("n256 and n384-prep are mutually exclusive profiles");
#[cfg(all(feature = "n384-h32", feature = "n384-h64"))]
compile_error!("n384-h32 and n384-h64 are mutually exclusive profiles");
#[cfg(all(
    feature = "n512-m512-piecewise-cadv33",
    any(
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("n512-m512-piecewise-cadv33 is mutually exclusive with every n384 profile");
#[cfg(all(
    feature = "n256-m512-piecewise-cadv33",
    any(
        feature = "n256",
        feature = "n512-m512-piecewise-cadv33",
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("n256-m512-piecewise-cadv33 is a mutually exclusive profile");
#[cfg(any(
    all(feature = "n384-h32", feature = "n384-piecewise"),
    all(feature = "n384-h64", feature = "n384-piecewise"),
    all(feature = "n384-h32", feature = "n384-piecewise-cadv33"),
    all(feature = "n384-h64", feature = "n384-piecewise-cadv33"),
    all(feature = "n384-piecewise", feature = "n384-piecewise-cadv33"),
    all(feature = "n384-h32", feature = "n384-m512-piecewise-cadv33"),
    all(feature = "n384-h64", feature = "n384-m512-piecewise-cadv33"),
    all(feature = "n384-piecewise", feature = "n384-m512-piecewise-cadv33"),
    all(
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("select only one exact n384 profile");
#[cfg(all(
    feature = "n384-prep",
    not(any(
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33",
        feature = "n512-m512-piecewise-cadv33",
        feature = "n256-m512-piecewise-cadv33"
    ))
))]
compile_error!("select an exact top-level n384 feature");

#[cfg(not(any(feature = "n256", feature = "n384-prep")))]
pub const N: usize = 192;
#[cfg(all(feature = "n256", not(feature = "n384-prep")))]
pub const N: usize = 256;
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n256"),
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub const N: usize = 384;
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub const N: usize = 512;
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub const N: usize = 256;
#[cfg(not(any(
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
)))]
pub const M: usize = 384;
#[cfg(any(
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
))]
pub const M: usize = 512;
#[cfg(not(any(
    feature = "n512-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
)))]
pub const OBSERVER_M: usize = 768;
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub const OBSERVER_M: usize = 1024;
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub const OBSERVER_M: usize = 512;
pub const WORKERS: usize = 32;
#[cfg(not(feature = "n384-prep"))]
pub const CAP: usize = 103_079_215_104;
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub const CAP: usize = 192 * 1024 * 1024 * 1024;
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub const CAP: usize = 207_627_451_152;
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub const CAP: usize = 68_719_476_736;
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub const FFT_WORKERS: usize = 8;
#[cfg(not(feature = "n384-prep"))]
pub const ADVECTIVE_LIMIT: f64 = 0.45;
#[cfg(all(feature = "n384-prep", not(feature = "n384-piecewise-common")))]
pub const ADVECTIVE_LIMIT: f64 = 0.8;
#[cfg(feature = "n384-piecewise")]
pub const ADVECTIVE_LIMIT: f64 = 1.6;
#[cfg(any(
    feature = "n384-piecewise-cadv33",
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
))]
pub const ADVECTIVE_LIMIT: f64 = 3.3;
const HISTORY_BYTES: usize = schedule::MAXIMUM_ATTEMPTS * 4096;
const TIMER_OVERHEAD: usize = TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead();
const OVERHEAD: usize = artifact::BUFFER_BYTES + HISTORY_BYTES + TIMER_OVERHEAD + 64 * 1024;

#[cfg(not(any(feature = "n256", feature = "n384-prep")))]
const PROFILE: &str = "n192-m384";
#[cfg(all(feature = "n256", not(feature = "n384-prep")))]
const PROFILE: &str = "n256-m384";
#[cfg(all(feature = "n384-h32", not(feature = "n256")))]
const PROFILE: &str = "n384-m384-h32-cadv08-w3-f13c29c";
#[cfg(all(feature = "n384-h64", not(feature = "n256")))]
const PROFILE: &str = "n384-m384-h64-cadv08-w3-f13c29c";
#[cfg(all(feature = "n384-piecewise", not(feature = "n256")))]
const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv16-w3-f13c29c";
#[cfg(all(feature = "n384-piecewise-cadv33", not(feature = "n256")))]
const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";
#[cfg(feature = "n384-m512-piecewise-cadv33")]
const PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
#[cfg(feature = "n512-m512-piecewise-cadv33")]
const PROFILE: &str = "n512-m512-h64to2048-h128to4096-cadv33-w3-pfft1ed6995";
#[cfg(feature = "n256-m512-piecewise-cadv33")]
const PROFILE: &str = "n256-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
const PREFLIGHT_SCHEMA: &str = "p10-avx-n384-preflight-v1";
#[cfg(feature = "n512-m512-piecewise-cadv33")]
const PREFLIGHT_SCHEMA: &str = "p10-avx-n512-m512-endpoint-capture-preflight-v1";
#[cfg(feature = "n256-m512-piecewise-cadv33")]
const PREFLIGHT_SCHEMA: &str = "p10-avx-n256-m512-endpoint-capture-preflight-v1";
#[cfg(not(feature = "n384-prep"))]
const PREFLIGHT_SCHEMA: &str = "p10-avx-scheduled-endpoint-v2";
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
const EXECUTION: &str = "separate-rhs-force-w3";
#[cfg(feature = "n512-m512-piecewise-cadv33")]
const EXECUTION: &str = "separate-rhs-force-w3-parallel8";
#[cfg(feature = "n256-m512-piecewise-cadv33")]
const EXECUTION: &str = "separate-rhs-force-w3-offline-capture";
#[cfg(not(feature = "n384-prep"))]
const EXECUTION: &str = "serial-component-fft";
#[cfg(all(feature = "n384-prep", not(feature = "n512-m512-piecewise-cadv33")))]
const PROVIDER: &str = "parallel-reduced-v2-force-w3-attempt-cache";
#[cfg(feature = "n512-m512-piecewise-cadv33")]
const PROVIDER: &str = "parallel-reduced-v2-force-w3-parallel8-attempt-cache";
#[cfg(not(feature = "n384-prep"))]
const PROVIDER: &str = "parallel-reduced-attempt-cache";
#[cfg(all(feature = "n384-prep", not(feature = "n384-piecewise-common")))]
const EXTERNAL_STOP: &str = "pgid-watchdog-v1-starttime-cmdline-deadline";
#[cfg(all(
    feature = "n384-piecewise-common",
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
const EXTERNAL_STOP: &str = "pgid-watchdog-v2-starttime-cmdline-deadline";
#[cfg(any(
    feature = "n512-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
))]
const EXTERNAL_STOP: &str = "pgid-watchdog-v3-confirmed-identity-absolute-deadline";

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
    #[cfg(feature = "n256-m512-piecewise-cadv33")]
    return require_n256_gate(
        std::env::var("NSBU_RUN_N256_M512_ENDPOINT_CAPTURE")
            .ok()
            .as_deref(),
    );
    #[cfg(not(any(
        feature = "n512-m512-piecewise-cadv33",
        feature = "n256-m512-piecewise-cadv33"
    )))]
    Ok(())
}

#[cfg(feature = "n512-m512-piecewise-cadv33")]
fn require_n512_gate(value: Option<&str>) -> Result<(), SolverError> {
    if value == Some("1") {
        Ok(())
    } else {
        Err(SolverError::InvalidPayload)
    }
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn require_n256_gate(value: Option<&str>) -> Result<(), SolverError> {
    if value == Some("1") {
        Ok(())
    } else {
        Err(SolverError::InvalidPayload)
    }
}

fn admit() -> Result<Admission, SolverError> {
    let geometry = geometry()?;
    admit_geometry(geometry, CAP)
}

fn geometry() -> Result<Geometry, SolverError> {
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

fn admit_geometry(geometry: Geometry, cap: usize) -> Result<Admission, SolverError> {
    let reservations = reservations(geometry)?;
    let resources = resources(geometry.domain, reservations, cap)?;
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

#[cfg(not(feature = "n384-prep"))]
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

#[cfg(all(feature = "n384-prep", not(feature = "n512-m512-piecewise-cadv33")))]
fn execution_reservations(geometry: Geometry) -> Result<(usize, ForceLimits, usize), SolverError> {
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

#[cfg(feature = "n512-m512-piecewise-cadv33")]
fn execution_reservations(geometry: Geometry) -> Result<(usize, ForceLimits, usize), SolverError> {
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
fn diagnostic_reservations(geometry: Geometry) -> Result<(usize, usize), SolverError> {
    let attempt = AttemptWorkspace::reservation_with_method(geometry.domain, Method::CoxMatthews)?;
    Ok((attempt, 0))
}

#[cfg(not(capture_offline))]
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

fn resources(domain: Domain, sizes: Reservations, cap: usize) -> Result<ResourcePlan, SolverError> {
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
    #[cfg(all(feature = "n384-prep", not(feature = "n512-m512-piecewise-cadv33")))]
    return crate::step_artifact::disk_preflight(snapshot, schedule::MAXIMUM_ATTEMPTS);
    #[cfg(feature = "n512-m512-piecewise-cadv33")]
    return crate::step_artifact::disk_preflight(snapshot, schedule::MAXIMUM_ATTEMPTS);
    #[cfg(not(feature = "n384-prep"))]
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

pub fn identity() -> String {
    #[cfg(feature = "n512-m512-piecewise-cadv33")]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;library_source=1ed699568be70dedf72492324be08600d4407c02;prototype_source=b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5;provider=parallel-reduced-v2-force-w3-parallel8-attempt-cache;rhs_w3=layout768-width3-bidirectional-add21812652048;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};observer_execution=offline-baccus-required;sampling_workers={WORKERS};rhs_w3_persistent_callers=3;rhs_fft_helpers={FFT_WORKERS};rhs_fft_total_workers=11;provider_w3_persistent_callers=3;provider_fft_helpers={FFT_WORKERS};provider_fft_total_workers=11;method=cox-matthews;schedule={};endpoint={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema=p10-avx-n512-observer-state-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(all(
        feature = "n384-prep",
        not(feature = "n512-m512-piecewise-cadv33"),
        not(feature = "n256-m512-piecewise-cadv33")
    ))]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;provider=parallel-reduced-v2-force-w3-attempt-cache;rhs_w3=layout576-width3-bidirectional-add9200926592;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};sampling_workers={WORKERS};rhs_w3_workers=3;provider_w3_workers=3;method=cox-matthews;schedule={};endpoint={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema=p10-avx-n384-every-step-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(feature = "n256-m512-piecewise-cadv33")]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;provider=parallel-reduced-v2-force-w3-attempt-cache;rhs_w3=layout384-width3-bidirectional-add2734010240;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};observer_execution=offline-baccus-required;rhs_dealias=384;sampling_workers={WORKERS};rhs_w3_workers=3;provider_w3_workers=3;method=cox-matthews;schedule={};endpoint={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema=p10-avx-n256-m512-observer-state-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=baccus;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(not(feature = "n384-prep"))]
    format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;provider=parallel-reduced-attempt-cache;rhs_timer={};n={N};m={M};workers={WORKERS};method=cox-matthews;step={};endpoint={};advective_limit={ADVECTIVE_LIMIT};cap={CAP};schema=p10-avx-scheduled-endpoint-v2;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported",
        env!("RUN_SOURCE"),
        crate::timed_rhs::IDENTITY,
        schedule::STEP,
        schedule::ENDPOINT,
    )
}

#[cfg(all(
    feature = "n384-prep",
    not(feature = "n384-m512-piecewise-cadv33"),
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
fn force_w3_identity() -> &'static str {
    "layout384-width3-forward-add1828040448"
}

#[cfg(any(
    feature = "n384-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
))]
fn force_w3_identity() -> &'static str {
    "layout512-width3-forward-add4318465792"
}

#[cfg(feature = "n512-m512-piecewise-cadv33")]
fn force_w3_identity() -> &'static str {
    "layout512-width3-forward-add4343035792"
}

#[cfg(all(
    test,
    feature = "n384-prep",
    not(feature = "n512-m512-piecewise-cadv33"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
mod n384_tests {
    use super::*;

    #[cfg(feature = "n384-h32")]
    const EXPECTED_N384_TOTAL: usize = 185_783_726_856;
    #[cfg(feature = "n384-h64")]
    const EXPECTED_N384_TOTAL: usize = 185_783_464_712;
    #[cfg(all(
        feature = "n384-piecewise-common",
        not(feature = "n384-m512-piecewise-cadv33")
    ))]
    const EXPECTED_N384_TOTAL: usize = 185_783_399_176;
    #[cfg(feature = "n384-m512-piecewise-cadv33")]
    const EXPECTED_N384_TOTAL: usize = 193_243_685_640;

    #[test]
    fn selected_profile_is_exact_and_execution_ready() {
        assert_eq!(N, 384);
        #[cfg(not(feature = "n384-m512-piecewise-cadv33"))]
        assert_eq!(M, 384);
        #[cfg(feature = "n384-m512-piecewise-cadv33")]
        assert_eq!(M, 512);
        assert_eq!(OBSERVER_M, 768);
        #[cfg(not(feature = "n384-piecewise-common"))]
        assert_eq!(ADVECTIVE_LIMIT, 0.8);
        #[cfg(feature = "n384-piecewise")]
        assert_eq!(ADVECTIVE_LIMIT, 1.6);
        #[cfg(any(
            feature = "n384-piecewise-cadv33",
            feature = "n384-m512-piecewise-cadv33"
        ))]
        assert_eq!(ADVECTIVE_LIMIT, 3.3);
        assert_eq!(CAP, 206_158_430_208);
        assert_eq!(require_execution_ready(), Ok(()));
        let identity = identity();
        assert!(identity.contains("provider=parallel-reduced-v2-force-w3-attempt-cache"));
        assert!(identity.contains("rhs_w3=layout576-width3-bidirectional-add9200926592"));
        assert!(identity.contains(&format!("force_w3={}", force_w3_identity())));
        assert!(identity.contains("observer_force_samples=768"));
        assert!(identity.contains("observer_conservative=768"));
        assert!(identity.contains("numa=whole-host-unbound-all-visible-cpus-memory"));
        assert!(identity.contains(&format!("external_stop={EXTERNAL_STOP}")));
        assert!(identity.contains(&format!("artifact_cap={}", artifact::DISK_CAP_BYTES)));
    }

    #[test]
    fn complete_w3_reservation_is_available_without_execution_admission() {
        let geometry = geometry().unwrap();
        let admission = admit_geometry(geometry, CAP).unwrap();
        let total = admission.resources.total();
        println!(
            "n384_complete_reservation={total} classes={:?} catalog={} force_storage={} rhs={} attempt={} observer={} overhead={} disk={}",
            admission.resources.classes(),
            admission.reservations.catalog,
            admission.reservations.force.storage_bytes,
            admission.reservations.rhs,
            admission.reservations.attempt,
            admission.reservations.observer,
            OVERHEAD,
            admission.disk,
        );
        assert_eq!(total, EXPECTED_N384_TOTAL);
        #[cfg(feature = "n384-m512-piecewise-cadv33")]
        {
            assert_eq!(admission.reservations.force.storage_bytes, 19_816_721_864);
            assert_eq!(admission.reservations.rhs, 46_267_774_008);
            assert_eq!(admission.reservations.observer, 98_075_369_752);
            assert_eq!(admission.integration_work, 10_022_091_230_016);
            assert_eq!(admission.observer_work, 467_480_346_624);
            assert_eq!(admission.disk, 65_573_289_984);
            assert_eq!(CAP - total, 12_914_744_568);
            assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
            assert_eq!(tolerances().relative, [1e-5; 2]);
        }
        assert!(matches!(
            admit_geometry(geometry, total - 1),
            Err(SolverError::ResourceLimit)
        ));
    }
}

#[cfg(all(test, feature = "n512-m512-piecewise-cadv33"))]
mod n512_resource_probe {
    use super::*;

    #[test]
    fn report_each_exact_api_reservation_without_allocation() {
        assert!(identity()
            .contains("external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline"));
        assert!(identity().contains("rhs_fft_helpers=8;rhs_fft_total_workers=11"));
        assert!(identity().contains("provider_fft_helpers=8;provider_fft_total_workers=11"));
        let geometry = geometry().unwrap();
        let (catalog, force, rhs) = execution_reservations(geometry).unwrap();
        assert_eq!(
            (catalog, force.storage_bytes, rhs),
            (29_362_480, 29_180_171_864, 91_860_200_792)
        );
        assert_eq!(
            diagnostic_reservations(geometry).unwrap(),
            (30_182_212_728, 0)
        );
        let observer = crate::observer::ReducedObserver::preflight(
            geometry.domain,
            geometry.observer_samples,
            WORKERS,
            geometry.backend,
        )
        .unwrap();
        assert_eq!(observer, 232_283_988_248);
        assert_eq!(
            nsbu_solver::spectral::W3FftPool::additional_parallel_reservation_with_backend(
                geometry.domain.padded_layout().unwrap(),
                geometry.backend,
                nsbu_solver::spectral::W3FftMode::Bidirectional,
                FFT_WORKERS,
            )
            .unwrap(),
            21_812_652_048,
        );
        assert_eq!(
            nsbu_solver::spectral::W3FftPool::additional_parallel_reservation_with_backend(
                geometry.samples,
                geometry.backend,
                nsbu_solver::spectral::W3FftMode::Forward,
                FFT_WORKERS,
            )
            .unwrap(),
            4_343_035_792,
        );
        let reservations = reservations(geometry).unwrap();
        assert_eq!(
            resources(geometry.domain, reservations, CAP)
                .unwrap()
                .total(),
            CAP
        );
        assert_eq!(
            resources(geometry.domain, reservations, CAP - 1),
            Err(SolverError::ResourceLimit)
        );
        let combined = Reservations {
            observer,
            ..reservations
        };
        assert_eq!(
            resources(geometry.domain, combined, usize::MAX)
                .unwrap()
                .total(),
            439_911_439_400
        );
        assert_eq!(schedule::validate(), Ok(()));
        assert_eq!(admit().unwrap().resources.total(), CAP);
        assert_eq!(disk_bound(geometry.domain), Ok(155_226_537_984));
        assert_eq!(observer_work(geometry), Ok(1_108_101_562_368));
    }

    #[test]
    fn run_gate_is_exact_and_preflight_remains_available() {
        assert_eq!(require_n512_gate(None), Err(SolverError::InvalidPayload));
        assert_eq!(
            require_n512_gate(Some("0")),
            Err(SolverError::InvalidPayload)
        );
        assert_eq!(require_n512_gate(Some("1")), Ok(()));
        assert_eq!(preflight().unwrap().total(), CAP);
    }
}

#[cfg(all(test, feature = "n256-m512-piecewise-cadv33"))]
mod n256_m512_resource_probe {
    use super::*;

    const EXPECTED_TOTAL: usize = 37_402_593_776;

    #[test]
    fn profile_is_exact_n256_and_distinct_from_n384_and_n512() {
        assert_eq!(N, 256);
        assert_eq!(M, 512);
        assert_eq!(OBSERVER_M, 512);
        assert_eq!(ADVECTIVE_LIMIT, 3.3);
        assert_eq!(CAP, 68_719_476_736);
        assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
        assert_eq!(tolerances().relative, [1e-5; 2]);
        assert_eq!(schedule::MAXIMUM_ATTEMPTS, 48);
        assert_eq!(schedule::ENDPOINT, 4096);
        assert_eq!(require_execution_ready(), Ok(()));
        let identity = identity();
        assert!(identity.contains("profile=n256-m512-h64to2048-h128to4096-cadv33-w3-f13c29c"));
        assert!(identity.contains("retained=256"));
        assert!(identity.contains("force_samples=512"));
        assert!(identity.contains("rhs_dealias=384"));
        assert!(identity.contains("rhs_w3=layout384-width3-bidirectional-add2734010240"));
        assert!(identity.contains("force_w3=layout512-width3-forward-add4318465792"));
        assert!(identity.contains("observer_execution=offline-baccus-required"));
        assert!(identity.contains("schema=p10-avx-n256-m512-observer-state-v1"));
        assert!(identity.contains("provider=parallel-reduced-v2-force-w3-attempt-cache"));
        assert!(identity.contains("rhs_w3_workers=3"));
        assert!(identity.contains("host=baccus"));
        assert!(identity
            .contains("external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline"));
        assert!(identity.contains("observer_force_samples=512"));
        assert!(identity.contains("observer_conservative=512"));
        assert!(!identity.contains("host=sulaco"));
        assert!(!identity.contains("pgid-watchdog-v2"));
        assert!(!identity.contains("n512"));
        assert!(!identity.contains("n384"));
        assert!(!identity.contains("parallel8"));
    }

    #[test]
    fn scalar_w3_capture_reservations_are_exact_without_numerical_arrays() {
        let geometry = geometry().unwrap();
        let (catalog, force, rhs) = execution_reservations(geometry).unwrap();
        assert_eq!((catalog, rhs), (29_362_480, 22_866_181_176));
        assert_eq!(
            force.storage_bytes,
            15_015_554_504,
            "M512 scalar W3 force storage"
        );
        assert_eq!(
            diagnostic_reservations(geometry).unwrap(),
            (3_787_457_656, 0),
            "offline capture reserves no inline observer"
        );
        let reservations = reservations(geometry).unwrap();
        let plan = resources(geometry.domain, reservations, CAP).unwrap();
        let admission = work_admission(geometry, reservations, plan).unwrap();
        assert_eq!(admission.resources.total(), EXPECTED_TOTAL);
        assert_eq!(admission.integration_work, 9_987_522_825_024);
        assert_eq!(admission.observer_work, 138_512_695_296);
        assert_eq!(disk_bound(geometry.domain), Ok(19_482_083_328));
        assert_eq!(
            resources(geometry.domain, reservations, EXPECTED_TOTAL - 1),
            Err(SolverError::ResourceLimit)
        );
        assert_eq!(
            resources(geometry.domain, reservations, CAP).unwrap().total(),
            EXPECTED_TOTAL
        );
    }

    #[test]
    fn run_gate_is_opt_in_and_preflight_admits() {
        assert_eq!(require_n256_gate(None), Err(SolverError::InvalidPayload));
        assert_eq!(
            require_n256_gate(Some("0")),
            Err(SolverError::InvalidPayload)
        );
        assert_eq!(require_n256_gate(Some("1")), Ok(()));
        assert_eq!(preflight().unwrap().total(), EXPECTED_TOTAL);
        assert_eq!(admit().unwrap().resources.total(), EXPECTED_TOTAL);
    }
}
