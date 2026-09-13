//! Baseline-versus-opt-in-parallel N512/M512 from-rest one-attempt harness.
mod cache;
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
const FFT_WORKERS: usize = 8;
const TICKS: u128 = 64;
const ADVECTIVE_LIMIT: f64 = 3.3;
const BASE_CAP: usize = 207_576_840_688;
const OVERHEAD: usize =
    64 * 1024 + TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead();
const FORWARD_ADDITIONAL: usize = 4_318_465_792;
const BIDIRECTIONAL_ADDITIONAL: usize = 21_787_856_768;
const PRODUCTION_SOURCE_COMMIT: &str = "0843b8b18e6a096a0208e3d896e391c7b1b2f5e0";
const TEST_SOURCE_COMMIT: &str = "9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645";
const PARALLEL_EXECUTOR_SOURCE_COMMIT: &str = "b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5";

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
        Some("preflight-baseline") => report_preflight(false),
        Some("preflight-parallel") => report_preflight(true),
        Some("run-baseline") | Some("run-parallel") => {
            let parallel = env::args().nth(1).as_deref() == Some("run-parallel");
            let output = env::args().nth(2).ok_or("missing create-new output path")?;
            gated_run(Path::new(&output), parallel)
        }
        _ => Err("usage: p10-n512-m512-parallel-fft-timing preflight-baseline | preflight-parallel | run-baseline OUTPUT | run-parallel OUTPUT".into()),
    }
}

