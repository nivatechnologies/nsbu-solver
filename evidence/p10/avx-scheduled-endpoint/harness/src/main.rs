mod artifact;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/cache.rs"]
mod cache;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/observer.rs"]
mod observer;
mod schedule;

use artifact::NodeRecord;
use cache::CachedReducedForce;
use nsbu_benchmarks::CASE_SHA256;
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory, quadrature::BalanceIntegral},
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
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Instant,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const N: usize = 192;
const M: usize = 384;
const WORKERS: usize = 32;
const CAP: usize = 103_079_215_104;
const ADVECTIVE_LIMIT: f64 = 0.45;
const HISTORY_BYTES: usize = schedule::MAXIMUM_ATTEMPTS * 4096;
const OVERHEAD: usize = artifact::BUFFER_BYTES + HISTORY_BYTES + 64 * 1024;

type AnyResult<T> = Result<T, HarnessError>;

#[derive(Debug)]
enum HarnessError {
    Numerical(SolverError),
    Io(io::Error),
    Config(&'static str),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numerical(error) => write!(formatter, "numerical:{error:?}"),
            Self::Io(error) => write!(formatter, "io:{error}"),
            Self::Config(error) => formatter.write_str(error),
        }
    }
}

impl Error for HarnessError {}

impl From<SolverError> for HarnessError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}

impl From<io::Error> for HarnessError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Copy)]
struct TimedBalance {
    clock: TickClock,
    sample: BalanceSample,
}

fn main() -> AnyResult<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [mode, path] = arguments.as_slice() else {
        return Err(HarnessError::Config(
            "usage: p10-avx-scheduled-endpoint preflight|run OUTPUT",
        ));
    };
    let output = PathBuf::from(path);
    match mode.as_str() {
        "preflight" => preflight().map(|_| ()).map_err(HarnessError::from),
        "run" => {
            if output.exists() {
                return Err(HarnessError::Config("output path already exists"));
            }
            fs::create_dir(&output)?;
            match execute(&output) {
                Ok(()) => Ok(()),
                Err(error) => {
                    let marker = format!(
                        "{{\n  \"status\": \"qualification_incomplete\",\n  \"error\": \"{error}\"\n}}\n"
                    );
                    let _ =
                        artifact::publish_status(&output, "qualification-incomplete.json", &marker);
                    Err(error)
                }
            }
        }
        _ => Err(HarnessError::Config("unknown mode")),
    }
}

fn preflight() -> Result<ResourcePlan, SolverError> {
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
    let snapshot_bytes = domain
        .layout()
        .half_len()
        .checked_mul(3 * 16)
        .ok_or(SolverError::SizeOverflow)?;
    let disk_bytes = artifact::disk_preflight(snapshot_bytes, schedule::FINE.len())?;
    let integration_work = force_limits
        .work_units
        .checked_mul(12)
        .and_then(|n| n.checked_mul(schedule::MAXIMUM_ATTEMPTS))
        .ok_or(SolverError::SizeOverflow)?;
    let diagnostic =
        nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(domain)?;
    let observer_force = nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        observer_samples,
        WORKERS,
        backend,
    )?;
    let observer_work = observer_force
        .work_units
        .checked_mul(8)
        .ok_or(SolverError::SizeOverflow)?;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} schema=p10-avx-scheduled-endpoint-v1 backend=rustfft-6.4.1-avx-avx2-fma provider=parallel-reduced-attempt-cache retained={N} sampled={M} workers={WORKERS} method=cox-matthews step={} maximum_attempts={} endpoint={} advective_limit={ADVECTIVE_LIMIT} observer_nodes={:?} observer_sampled={} nested_middle={:?} nested_coarse={:?} catalog_bytes={catalog_bytes} rhs_bytes={rhs_bytes} attempt_bytes={attempt_bytes} observer_bytes={observer_bytes} overhead={OVERHEAD} total={} cap={CAP} disk_preflight_bytes={disk_bytes} disk_cap_bytes={} integration_work_bound={integration_work} observer_work_bound={observer_work} archive_profile=unsupported qualification=experimental",
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
    Ok(resources)
}

