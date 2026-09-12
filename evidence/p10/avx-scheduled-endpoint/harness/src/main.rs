mod artifact;
mod balance;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/cache.rs"]
mod cache;
mod command;
mod config;
mod error;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/observer.rs"]
mod observer;
mod owners;
#[cfg(all(test, feature = "n384-prep"))]
mod prep_tests;
mod publication;
mod records;
mod run_types;
mod schedule;
#[cfg(feature = "n384-prep")]
mod step_artifact;
mod timed_rhs;

use artifact::{NodeRecord, StagedArtifact};
use balance::TimedBalance;
use cache::CachedReducedForce;
use error::{AnyResult, HarnessError};
#[cfg(not(feature = "n384-prep"))]
use nsbu_solver::diagnostics::balances::BalanceSample;
use nsbu_solver::{
    domain::{ResourcePlan, SpectralState},
    integrators::{
        attempt::{AttemptResult, AttemptWorkspace},
        rhs::SpectralRhs,
        transaction::{prepare_commit, CandidateState, PreparedCommit},
    },
    SolverError,
};
use observer::ReducedObserver;
use publication::Frontiers;
use records::{AttemptFacts, ObservationTiming};
use run_types::{AcceptedFacts, AcceptedStage, StageMeta};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, fs, path::Path, time::Instant};
use timed_rhs::TimedRhs;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

struct RunOwners {
    resources: ResourcePlan,
    state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: TimedRhs<SpectralRhs<CachedReducedForce>>,
    observer: ReducedObserver,
    identity: String,
    balances: Vec<TimedBalance>,
    frontiers: Frontiers,
}

fn main() -> AnyResult<()> {
    dispatch(command::parse()?)
}

fn dispatch(command: command::Command) -> AnyResult<()> {
    match command {
        command::Command::Preflight => config::preflight().map(|_| ()).map_err(HarnessError::from),
        command::Command::Run(output) => start(&output),
    }
}

fn start(output: &Path) -> AnyResult<()> {
    let resources = prepare_run(output)?;
    let mut run = RunOwners::new(resources)?;
    run.execute_and_record(output)
}

fn prepare_run(output: &Path) -> AnyResult<ResourcePlan> {
    config::require_execution_ready()?;
    prepare_output(output)?;
    config::preflight().map_err(HarnessError::from)
}

fn prepare_output(output: &Path) -> AnyResult<()> {
    if output.exists() {
        return Err(SolverError::InvalidPayload.into());
    }
    fs::create_dir(output)?;
    Ok(())
}

impl RunOwners {
    fn new(resources: ResourcePlan) -> AnyResult<Self> {
        let execution = owners::execution(resources)?;
        let state_owners = owners::states(resources)?;
        let balances = balance::storage(state_owners.state.clock())?;
        Ok(Self {
            resources,
            state: state_owners.state,
            candidate: state_owners.candidate,
            attempts: state_owners.attempts,
            rhs: TimedRhs::new(execution.rhs),
            observer: execution.observer,
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

    fn execute_and_record(&mut self, output: &Path) -> AnyResult<()> {
        match self.execute(output) {
            Ok(()) => Ok(()),
            Err(error) => {
                let marker = records::incomplete(self.frontiers, &error.to_string());
                let _ = artifact::publish_status(output, "qualification-incomplete.json", &marker);
                Err(error)
            }
        }
    }

    #[cfg(feature = "n384-prep")]
    fn publish_rest(&mut self, output: &Path) -> AnyResult<()> {
        step_artifact::publish_rest(output, &self.identity)?;
        self.frontiers.durable_clock = 0;
        println!("published clock=0 state_payload=false balance=REST");
        Ok(())
    }

    #[cfg(not(feature = "n384-prep"))]
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
        self.rhs.reset_measurement();
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
        self.require_allocations(output, index, from, seconds, allocations)?;
        let accepted = self.acceptance(output, index, from, seconds, result)?;
        self.commit_accepted(output, accepted)
    }

    fn require_allocations(
        &mut self,
        output: &Path,
        index: usize,
        from: u128,
        seconds: f64,
        allocations: Stats,
    ) -> AnyResult<()> {
        match records::require_no_allocations(allocations) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.publish_numerical_failure(
                    output,
                    index,
                    from,
                    seconds,
                    &SolverError::ResourceLimit,
                )?;
                Err(error.into())
            }
        }
    }

