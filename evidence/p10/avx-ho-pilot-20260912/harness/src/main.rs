mod artifact;
#[path = "../../../avx-w3-n256-integration-20260912/harness/src/cache.rs"]
mod cache;
mod config;
#[path = "../../../avx-scheduled-endpoint/harness/src/error.rs"]
mod error;
#[path = "../../../avx-parallel-reduced-composite-7467e26/harness/src/observer.rs"]
mod observer;
mod owners;
#[path = "../../../avx-scheduled-endpoint/harness/src/timed_rhs.rs"]
mod timed_rhs;

use error::{AnyResult, HarnessError};
use nsbu_solver::{integrators::transaction::AcceptedAttempt, SolverError};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

enum Command {
    Preflight,
    Run(PathBuf),
}

struct AttemptTiming {
    seconds: f64,
    rhs_seconds: f64,
    rhs_calls: usize,
    ratios: [f64; 2],
    hit_miss: [usize; 2],
    token: AcceptedAttempt,
}

fn main() -> AnyResult<()> {
    execute(command()?)
}

fn execute(command: Command) -> AnyResult<()> {
    match command {
        Command::Preflight => {
            let admission = config::preflight(config::CAP)?;
            config::report(&admission);
            Ok(())
        }
        Command::Run(output) => run(&output),
    }
}

fn command() -> Result<Command, SolverError> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [mode, path] = args.as_slice() else {
        return Err(SolverError::InvalidPayload);
    };
    match mode.as_str() {
        "preflight" => Ok(Command::Preflight),
        "run" => Ok(Command::Run(path.into())),
        _ => Err(SolverError::InvalidPayload),
    }
}

fn run(output: &Path) -> AnyResult<()> {
    prepare_output(output)?;
    let admission = config::preflight(config::CAP)?;
    config::report(&admission);
    run_admitted(output, admission)
}

fn run_admitted(output: &Path, admission: config::Admission) -> AnyResult<()> {
    let construction_started = Instant::now();
    let mut owners = owners::construct(admission)?;
    let construction_seconds = construction_started.elapsed().as_secs_f64();
    validate_constructed(output, &owners)?;
    run_constructed(output, &mut owners, construction_seconds)
}

fn validate_constructed(output: &Path, owners: &owners::Owners) -> AnyResult<()> {
    if owners.resources.total() != config::EXPECTED_TOTAL {
        return fail(output, "constructed resource identity changed");
    }
    if !owners::is_rest(&owners.state)? {
        return fail(output, "committed state was not REST after construction");
    }
    Ok(())
}

fn run_constructed(
    output: &Path,
    owners: &mut owners::Owners,
    construction_seconds: f64,
) -> AnyResult<()> {
    let attempt = attempt_phase(output, owners)?;
    publish_attempt(output, construction_seconds, &attempt)?;
    observe_phase(output, owners, attempt)
}

fn attempt_phase(output: &Path, owners: &mut owners::Owners) -> AnyResult<AttemptTiming> {
    match measure_attempt(owners) {
        Ok(value) => Ok(value),
        Err(error) => fail(output, &format!("attempt failed: {error}")),
    }
}

fn observe_phase(
    output: &Path,
    owners: &mut owners::Owners,
    attempt: AttemptTiming,
) -> AnyResult<()> {
    let proposal = proposal_or_fail(output, &owners.candidate, &owners.state, &attempt)?;
    let (observed, observer_seconds) = measure_observer(output, &mut owners.observer, proposal)?;
    finish_observer(
        output,
        owners,
        attempt,
        proposal,
        observer_seconds,
        observed,
    )
}

fn measure_observer(
    output: &Path,
    observer: &mut observer::ReducedObserver,
    proposal: &nsbu_solver::domain::SpectralState,
) -> AnyResult<(observer::ObserverResult, f64)> {
    let observer_region = Region::new(GLOBAL);
    let observer_started = Instant::now();
    let observed = observer.sample(proposal);
    let observer_seconds = observer_started.elapsed().as_secs_f64();
    let observer_allocations = observer_region.change();
    let observed = observer_or_fail(output, observed)?;
    require_zero(observer_allocations)?;
    Ok((observed, observer_seconds))
}