fn execute(output: &Path) -> AnyResult<()> {
    let resources = preflight()?;
    let domain = domain()?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog = FftCatalog::new(backend, resources.classes()[4])?;
    let samples = Layout::new([M; 3])?;
    let observer_samples = Layout::new([2 * M; 3])?;
    let force_limits = CachedReducedForce::preflight(domain, samples, WORKERS, backend)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        WORKERS,
        &catalog,
        force_limits.storage_bytes,
    )?;
    let rhs_bytes = SpectralRhs::<CachedReducedForce>::reservation_with_catalog(
        domain,
        force_limits,
        &catalog,
    )?;
    let mut rhs =
        SpectralRhs::new_with_catalog(domain, force, ADVECTIVE_LIMIT, &catalog, rhs_bytes)?;
    let observer_bytes = ReducedObserver::preflight(domain, observer_samples, WORKERS, backend)?;
    let mut observer =
        ReducedObserver::new(domain, observer_samples, WORKERS, &catalog, observer_bytes)?;
    let initial_clock = TickClock::from_rest(-20, 8192)?;
    let mut state = SpectralState::from_rest(resources, initial_clock, Epoch(0))?;
    let mut candidate = CandidateState::new(resources, initial_clock, Epoch(0))?;
    let mut attempts = AttemptWorkspace::new_with_method(resources, Method::CoxMatthews)?;
    let tolerances = Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    };
    let identity = identity();
    let mut balances = Vec::new();
    balances
        .try_reserve_exact(schedule::FINE.len())
        .map_err(|_| SolverError::AllocationFailed)?;
    balances.push(TimedBalance {
        clock: state.clock(),
        sample: BalanceSample::REST,
    });
    let rest_hash = artifact::publish_node(
        output,
        &state,
        NodeRecord {
            identity: &identity,
            balance: BalanceSample::REST,
            observer_seconds: 0.0,
            force_seconds: 0.0,
            conservative_seconds: 0.0,
            transfer_measure_seconds: 0.0,
        },
    )?;
    println!("published clock=0 state_sha256={rest_hash} balance=REST");

    for index in 1..=schedule::MAXIMUM_ATTEMPTS {
        let integration_region = Region::new(GLOBAL);
        let started = Instant::now();
        let result =
            attempts.try_advance(&state, &mut candidate, schedule::STEP, tolerances, &mut rhs)?;
        let integration_seconds = started.elapsed().as_secs_f64();
        let integration_allocations = integration_region.change();
        let accepted = result
            .accepted
            .ok_or(SolverError::ArithmeticResolutionLimited)?;
        commit_candidate(resources, &mut state, &mut candidate, accepted)?;
        let scheduled = schedule::positive_node(state.clock().elapsed());
        let mut observer_seconds = None;
        if scheduled {
            let observer_region = Region::new(GLOBAL);
            let observer_started = Instant::now();
            let observed = observer.sample(&state)?;
            let elapsed = observer_started.elapsed().as_secs_f64();
            let observer_allocations = observer_region.change();
            if observer_allocations.allocations != 0
                || observer_allocations.deallocations != 0
                || observer_allocations.reallocations != 0
            {
                return Err(SolverError::ResourceLimit.into());
            }
            balances.push(TimedBalance {
                clock: state.clock(),
                sample: observed.balance,
            });
            let state_hash = artifact::publish_node(
                output,
                &state,
                NodeRecord {
                    identity: &identity,
                    balance: observed.balance,
                    observer_seconds: elapsed,
                    force_seconds: observed.force_seconds,
                    conservative_seconds: observed.conservative_seconds,
                    transfer_measure_seconds: observed.transfer_measure_seconds,
                },
            )?;
            observer_seconds = Some(elapsed);
            println!(
                "published clock={} state_sha256={state_hash} observer_seconds={elapsed:.9} observer_force_seconds={:.9} observer_conservative_seconds={:.9} observer_transfer_measure_seconds={:.9}",
                state.clock().elapsed(),
                observed.force_seconds,
                observed.conservative_seconds,
                observed.transfer_measure_seconds,
            );
        }
        if integration_allocations.allocations != 0
            || integration_allocations.deallocations != 0
            || integration_allocations.reallocations != 0
        {
            return Err(SolverError::ResourceLimit.into());
        }
        let attempt_json = format!(
            concat!(
                "{{\n  \"schema\": \"p10-avx-scheduled-attempt-v1\",\n",
                "  \"identity\": \"{}\",\n  \"attempt\": {},\n  \"clock\": {},\n",
                "  \"outcome\": \"committed\",\n  \"rhs_calls\": {},\n",
                "  \"cache_hits\": {},\n  \"cache_misses\": {},\n",
                "  \"integration_seconds\": {:.9},\n  \"observer_seconds\": {},\n",
                "  \"error_ratio_l2\": {:.17e},\n  \"error_ratio_h1\": {:.17e},\n",
                "  \"steady_allocations\": 0\n}}\n"
            ),
            identity,
            index,
            state.clock().elapsed(),
            result.rhs_calls,
            rhs.provider().hit_miss()[0],
            rhs.provider().hit_miss()[1],
            integration_seconds,
            observer_seconds.map_or_else(|| "null".to_owned(), |value| format!("{value:.9}")),
            result.indicators.ratios[0],
            result.indicators.ratios[1],
        );
        artifact::publish_attempt(output, index, &attempt_json)?;
        println!(
            "attempt={index} clock={} integration_seconds={integration_seconds:.9} cache_hit_miss={:?} observer_seconds={observer_seconds:?} ratios={:?} steady_allocations=0",
            state.clock().elapsed(),
            rhs.provider().hit_miss(),
            result.indicators.ratios,
        );
    }
    if state.clock().elapsed() != schedule::ENDPOINT || balances.len() != schedule::FINE.len() {
        return Err(SolverError::InvalidClock.into());
    }
    let [coarse, middle, fine] = quadrature(&balances)?;
    let terminal = format!(
        concat!(
            "{{\n  \"status\": \"endpoint_complete_qualification_pending\",\n",
            "  \"identity\": \"{}\",\n  \"clock\": {},\n  \"samples\": {},\n",
            "  \"quadrature_sufficiency\": \"not_assessed\",\n",
            "  \"coarse\": \"{:?}\",\n  \"middle\": \"{:?}\",\n  \"fine\": \"{:?}\"\n}}\n"
        ),
        identity,
        state.clock().elapsed(),
        balances.len(),
        coarse,
        middle,
        fine,
    );
    artifact::publish_status(output, "endpoint-complete.json", &terminal)?;
    println!("terminal endpoint_complete_qualification_pending clock=4096");
    Ok(())
}

