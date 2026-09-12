mod artifact;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/cache.rs"]
mod cache;
mod config;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/observer.rs"]
mod observer;
mod publication;
mod records;
mod schedule;

use artifact::{NodeRecord, StagedArtifact};
use cache::CachedReducedForce;
use config::{ADVECTIVE_LIMIT, M, WORKERS};
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory, quadrature::BalanceIntegral},
    domain::{Epoch, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::{AttemptResult, AttemptWorkspace},
        method::Method,
        rhs::SpectralRhs,
        transaction::{prepare_commit, CandidateState},
    },
    spectral::{FftBackend, FftCatalog},
    SolverError,
};
use observer::ReducedObserver;
use publication::Frontiers;
use records::{AttemptFacts, ObservationTiming};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Instant,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type AnyResult<T> = Result<T, HarnessError>;

#[derive(Debug)]
enum HarnessError {
    Numerical(SolverError),
    Io(io::Error),
    Config(&'static str),
    Rejected { ticks: u128, ratios: [f64; 2] },
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numerical(error) => write!(formatter, "numerical:{error:?}"),
            Self::Io(error) => write!(formatter, "io:{error}"),
            Self::Config(error) => formatter.write_str(error),
            Self::Rejected { ticks, ratios } => {
                write!(
                    formatter,
                    "attempt_rejected:ticks={ticks}:ratios={ratios:?}"
                )
            }
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

struct AcceptedStage {
    artifact: StagedArtifact,
    balance: Option<TimedBalance>,
    timing: Option<ObservationTiming>,
}

struct RunOwners {
    resources: ResourcePlan,
    state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: SpectralRhs<CachedReducedForce>,
    observer: ReducedObserver,
    identity: String,
    balances: Vec<TimedBalance>,
    frontiers: Frontiers,
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
        "preflight" => config::preflight().map(|_| ()).map_err(HarnessError::from),
        "run" => start(&output),
        _ => Err(HarnessError::Config("unknown mode")),
    }
}

fn start(output: &Path) -> AnyResult<()> {
    if output.exists() {
        return Err(HarnessError::Config("output path already exists"));
    }
    fs::create_dir(output)?;
    let resources = config::preflight()?;
    let mut run = RunOwners::new(resources)?;
    if let Err(error) = run.execute(output) {
        let marker = run.incomplete_json(&error);
        let _ = artifact::publish_status(output, "qualification-incomplete.json", &marker);
        return Err(error);
    }
    Ok(())
}

impl RunOwners {
    fn new(resources: ResourcePlan) -> AnyResult<Self> {
        let domain = config::domain()?;
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
        let rhs =
            SpectralRhs::new_with_catalog(domain, force, ADVECTIVE_LIMIT, &catalog, rhs_bytes)?;
        let observer_bytes =
            ReducedObserver::preflight(domain, observer_samples, WORKERS, backend)?;
        let observer =
            ReducedObserver::new(domain, observer_samples, WORKERS, &catalog, observer_bytes)?;
        let initial_clock = TickClock::from_rest(-20, 8192)?;
        let state = SpectralState::from_rest(resources, initial_clock, Epoch(0))?;
        let candidate = CandidateState::new(resources, initial_clock, Epoch(0))?;
        let attempts = AttemptWorkspace::new_with_method(resources, Method::CoxMatthews)?;
        let mut balances = Vec::new();
        balances
            .try_reserve_exact(schedule::FINE.len())
            .map_err(|_| SolverError::AllocationFailed)?;
        balances.push(TimedBalance {
            clock: state.clock(),
            sample: BalanceSample::REST,
        });
        Ok(Self {
            resources,
            state,
            candidate,
            attempts,
            rhs,
            observer,
            identity: config::identity(),
            balances,
            frontiers: Frontiers::default(),
        })
    }

    fn execute(&mut self, output: &Path) -> AnyResult<()> {
        self.publish_rest(output)?;
        for index in 1..=schedule::MAXIMUM_ATTEMPTS {
            self.attempt(output, index)?;
        }
        self.finish(output)
    }

    fn publish_rest(&mut self, output: &Path) -> AnyResult<()> {
        let rest_hash = artifact::publish_node(
            output,
            &self.state,
            NodeRecord {
                identity: &self.identity,
                balance: BalanceSample::REST,
                observer_seconds: 0.0,
                force_seconds: 0.0,
                conservative_seconds: 0.0,
                transfer_measure_seconds: 0.0,
            },
        )?;
        self.frontiers.durable_clock = 0;
        println!("published clock=0 state_sha256={rest_hash} balance=REST");
        Ok(())
    }

    fn attempt(&mut self, output: &Path, index: usize) -> AnyResult<()> {
        self.frontiers.attempted = index;
        let from = self.state.clock().elapsed();
        let integration_region = Region::new(GLOBAL);
        let started = Instant::now();
        let result = self.attempts.try_advance(
            &self.state,
            &mut self.candidate,
            schedule::STEP,
            config::tolerances(),
            &mut self.rhs,
        );
        let seconds = started.elapsed().as_secs_f64();
        let allocations = integration_region.change();
        match result {
            Ok(result) => self.finish_attempt(output, index, from, seconds, allocations, result),
            Err(error) => {
                self.publish_numerical_failure(output, index, from, seconds, &error)?;
                Err(error.into())
            }
        }
    }