    fn acceptance(
        &mut self,
        output: &Path,
        index: usize,
        from: u128,
        seconds: f64,
        result: AttemptResult,
    ) -> AnyResult<AcceptedFacts> {
        let Some(token) = result.accepted else {
            let json = records::rejected(
                &self.identity,
                index,
                from,
                seconds,
                self.rhs.measurement(),
                &result,
            );
            artifact::publish_attempt(output, index, &json)?;
            self.frontiers.durable_attempt = index;
            return Err(HarnessError::Rejected {
                ticks: result.ticks,
                ratios: result.indicators.ratios,
            });
        };
        let clock = from
            .checked_add(result.ticks)
            .ok_or(SolverError::SizeOverflow)?;
        let facts = AttemptFacts {
            index,
            clock,
            integration_seconds: seconds,
            ticks: result.ticks,
            rhs_calls: result.rhs_calls,
            ratios: result.indicators.ratios,
            hit_miss: self.rhs.inner().provider().hit_miss(),
            rhs_timing: self.rhs.measurement(),
        };
        Ok(AcceptedFacts { token, facts })
    }

    fn commit_accepted(&mut self, output: &Path, accepted: AcceptedFacts) -> AnyResult<()> {
        let facts = accepted.facts;
        let transaction = prepare_commit(
            self.resources,
            &mut self.state,
            &mut self.candidate,
            accepted.token,
        )?;
        let staged = stage_accepted(
            output,
            facts.index,
            &self.identity,
            transaction.proposal(),
            &mut self.observer,
            facts,
        );
        publish_prepared(
            &mut self.frontiers,
            &mut self.balances,
            facts,
            transaction,
            staged,
        )
    }

    fn publish_numerical_failure(
        &mut self,
        output: &Path,
        index: usize,
        from: u128,
        seconds: f64,
        error: &SolverError,
    ) -> AnyResult<()> {
        let json = records::numerical_error(
            &self.identity,
            index,
            from,
            seconds,
            self.rhs.measurement(),
            error,
        );
        artifact::publish_attempt(output, index, &json)?;
        self.frontiers.durable_attempt = index;
        Ok(())
    }

    fn finish(&self, output: &Path) -> AnyResult<()> {
        self.validate_finish()?;
        let integrals = balance::quadrature(&self.balances)?;
        let terminal = balance::terminal_json(
            &self.identity,
            self.state.clock(),
            self.balances.len(),
            integrals,
        );
        artifact::publish_status(output, "endpoint-complete.json", &terminal)?;
        println!("terminal endpoint_complete_qualification_pending clock=4096");
        Ok(())
    }

    fn validate_finish(&self) -> AnyResult<()> {
        if self.state.clock().elapsed() != schedule::ENDPOINT
            || self.frontiers.durable_clock != schedule::ENDPOINT
            || self.balances.len() != schedule::FINE.len()
        {
            return Err(SolverError::InvalidClock.into());
        }
        Ok(())
    }
}

fn publish_prepared(
    frontiers: &mut Frontiers,
    balances: &mut Vec<TimedBalance>,
    facts: AttemptFacts,
    transaction: PreparedCommit<'_>,
    staged: AnyResult<AcceptedStage>,
) -> AnyResult<()> {
    let meta = stage_meta(&staged);
    let published = commit_publication(frontiers, facts, transaction, staged)?;
    record_publication(balances, facts, meta, published)
}