fn geometry() -> Result<(Domain, Layout, FftBackend), SolverError> {
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([SAMPLES; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    Ok((domain, samples, backend))
}

fn admit(cap: usize, parallel: bool) -> Result<Admission, SolverError> {
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
    let limits = CachedReducedForce::preflight(
        domain,
        samples,
        WORKERS,
        backend,
        true,
        parallel.then_some(FFT_WORKERS),
    )?;
    let rhs = if parallel {
        SpectralRhs::<CachedReducedForce>::reservation_with_parallel_w3_fft_backend(
            domain,
            limits,
            backend,
            FFT_WORKERS,
        )?
    } else {
        SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(domain, limits, backend)?
    };
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

fn report_preflight(parallel: bool) -> Result<(), String> {
    let required = admit(usize::MAX, parallel).map_err(debug)?.plan.total();
    let a = admit(required, parallel).map_err(debug)?;
    if !matches!(
        admit(required - 1, parallel),
        Err(SolverError::ResourceLimit)
    ) {
        return Err("one-byte-under admission mismatch".into());
    }
    if !parallel && required != BASE_CAP {
        return Err("baseline resource identity changed".into());
    }
    println!("status=preflight_only mode={} executor_source={} retained={RETAINED} samples={SAMPLES} fft_workers={} ticks={TICKS} catalog_bytes={} force_bytes={} rhs_bytes={} attempt_bytes={} overhead_bytes={OVERHEAD} total={}", if parallel {"parallel"} else {"baseline"}, PARALLEL_EXECUTOR_SOURCE_COMMIT, if parallel {FFT_WORKERS} else {0}, a.catalog, a.force.storage_bytes, a.rhs, a.attempt, a.plan.total());
    Ok(())
}

fn gated_run(output: &Path, parallel: bool) -> Result<(), String> {
    if env::var("NSBU_RUN_N512_M512_PARALLEL_FFT_ONE_ATTEMPT").as_deref() != Ok("1") {
        return Err("refused: missing reviewed one-attempt execution gate".into());
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    let mut output = BufWriter::with_capacity(32 * 1024, file);
    let required = admit(usize::MAX, parallel).map_err(debug)?.plan.total();
    let a = admit(required, parallel).map_err(debug)?;
    let (domain, samples, backend) = geometry().map_err(debug)?;
    let catalog = FftCatalog::new(backend, a.catalog).map_err(debug)?;
    let limits = CachedReducedForce::preflight(
        domain,
        samples,
        WORKERS,
        backend,
        true,
        parallel.then_some(FFT_WORKERS),
    )
    .map_err(debug)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        WORKERS,
        &catalog,
        limits.storage_bytes,
        true,
        parallel.then_some(FFT_WORKERS),
    )
    .map_err(debug)?;
    let rhs = if parallel {
        SpectralRhs::new_with_catalog_parallel_w3(
            domain,
            force,
            ADVECTIVE_LIMIT,
            &catalog,
            FFT_WORKERS,
            a.rhs,
        )
    } else {
        SpectralRhs::new_with_catalog_w3(domain, force, ADVECTIVE_LIMIT, &catalog, a.rhs)
    }
    .map_err(debug)?;
    require_identities(domain, samples, backend, &rhs, parallel)?;
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
    writeln!(output, "{{\"schema\":\"p10-n512-m512-parallel-fft-one-attempt-v1\",\"status\":\"actual_from_rest_attempt_complete_uncommitted\",\"fft_execution_mode\":\"{}\",\"fft_workers\":{},\"production_source_commit\":\"{PRODUCTION_SOURCE_COMMIT}\",\"test_source_commit\":\"{TEST_SOURCE_COMMIT}\",\"executor_source_commit\":\"{PARALLEL_EXECUTOR_SOURCE_COMMIT}\",\"case_sha256\":\"{CASE_SHA256}\",\"retained\":512,\"samples\":512,\"method\":\"cox-matthews\",\"advective_limit\":3.3,\"absolute_tolerances\":[1e-5,1e-4],\"relative_tolerances\":[1e-5,1e-5],\"clock_exponent\":-20,\"clock_target\":8192,\"attempted_from\":0,\"ticks\":64,\"attempted_to\":64,\"maximum_attempts\":1,\"rhs_calls\":{},\"rhs_timed_calls\":{},\"cache_misses\":{},\"cache_hits\":{},\"consumed_work_units\":{},\"consumed_scalar_transforms\":{},\"integration_seconds\":{:.17e},\"rhs_seconds\":{:.17e},\"error_ratio_l2\":{:.17e},\"error_ratio_h1\":{:.17e},\"local_accepted_token_present\":{},\"candidate_coefficient_sha256\":{},\"candidate_hash_encoding\":\"component-major-axis-0-through-2;coefficient-order;re-u64-le-then-im-u64-le\",\"committed\":false,\"published\":false,\"steady_allocations\":0,\"resource_bytes\":{},\"timer_identity\":\"{}\",\"qualification\":false}}", if parallel { "parallel" } else { "baseline" }, if parallel { FFT_WORKERS } else { 0 }, result.rhs_calls, measurement.calls, hits[1], hits[0], consumption[1], consumption[2], seconds, measurement.seconds, result.indicators.ratios[0], result.indicators.ratios[1], result.accepted.is_some(), candidate_sha256.as_ref().map_or("null".to_string(), |value| format!("\"{value}\"")), a.plan.total(), timed_rhs::IDENTITY).map_err(|e| e.to_string())?;
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
    parallel: bool,
) -> Result<(), String> {
    let rhs_additional = if parallel {
        W3FftPool::additional_parallel_reservation_with_backend(
            domain.padded_layout().map_err(debug)?,
            backend,
            W3FftMode::Bidirectional,
            FFT_WORKERS,
        )
        .map_err(debug)?
    } else {
        BIDIRECTIONAL_ADDITIONAL
    };
    let force_additional = if parallel {
        W3FftPool::additional_parallel_reservation_with_backend(
            samples,
            backend,
            W3FftMode::Forward,
            FFT_WORKERS,
        )
        .map_err(debug)?
    } else {
        FORWARD_ADDITIONAL
    };
    let expected_rhs = W3FftIdentity {
        layout: domain.padded_layout().map_err(debug)?,
        backend,
        width: 3,
        mode: W3FftMode::Bidirectional,
        additional_bytes: rhs_additional,
    };
    let expected_force = W3FftIdentity {
        layout: samples,
        backend,
        width: 3,
        mode: W3FftMode::Forward,
        additional_bytes: force_additional,
    };
    if rhs.w3_fft_identity() != Some(expected_rhs)
        || rhs.provider().w3_identity() != Some(expected_force)
    {
        return Err("exact W3 identity mismatch".into());
    }
    let rhs_parallel = rhs.parallel_fft_identity();
    let force_parallel = rhs.provider().parallel_fft_identity();
    if parallel {
        for (identity, layout) in [
            (rhs_parallel, expected_rhs.layout),
            (force_parallel, expected_force.layout),
        ] {
            let identity = identity.ok_or("missing parallel FFT identity")?;
            if identity.layout != layout
                || identity.backend != backend
                || identity.workers != FFT_WORKERS
            {
                return Err("parallel FFT identity mismatch".into());
            }
        }
    } else if rhs_parallel.is_some() || force_parallel.is_some() {
        return Err("baseline unexpectedly attached parallel FFT identity".into());
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