    fn finish_attempt(
        &mut self,
        output: &Path,
        index: usize,
        from: u128,
        seconds: f64,
        allocations: Stats,
        result: AttemptResult,
    ) -> AnyResult<()> {
        if let Err(error) = require_no_allocations(allocations) {
            self.publish_numerical_failure(
                output,
                index,
                from,
                seconds,
                &SolverError::ResourceLimit,
            )?;
            return Err(error);
        }
        let ticks = result.ticks;
        let rhs_calls = result.rhs_calls;
        let ratios = result.indicators.ratios;
        let Some(accepted) = result.accepted else {
            let json = records::rejected(&self.identity, index, from, seconds, &result);
            artifact::publish_attempt(output, index, &json)?;
            self.frontiers.durable_attempt = index;
            return Err(HarnessError::Rejected {
                ticks: result.ticks,
                ratios: result.indicators.ratios,
            });
        };
        let clock = from.checked_add(ticks).ok_or(SolverError::SizeOverflow)?;
        let facts = AttemptFacts {
            index,
            clock,
            integration_seconds: seconds,
            ticks,
            rhs_calls,
            ratios,
            hit_miss: self.rhs.provider().hit_miss(),
        };
        let transaction = prepare_commit(
            self.resources,
            &mut self.state,
            &mut self.candidate,
            accepted,
        )?;
        let staged = stage_accepted(
            output,
            index,
            &self.identity,
            transaction.proposal(),
            &mut self.observer,
            facts,
        );
        let stage_timing = staged.as_ref().ok().and_then(|stage| stage.timing);
        let stage_balance = staged.as_ref().ok().and_then(|stage| stage.balance);
        let published = publication::commit_staged(
            &mut self.frontiers,
            index,
            clock,
            staged,
            || transaction.commit(),
            |stage| stage.artifact.publish(),
        )?;
        if let Some(balance) = stage_balance {
            self.balances.push(balance);
        }
        records::report(
            facts,
            stage_timing,
            published.kind,
            published.state_hash.as_deref(),
        );
        Ok(())
    }

    fn publish_numerical_failure(
        &mut self,
        output: &Path,
        index: usize,
        from: u128,
        seconds: f64,
        error: &SolverError,
    ) -> AnyResult<()> {
        let json = records::numerical_error(&self.identity, index, from, seconds, error);
        artifact::publish_attempt(output, index, &json)?;
        self.frontiers.durable_attempt = index;
        Ok(())
    }

    fn incomplete_json(&self, error: &HarnessError) -> String {
        let provisional = self
            .frontiers
            .provisional_clock
            .map_or_else(|| "null".to_owned(), |value| value.to_string());
        format!(
            concat!(
                "{{\n  \"status\": \"qualification_incomplete\",\n",
                "  \"error\": {},\n  \"attempted_frontier\": {},\n",
                "  \"in_memory_clock\": {},\n  \"durable_clock\": {},\n",
                "  \"durable_attempt_frontier\": {},\n  \"provisional_clock\": {}\n}}\n"
            ),
            artifact::json_string(&error.to_string()),
            self.frontiers.attempted,
            self.frontiers.in_memory_clock,
            self.frontiers.durable_clock,
            self.frontiers.durable_attempt,
            provisional,
        )
    }

    fn finish(&self, output: &Path) -> AnyResult<()> {
        if self.state.clock().elapsed() != schedule::ENDPOINT
            || self.frontiers.durable_clock != schedule::ENDPOINT
            || self.balances.len() != schedule::FINE.len()
        {
            return Err(SolverError::InvalidClock.into());
        }
        let [coarse, middle, fine] = quadrature(&self.balances)?;
        let terminal = format!(
            concat!(
                "{{\n  \"status\": \"endpoint_complete_qualification_pending\",\n",
                "  \"identity\": {},\n  \"clock\": {},\n  \"samples\": {},\n",
                "  \"quadrature_sufficiency\": \"not_assessed\",\n",
                "  \"coarse\": {},\n  \"middle\": {},\n  \"fine\": {}\n}}\n"
            ),
            artifact::json_string(&self.identity),
            self.state.clock().elapsed(),
            self.balances.len(),
            artifact::json_string(&format!("{coarse:?}")),
            artifact::json_string(&format!("{middle:?}")),
            artifact::json_string(&format!("{fine:?}")),
        );
        artifact::publish_status(output, "endpoint-complete.json", &terminal)?;
        println!("terminal endpoint_complete_qualification_pending clock=4096");
        Ok(())
    }
}

fn stage_accepted(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    observer: &mut ReducedObserver,
    facts: AttemptFacts,
) -> AnyResult<AcceptedStage> {
    let scheduled = schedule::positive_node(proposal.clock().elapsed());
    let (balance, timing) = if scheduled {
        let region = Region::new(GLOBAL);
        let started = Instant::now();
        let observed = observer.sample(proposal)?;
        let timing = ObservationTiming {
            total: started.elapsed().as_secs_f64(),
            force: observed.force_seconds,
            conservative: observed.conservative_seconds,
            transfer_measure: observed.transfer_measure_seconds,
        };
        require_no_allocations(region.change())?;
        (
            Some(TimedBalance {
                clock: proposal.clock(),
                sample: observed.balance,
            }),
            Some(timing),
        )
    } else {
        (None, None)
    };
    let attempt_json = records::committed(identity, timing, facts);
    let artifact = if let (Some(balance), Some(timing)) = (balance, timing) {
        artifact::stage_node(
            output,
            proposal,
            NodeRecord {
                identity,
                balance: balance.sample,
                observer_seconds: timing.total,
                force_seconds: timing.force,
                conservative_seconds: timing.conservative,
                transfer_measure_seconds: timing.transfer_measure,
            },
            Some(&attempt_json),
        )?
    } else {
        artifact::stage_attempt(output, index, &attempt_json)?
    };
    Ok(AcceptedStage {
        artifact,
        balance,
        timing,
    })
}

fn require_no_allocations(allocations: Stats) -> AnyResult<()> {
    if allocations.allocations != 0
        || allocations.deallocations != 0
        || allocations.reallocations != 0
    {
        Err(SolverError::ResourceLimit.into())
    } else {
        Ok(())
    }
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