fn stage_meta(staged: &AnyResult<AcceptedStage>) -> StageMeta {
    StageMeta {
        timing: staged.as_ref().ok().and_then(|stage| stage.timing),
        balance: staged.as_ref().ok().and_then(|stage| stage.balance),
    }
}

fn commit_publication(
    frontiers: &mut Frontiers,
    facts: AttemptFacts,
    transaction: PreparedCommit<'_>,
    staged: AnyResult<AcceptedStage>,
) -> AnyResult<artifact::PublishedArtifact> {
    publication::commit_staged(
        frontiers,
        facts.index,
        facts.clock,
        staged,
        || transaction.commit(),
        |stage| stage.artifact.publish(),
    )
}

fn record_publication(
    balances: &mut Vec<TimedBalance>,
    facts: AttemptFacts,
    meta: StageMeta,
    published: artifact::PublishedArtifact,
) -> AnyResult<()> {
    if let Some(balance) = meta.balance {
        balances.push(balance);
    }
    records::report(
        facts,
        meta.timing,
        published.kind,
        published.state_hash.as_deref(),
    );
    Ok(())
}

fn stage_accepted(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    observer: &mut ReducedObserver,
    facts: AttemptFacts,
) -> AnyResult<AcceptedStage> {
    let observation = observe_proposal(proposal, observer)?;
    stage_observed(output, index, identity, proposal, facts, observation)
}

fn observe_proposal(
    proposal: &SpectralState,
    observer: &mut ReducedObserver,
) -> AnyResult<Option<(TimedBalance, ObservationTiming)>> {
    if !schedule::positive_node(proposal.clock().elapsed()) {
        return Ok(None);
    }
    let region = Region::new(GLOBAL);
    let started = Instant::now();
    let observed = observer.sample(proposal)?;
    let timing = ObservationTiming {
        total: started.elapsed().as_secs_f64(),
        force: observed.force_seconds,
        conservative: observed.conservative_seconds,
        transfer_measure: observed.transfer_measure_seconds,
    };
    records::require_no_allocations(region.change())?;
    Ok(Some((
        TimedBalance {
            clock: proposal.clock(),
            sample: observed.balance,
        },
        timing,
    )))
}

fn stage_observed(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    facts: AttemptFacts,
    observation: Option<(TimedBalance, ObservationTiming)>,
) -> AnyResult<AcceptedStage> {
    let (balance, timing) = observation.unzip();
    let attempt_json = records::committed(identity, timing, facts);
    let artifact = stage_artifact(
        output,
        index,
        identity,
        proposal,
        &attempt_json,
        observation,
    )?;
    Ok(AcceptedStage {
        artifact,
        balance,
        timing,
    })
}

#[cfg(not(feature = "n384-prep"))]
fn stage_artifact(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    attempt_json: &str,
    observation: Option<(TimedBalance, ObservationTiming)>,
) -> AnyResult<StagedArtifact> {
    match observation {
        Some((balance, timing)) => artifact::stage_node(
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
            Some(attempt_json),
        )
        .map_err(HarnessError::from),
        None => artifact::stage_attempt(output, index, attempt_json).map_err(HarnessError::from),
    }
}

#[cfg(feature = "n384-prep")]
fn stage_artifact(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    attempt_json: &str,
    observation: Option<(TimedBalance, ObservationTiming)>,
) -> AnyResult<StagedArtifact> {
    let observation = match observation {
        Some((balance, timing)) => step_artifact::Observation::Scheduled(NodeRecord {
            identity,
            balance: balance.sample,
            observer_seconds: timing.total,
            force_seconds: timing.force,
            conservative_seconds: timing.conservative,
            transfer_measure_seconds: timing.transfer_measure,
        }),
        None => step_artifact::Observation::NotScheduled { identity },
    };
    step_artifact::stage_step(output, index, proposal, observation, attempt_json)
        .map_err(HarnessError::from)
}