fn quadrature(samples: &[TimedBalance]) -> Result<[BalanceIntegral; 3], SolverError> {
    Ok([
        history(samples, &schedule::COARSE)?,
        history(samples, &schedule::MIDDLE)?,
        history(samples, &schedule::FINE)?,
    ])
}

fn history(samples: &[TimedBalance], clocks: &[u128]) -> Result<BalanceIntegral, SolverError> {
    let initial = samples
        .iter()
        .find(|sample| sample.clock.elapsed() == clocks[0])
        .ok_or(SolverError::InvalidClock)?;
    let mut history = BalanceHistory::new(initial.clock, initial.sample, clocks.len())?;
    for clock in &clocks[1..] {
        let sample = samples
            .iter()
            .find(|sample| sample.clock.elapsed() == *clock)
            .ok_or(SolverError::InvalidClock)?;
        history = history.with_sample(sample.clock, sample.sample)?;
    }
    history.integral()
}

fn domain() -> Result<Domain, SolverError> {
    Domain::new([N; 3], [1.0; 3], 1.0)
}

fn identity() -> String {
    format!(
        "source={};case={CASE_SHA256};backend=rustfft-6.4.1-avx-avx2-fma;provider=parallel-reduced-attempt-cache;n={N};m={M};workers={WORKERS};method=cox-matthews;step={};endpoint={};advective_limit={ADVECTIVE_LIMIT};cap={CAP};schema=p10-avx-scheduled-endpoint-v1;resume=unsupported",
        env!("RUN_SOURCE"),
        schedule::STEP,
        schedule::ENDPOINT,
    )
}
