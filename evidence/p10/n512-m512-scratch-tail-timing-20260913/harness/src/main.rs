//! One actual, uncommitted N512/M512 from-rest Cox--Matthews timing attempt.
#[path = "../../../avx-w3-n256-integration-20260912/harness/src/cache.rs"]
mod cache;
#[path = "../../../avx-scheduled-endpoint/harness/src/timed_rhs.rs"]
mod timed_rhs;

use cache::CachedReducedForce;
use nsbu_benchmarks::CASE_SHA256;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace, forcing::ForceLimits, indicator::Tolerances, method::Method,
        rhs::SpectralRhs, transaction::CandidateState,
    },
    spectral::{FftBackend, FftCatalog, W3FftIdentity, W3FftMode, W3FftPool},
    SolverError,
};
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    env,
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};
use timed_rhs::TimedRhs;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const RETAINED: usize = 512;
const SAMPLES: usize = 512;
const WORKERS: usize = 32;
const TICKS: u128 = 64;
const ADVECTIVE_LIMIT: f64 = 3.3;
const CAP: usize = 207_576_840_688;
const OVERHEAD: usize =
    64 * 1024 + TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead();
const FORWARD_ADDITIONAL: usize = 4_318_465_792;
const BIDIRECTIONAL_ADDITIONAL: usize = 21_787_856_768;
const PRODUCTION_SOURCE_COMMIT: &str = "0843b8b18e6a096a0208e3d896e391c7b1b2f5e0";
const TEST_SOURCE_COMMIT: &str = "9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645";
const HARNESS_SOURCE_COMMIT: &str = "PENDING_ADAPTATION_COMMIT";

#[derive(Clone, Copy)]
struct Admission {
    plan: ResourcePlan,
    catalog: usize,
    force: ForceLimits,
    rhs: usize,
    attempt: usize,
}

fn main() -> Result<(), String> {
    match env::args().nth(1).as_deref() {
        Some("preflight") => report_preflight(),
        Some("run") => {
            let output = env::args().nth(2).ok_or("missing create-new output path")?;
            gated_run(Path::new(&output))
        }
        _ => Err("usage: p10-n512-m512-one-attempt-timing preflight | run OUTPUT".into()),
    }
}

fn geometry() -> Result<(Domain, Layout, FftBackend), SolverError> {
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([SAMPLES; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    Ok((domain, samples, backend))
}

fn admit(cap: usize) -> Result<Admission, SolverError> {
    let (domain, samples, backend) = geometry()?;
    if W3FftPool::additional_reservation_with_backend(samples, backend, W3FftMode::Forward)?
        != FORWARD_ADDITIONAL
        || W3FftPool::additional_reservation_with_backend(
            domain.padded_layout()?,
            backend,
            W3FftMode::Bidirectional,
        )? != BIDIRECTIONAL_ADDITIONAL
    {
        return Err(SolverError::InvalidPayload);
    }
    let catalog = FftCatalog::reservation(backend)?;
    let limits = CachedReducedForce::preflight(domain, samples, WORKERS, backend, true)?;
    let rhs = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        domain, limits, backend,
    )?;
    let attempt = AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews)?;
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: catalog,
            force: rhs,
            diagnostics: attempt,
            overhead: OVERHEAD,
        },
        cap,
        Epoch(0),
    )?;
    Ok(Admission {
        plan,
        catalog,
        force: limits,
        rhs,
        attempt,
    })
}

fn report_preflight() -> Result<(), String> {
    let a = admit(CAP).map_err(debug)?;
    if a.plan.total() != CAP || !matches!(admit(CAP - 1), Err(SolverError::ResourceLimit)) {
        return Err("closed peak or one-byte-under admission mismatch".into());
    }
    println!(
        "status=preflight_only production_source={PRODUCTION_SOURCE_COMMIT} test_source={TEST_SOURCE_COMMIT} harness_source={HARNESS_SOURCE_COMMIT} case_sha256={CASE_SHA256} retained={RETAINED} samples={SAMPLES} method=cox-matthews ticks={TICKS} maximum_attempts=1 advective_limit={ADVECTIVE_LIMIT} absolute_tolerance_l2=1e-5 absolute_tolerance_h1=1e-4 relative_tolerance_l2=1e-5 relative_tolerance_h1=1e-5 catalog_bytes={} cached_force_bytes={} force_work_units={} force_scalar_transforms={} rhs_bytes={} attempt_bytes={} overhead_bytes={OVERHEAD} total={CAP}",
        a.catalog, a.force.storage_bytes, a.force.work_units, a.force.scalar_transforms, a.rhs, a.attempt
    );
    Ok(())
}

