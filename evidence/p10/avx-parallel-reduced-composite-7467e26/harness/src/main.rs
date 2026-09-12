use nsbu_benchmarks::{provider::parallel_reduced::ParallelReducedV2Force, CASE_SHA256};
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
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, time::Instant};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const CAP: usize = 34_359_738_368;
const OVERHEAD: usize = 64 * 1024;

fn main() -> Result<(), SolverError> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [n, m, workers, ticks, attempts] = arguments.as_slice() else {
        return Err(SolverError::InvalidPayload);
    };
    execute(
        parse(n)?,
        parse(m)?,
        parse(workers)?,
        parse(ticks)?,
        parse(attempts)?,
    )
}

fn execute(
    n: usize,
    m: usize,
    workers: usize,
    ticks: u128,
    attempts_count: usize,
) -> Result<(), SolverError> {
    if attempts_count == 0 {
        return Err(SolverError::InvalidPayload);
    }
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([m; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let force_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, workers, backend)?;
    let rhs_bytes = SpectralRhs::<ParallelReducedV2Force>::reservation_with_fft_backend(
        domain,
        force_limits,
        backend,
    )?;
    let attempt_bytes = AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews)?;
    let resources = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: catalog_bytes,
            force: rhs_bytes,
            diagnostics: attempt_bytes,
            overhead: OVERHEAD,
        },
        CAP,
        Epoch(0),
    )?;
    let endpoint = ticks
        .checked_mul(attempts_count as u128)
        .ok_or(SolverError::SizeOverflow)?;
    if endpoint > 8192 || ticks % 2 != 0 {
        return Err(SolverError::InvalidStep);
    }
    println!(
        "preflight source={} case_sha256={CASE_SHA256} arithmetic=rustfft-6.4.1-avx-avx2-fma provider=parallel-reduced retained={n} sampled={m} workers={workers} ticks={ticks} attempts={attempts_count} catalog_bytes={catalog_bytes} force_limits={force_limits:?} rhs_bytes={rhs_bytes} attempt_bytes={attempt_bytes} overhead={OVERHEAD} total={} cap={CAP}",
        env!("RUN_SOURCE"),
        resources.total(),
    );

    let catalog = FftCatalog::new(backend, catalog_bytes)?;
    let force = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        workers,
        &catalog,
        force_limits.storage_bytes,
    )?;
    println!(
        "provider_identity={:?} fft_backend={:?}",
        force.identity(),
        force.fft_backend()
    );
    let mut rhs = SpectralRhs::new_with_catalog(domain, force, 0.3, &catalog, rhs_bytes)?;
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
        let started = Instant::now();
        let result = attempts.try_advance(&state, &mut candidate, ticks, tolerances, &mut rhs)?;
        let seconds = started.elapsed().as_secs_f64();
        let allocations = region.change();
        let accepted = result
            .accepted
            .ok_or(SolverError::ArithmeticResolutionLimited)?;
        commit_candidate(resources, &mut state, &mut candidate, accepted)?;
        println!(
            "attempt={} start_state={} clock={} seconds={seconds:.9} rhs_calls={} indicators={:?} work={:?} allocations={} deallocations={} reallocations={} bytes_allocated={} bytes_deallocated={} state_sha256={}",
            index + 1,
            if index == 0 { "rest" } else { "nonzero" },
            state.clock().elapsed(),
            result.rhs_calls,
            result.indicators,
            rhs.consumption(),
            allocations.allocations,
            allocations.deallocations,
            allocations.reallocations,
            allocations.bytes_allocated,
            allocations.bytes_deallocated,
            state_hash(&state)?,
        );
    }
    println!(
        "terminal=complete attempts={attempts_count} endpoint={} final_sha256={} archive_profile=unsupported arithmetic_qualification=missing",
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