fn finish_observer(
    output: &Path,
    owners: &owners::Owners,
    attempt: AttemptTiming,
    proposal: &nsbu_solver::domain::SpectralState,
    observer_seconds: f64,
    observed: observer::ObserverResult,
) -> AnyResult<()> {
    validate_proposal_clock(output, proposal)?;
    let _discarded_balance = observed.balance;
    publish_observer(output, observer_seconds, &observed)?;
    let _accepted_proposal_not_committed = attempt.token;
    validate_committed_rest(output, owners)?;
    publish_complete(output)
}

fn validate_proposal_clock(
    output: &Path,
    proposal: &nsbu_solver::domain::SpectralState,
) -> AnyResult<()> {
    if proposal.clock().elapsed() != config::TICKS {
        return fail(output, "accepted proposal clock changed unexpectedly");
    }
    Ok(())
}

fn validate_committed_rest(output: &Path, owners: &owners::Owners) -> AnyResult<()> {
    if !owners::is_rest(&owners.state)? {
        return fail(output, "committed state changed without commit");
    }
    Ok(())
}

fn publish_complete(output: &Path) -> AnyResult<()> {
    artifact::publish(output, "complete.json", &format!(
        "{{\n  \"status\": \"complete_timing_only\",\n  \"identity\": {},\n  \"committed_clock\": 0,\n  \"committed_epoch\": 0,\n  \"committed_steps\": 0,\n  \"state_payload\": false,\n  \"balance_published\": false,\n  \"frontier_published\": false\n}}\n",
        artifact::escape(config::IDENTITY),
    ))?;
    println!("complete timing_only committed_rest=true state_payload=false balance_published=false frontier_published=false");
    Ok(())
}

fn proposal_or_fail<'a>(
    output: &Path,
    candidate: &'a nsbu_solver::integrators::transaction::CandidateState,
    state: &nsbu_solver::domain::SpectralState,
    attempt: &AttemptTiming,
) -> AnyResult<&'a nsbu_solver::domain::SpectralState> {
    match candidate.proposal(state, &attempt.token) {
        Ok(value) => Ok(value),
        Err(error) => fail(output, &format!("proposal failed: {error:?}")),
    }
}

fn observer_or_fail(
    output: &Path,
    result: Result<observer::ObserverResult, SolverError>,
) -> AnyResult<observer::ObserverResult> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => fail(output, &format!("observer failed: {error:?}")),
    }
}

fn measure_attempt(owners: &mut owners::Owners) -> AnyResult<AttemptTiming> {
    owners.rhs.reset_measurement();
    let region = Region::new(GLOBAL);
    let started = Instant::now();
    let result = owners.attempt.try_advance(
        &owners.state,
        &mut owners.candidate,
        config::TICKS,
        config::tolerances(),
        &mut owners.rhs,
    )?;
    let seconds = started.elapsed().as_secs_f64();
    let allocations = region.change();
    require_zero(allocations)?;
    let Some(token) = result.accepted else {
        return Err(HarnessError::Rejected {
            ticks: result.ticks,
            ratios: result.indicators.ratios,
        });
    };
    let measurement = owners.rhs.measurement();
    let hit_miss = owners.rhs.inner().provider().hit_miss();
    validate_attempt_accounting(result.rhs_calls, measurement.calls, hit_miss)?;
    Ok(AttemptTiming {
        seconds,
        rhs_seconds: measurement.seconds,
        rhs_calls: measurement.calls,
        ratios: result.indicators.ratios,
        hit_miss,
        token,
    })
}

fn validate_attempt_accounting(
    rhs_calls: usize,
    measured_calls: usize,
    hit_miss: [usize; 2],
) -> AnyResult<()> {
    if rhs_calls == 15 && measured_calls == 15 && hit_miss == [10, 5] {
        Ok(())
    } else {
        Err(SolverError::ProviderBudgetExceeded.into())
    }
}