fn gated_run(output: &Path) -> Result<(), String> {
    if env::var("NSBU_RUN_N512_M512_ONE_ATTEMPT").as_deref() != Ok("1") {
        return Err("refused: missing reviewed one-attempt execution gate".into());
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let mut output = BufWriter::with_capacity(32 * 1024, file);
    let a = admit(CAP).map_err(debug)?;
    let (domain, samples, backend) = geometry().map_err(debug)?;
    let catalog = FftCatalog::new(backend, a.catalog).map_err(debug)?;
    let limits =
        CachedReducedForce::preflight(domain, samples, WORKERS, backend, true).map_err(debug)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        WORKERS,
        &catalog,
        limits.storage_bytes,
        true,
    )
    .map_err(debug)?;
    let rhs = SpectralRhs::new_with_catalog_w3(domain, force, ADVECTIVE_LIMIT, &catalog, a.rhs)
        .map_err(debug)?;
    require_identities(domain, samples, backend, &rhs)?;
    let mut rhs = TimedRhs::new(rhs);
    let clock = TickClock::from_rest(-20, 8192).map_err(debug)?;
    let state = SpectralState::from_rest(a.plan, clock, Epoch(0)).map_err(debug)?;
    let mut candidate = CandidateState::new(a.plan, clock, Epoch(0)).map_err(debug)?;
    let mut attempt =
        AttemptWorkspace::new_with_method(a.plan, Method::CoxMatthews).map_err(debug)?;
    let region = Region::new(GLOBAL);
    let started = Instant::now();
    let result = attempt
        .try_advance(&state, &mut candidate, TICKS, tolerances(), &mut rhs)
        .map_err(debug)?;
    let seconds = started.elapsed().as_secs_f64();
    let allocations = region.change();
    if (
        allocations.allocations,
        allocations.deallocations,
        allocations.reallocations,
    ) != (0, 0, 0)
    {
        return Err("steady attempt allocated".into());
    }
    let measurement = rhs.measurement();
    let hits = rhs.inner().provider().hit_miss();
    let consumption = rhs.inner().consumption();
    if result.rhs_calls != 12 || measurement.calls != 12 || hits != [7, 5] || consumption[0] != 12 {
        return Err("attempt work identity mismatch".into());
    }
    let candidate_sha256 = match result.accepted.as_ref() {
        Some(token) => Some(coefficient_sha256(
            candidate.proposal(&state, token).map_err(debug)?,
        )?),
        None => None,
    };
    writeln!(output, "{{\"schema\":\"p10-n512-m512-scratch-tail-one-attempt-timing-v1\",\"status\":\"actual_from_rest_attempt_complete_uncommitted\",\"production_source_commit\":\"{PRODUCTION_SOURCE_COMMIT}\",\"test_source_commit\":\"{TEST_SOURCE_COMMIT}\",\"harness_source_commit\":\"{HARNESS_SOURCE_COMMIT}\",\"case_sha256\":\"{CASE_SHA256}\",\"retained\":512,\"samples\":512,\"method\":\"cox-matthews\",\"advective_limit\":3.3,\"absolute_tolerances\":[1e-5,1e-4],\"relative_tolerances\":[1e-5,1e-5],\"clock_exponent\":-20,\"clock_target\":8192,\"attempted_from\":0,\"ticks\":64,\"attempted_to\":64,\"maximum_attempts\":1,\"rhs_calls\":{},\"rhs_timed_calls\":{},\"cache_misses\":{},\"cache_hits\":{},\"consumed_work_units\":{},\"consumed_scalar_transforms\":{},\"integration_seconds\":{:.17e},\"rhs_seconds\":{:.17e},\"error_ratio_l2\":{:.17e},\"error_ratio_h1\":{:.17e},\"local_accepted_token_present\":{},\"candidate_coefficient_sha256\":{},\"candidate_hash_encoding\":\"component-major-axis-0-through-2;coefficient-order;re-u64-le-then-im-u64-le\",\"committed\":false,\"published\":false,\"steady_allocations\":0,\"resource_bytes\":{},\"w3_forward_additional_bytes\":{FORWARD_ADDITIONAL},\"w3_bidirectional_additional_bytes\":{BIDIRECTIONAL_ADDITIONAL},\"timer_identity\":\"{}\",\"qualification\":false}}", result.rhs_calls, measurement.calls, hits[1], hits[0], consumption[1], consumption[2], seconds, measurement.seconds, result.indicators.ratios[0], result.indicators.ratios[1], result.accepted.is_some(), candidate_sha256.as_ref().map_or("null".to_string(), |value| format!("\"{value}\"")), a.plan.total(), timed_rhs::IDENTITY).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

fn coefficient_sha256(state: &SpectralState) -> Result<String, String> {
    coefficient_sha256_components([
        state.component(0).map_err(debug)?,
        state.component(1).map_err(debug)?,
        state.component(2).map_err(debug)?,
    ])
}

fn coefficient_sha256_components(
    components: [&[nsbu_solver::Complex64]; 3],
) -> Result<String, String> {
    let mut hash = Sha256::new();
    for component in components {
        for value in component {
            hash.update(value.re.to_bits().to_le_bytes());
            hash.update(value.im.to_bits().to_le_bytes());
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn require_identities(
    domain: Domain,
    samples: Layout,
    backend: FftBackend,
    rhs: &SpectralRhs<CachedReducedForce>,
) -> Result<(), String> {
    let expected_rhs = W3FftIdentity {
        layout: domain.padded_layout().map_err(debug)?,
        backend,
        width: 3,
        mode: W3FftMode::Bidirectional,
        additional_bytes: BIDIRECTIONAL_ADDITIONAL,
    };
    let expected_force = W3FftIdentity {
        layout: samples,
        backend,
        width: 3,
        mode: W3FftMode::Forward,
        additional_bytes: FORWARD_ADDITIONAL,
    };
    if rhs.w3_fft_identity() != Some(expected_rhs)
        || rhs.provider().w3_identity() != Some(expected_force)
    {
        return Err("exact W3 identity mismatch".into());
    }
    Ok(())
}

fn tolerances() -> Tolerances {
    Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    }
}

fn debug(error: SolverError) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests;
