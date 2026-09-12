mod cache;
mod observer;

use cache::CachedReducedForce;
use nsbu_benchmarks::CASE_SHA256;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        method::Method,
        rhs::SpectralRhs,
        transaction::{commit_candidate, CandidateState},
    },
    spectral::{FftBackend, FftCatalog},
    SolverError,
};
use observer::ReducedObserver;
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, time::Instant};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const CAP_32_GIB: usize = 34_359_738_368;
const CAP_96_GIB: usize = 103_079_215_104;
const OVERHEAD: usize = 64 * 1024;

struct Configuration {
    n: usize,
    m: usize,
    workers: usize,
    ticks: u128,
    attempts: usize,
    preflight_only: bool,
    w3: bool,
    cap: usize,
}

fn main() -> Result<(), SolverError> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [n, m, workers, ticks, attempts, mode] = arguments.as_slice() else {
        return Err(SolverError::InvalidPayload);
    };
    let (preflight_only, w3, cap) = match mode.as_str() {
        "serial-preflight32" => (true, false, CAP_32_GIB),
        "serial-preflight96" => (true, false, CAP_96_GIB),
        "serial-run96" => (false, false, CAP_96_GIB),
        "w3-preflight96" => (true, true, CAP_96_GIB),
        "w3-run96" => (false, true, CAP_96_GIB),
        _ => return Err(SolverError::InvalidPayload),
    };
    execute(Configuration {
        n: parse(n)?,
        m: parse(m)?,
        workers: parse(workers)?,
        ticks: parse(ticks)?,
        attempts: parse(attempts)?,
        preflight_only,
        w3,
        cap,
    })
}

fn execute(configuration: Configuration) -> Result<(), SolverError> {
    let Configuration {
        n,
        m,
        workers,
        ticks,
        attempts: attempts_count,
        preflight_only,
        w3,
        cap,
    } = configuration;
    if attempts_count == 0 || !ticks.is_multiple_of(2) {
        return Err(SolverError::InvalidPayload);
    }
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([m; 3])?;
    let observer_length = m.checked_mul(2).ok_or(SolverError::SizeOverflow)?;
    let observer_samples = Layout::new([observer_length; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let force_limits = CachedReducedForce::preflight(domain, samples, workers, backend, w3)?;
    let rhs_bytes = if w3 {
        SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
            domain,
            force_limits,
            backend,
        )?
    } else {
        SpectralRhs::<CachedReducedForce>::reservation_with_fft_backend(
            domain,
            force_limits,
            backend,
        )?
    };
    let attempt_bytes = AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews)?;
    let observer_bytes = ReducedObserver::preflight(domain, observer_samples, workers, backend)?;
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
        cap,
        Epoch(0),
    )?;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} arithmetic=rustfft-6.4.1-avx-avx2-fma execution={} provider=parallel-reduced-attempt-cache observer=parallel-reduced-2m retained={n} sampled={m} observer_sampled={} workers={workers} ticks={ticks} attempts={attempts_count} catalog_bytes={catalog_bytes} force_limits={force_limits:?} rhs_bytes={rhs_bytes} attempt_bytes={attempt_bytes} observer_bytes={observer_bytes} overhead={OVERHEAD} total={} cap={cap}",
        env!("RUN_SOURCE"),
        if w3 { "separate-rhs-force-w3" } else { "serial" },
        2 * m,
        resources.total(),
    );
    if preflight_only {
        return Ok(());
    }

    let catalog = FftCatalog::new(backend, catalog_bytes)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        workers,
        &catalog,
        force_limits.storage_bytes,
        w3,
    )?;
    let mut rhs = if w3 {
        SpectralRhs::new_with_catalog_w3(domain, force, 0.3, &catalog, rhs_bytes)?
    } else {
        SpectralRhs::new_with_catalog(domain, force, 0.3, &catalog, rhs_bytes)?
    };
    println!(
        "execution_identity mode={} rhs_w3={:?} force_w3={:?}",
        if w3 {
            "separate-rhs-force-w3"
        } else {
            "serial"
        },
        rhs.w3_fft_identity(),
        rhs.provider().w3_identity(),
    );
    let mut observer =
        ReducedObserver::new(domain, observer_samples, workers, &catalog, observer_bytes)?;
    let clock = TickClock::from_rest(-20, 8192)?;
    let mut state = SpectralState::from_rest(resources, clock, Epoch(0))?;
    let mut candidate = CandidateState::new(resources, clock, Epoch(0))?;
    let mut attempts = AttemptWorkspace::new_with_method(resources, Method::CoxMatthews)?;
    let tolerances = Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    };

    for index in 0..attempts_count {
        let region = Region::new(GLOBAL);
        let total_started = Instant::now();
        let integration_started = Instant::now();
        let result = attempts.try_advance(&state, &mut candidate, ticks, tolerances, &mut rhs)?;
        let integration_seconds = integration_started.elapsed().as_secs_f64();
        let accepted = result
            .accepted
            .ok_or(SolverError::ArithmeticResolutionLimited)?;
        commit_candidate(resources, &mut state, &mut candidate, accepted)?;
        let observer_started = Instant::now();
        let observer_result = observer.sample(&state)?;
        let observer_seconds = observer_started.elapsed().as_secs_f64();
        let total_seconds = total_started.elapsed().as_secs_f64();
        let allocations = region.change();
        let hit_miss = rhs.provider().hit_miss();
        let rhs_triplets = usize::from(w3) * result.rhs_calls * 3;
        let force_triplets = usize::from(w3) * hit_miss[1];
        let scalar_pressure_ffts = usize::from(w3) * result.rhs_calls;
        println!(
            "attempt={} start_state={} clock={} integration_seconds={integration_seconds:.9} observer_seconds={observer_seconds:.9} observer_force_seconds={:.9} observer_conservative_seconds={:.9} observer_transfer_measure_seconds={:.9} total_seconds={total_seconds:.9} rhs_calls={} cache_hit_miss={hit_miss:?} rhs_w3_triplets={rhs_triplets} force_w3_triplets={force_triplets} scalar_pressure_ffts={scalar_pressure_ffts} indicators={:?} work={:?} allocations={} deallocations={} reallocations={} bytes_allocated={} bytes_deallocated={} state_sha256={} balance={:?}",
            index + 1,
            if index == 0 { "rest" } else { "startup-ramp" },
            state.clock().elapsed(),
            observer_result.force_seconds,
            observer_result.conservative_seconds,
            observer_result.transfer_measure_seconds,
            result.rhs_calls,
            result.indicators,
            rhs.consumption(),
            allocations.allocations,
            allocations.deallocations,
            allocations.reallocations,
            allocations.bytes_allocated,
            allocations.bytes_deallocated,
            state_hash(&state)?,
            observer_result.balance,
        );
    }
    println!(
        "terminal=complete attempts={attempts_count} endpoint={} final_sha256={} archive_profile=unsupported arithmetic_qualification=missing later_state_representative=false",
        state.clock().elapsed(),
        state_hash(&state)?,
    );
    Ok(())
}

fn state_hash(state: &SpectralState) -> Result<String, SolverError> {
    let mut hash = Sha256::new();
    for axis in 0..3 {
        for value in state.component(axis)? {
            hash.update(value.re.to_bits().to_le_bytes());
            hash.update(value.im.to_bits().to_le_bytes());
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, SolverError> {
    value.parse().map_err(|_| SolverError::InvalidPayload)
}