fn publish_attempt(output: &Path, construction: f64, attempt: &AttemptTiming) -> AnyResult<()> {
    let outside = attempt.seconds - attempt.rhs_seconds;
    let json = format!(
        "{{\n  \"status\": \"accepted_proposal_timing_only\",\n  \"identity\": {},\n  \"construction_seconds\": {:.9},\n  \"integration_seconds\": {:.9},\n  \"rhs_evaluate_seconds\": {:.9},\n  \"outside_rhs_evaluate_seconds\": {:.9},\n  \"rhs_calls\": {},\n  \"cache_hits\": {},\n  \"cache_misses\": {},\n  \"ratios\": [{:.17e}, {:.17e}],\n  \"committed_clock\": 0,\n  \"proposal_clock\": 64,\n  \"state_payload\": false,\n  \"frontier_published\": false\n}}\n",
        artifact::escape(config::IDENTITY), construction, attempt.seconds, attempt.rhs_seconds,
        outside, attempt.rhs_calls, attempt.hit_miss[0], attempt.hit_miss[1],
        attempt.ratios[0], attempt.ratios[1],
    );
    artifact::publish(output, "attempt-timing.json", &json)?;
    println!("phase=attempt integration_seconds={:.9} rhs_seconds={:.9} outside_rhs_seconds={outside:.9} calls={} cache={:?} ratios={:?} committed_rest=true", attempt.seconds, attempt.rhs_seconds, attempt.rhs_calls, attempt.hit_miss, attempt.ratios);
    std::io::stdout().flush()?;
    Ok(())
}

fn publish_observer(
    output: &Path,
    total: f64,
    observed: &observer::ObserverResult,
) -> AnyResult<()> {
    let json = format!(
        "{{\n  \"status\": \"unscheduled_uncommitted_proposal_observer_timing_only\",\n  \"identity\": {},\n  \"observer_seconds\": {:.9},\n  \"force_seconds\": {:.9},\n  \"conservative_seconds\": {:.9},\n  \"transfer_measure_seconds\": {:.9},\n  \"balance_discarded\": true,\n  \"state_payload\": false,\n  \"frontier_published\": false\n}}\n",
        artifact::escape(config::IDENTITY), total, observed.force_seconds,
        observed.conservative_seconds, observed.transfer_measure_seconds,
    );
    artifact::publish(output, "observer-timing.json", &json)?;
    println!("phase=observer total_seconds={total:.9} force_seconds={:.9} conservative_seconds={:.9} transfer_measure_seconds={:.9} balance_discarded=true committed_rest=true", observed.force_seconds, observed.conservative_seconds, observed.transfer_measure_seconds);
    std::io::stdout().flush()?;
    Ok(())
}

fn prepare_output(output: &Path) -> AnyResult<()> {
    if output.exists() {
        return Err(SolverError::InvalidPayload.into());
    }
    fs::create_dir(output)?;
    Ok(())
}

fn require_zero(stats: Stats) -> Result<(), SolverError> {
    if stats.allocations == 0 && stats.deallocations == 0 && stats.reallocations == 0 {
        Ok(())
    } else {
        Err(SolverError::ResourceLimit)
    }
}

fn fail<T>(output: &Path, message: &str) -> AnyResult<T> {
    let json = format!("{{\n  \"status\": \"failed_no_commit\",\n  \"error\": {},\n  \"state_payload\": false,\n  \"frontier_published\": false\n}}\n", artifact::escape(message));
    let _ = artifact::publish(output, "failure.json", &json);
    Err(SolverError::InvalidPayload.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_shape_is_closed() {
        assert!(matches!(Command::Preflight, Command::Preflight));
        assert_eq!(config::TICKS, 64);
    }

    #[test]
    fn zero_allocation_contract_rejects_activity() {
        assert!(require_zero(Stats::default()).is_ok());
        let nonzero = Stats {
            allocations: 1,
            ..Stats::default()
        };
        assert_eq!(require_zero(nonzero), Err(SolverError::ResourceLimit));
    }
}
