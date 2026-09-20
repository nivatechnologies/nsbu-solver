//! Staging of accepted attempts into capture artifacts; split out of the
//! run driver so the driver stays focused on the attempt transaction.
use crate::{
    artifact::StagedArtifact,
    balance::TimedBalance,
    error::{AnyResult, HarnessError},
    observer::ReducedObserver,
    records::{AttemptFacts, ObservationTiming},
    run_types::AcceptedStage,
    schedule,
};
#[cfg(not(feature = "n384-prep"))]
use crate::artifact;
#[cfg(not(capture_offline))]
use crate::{artifact::NodeRecord, GLOBAL};
use nsbu_solver::domain::SpectralState;
#[cfg(not(capture_offline))]
use {nsbu_solver::SolverError, stats_alloc::Region, std::time::Instant};
use std::path::Path;

pub fn stage_accepted(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    observer: &mut Option<ReducedObserver>,
    facts: AttemptFacts,
) -> AnyResult<AcceptedStage> {
    let observation = observe_proposal(proposal, observer)?;
    stage_observed(output, index, identity, proposal, facts, observation)
}

fn observe_proposal(
    proposal: &SpectralState,
    observer: &mut Option<ReducedObserver>,
) -> AnyResult<Option<(TimedBalance, ObservationTiming)>> {
    #[cfg(capture_offline)]
    {
        let _ = (proposal, observer);
        return Ok(None);
    }
    #[cfg(not(capture_offline))]
    {
        if !schedule::positive_node(proposal.clock().elapsed()) {
            return Ok(None);
        }
        let observer = observer.as_mut().ok_or(SolverError::InvalidPayload)?;
        let region = Region::new(GLOBAL);
        let started = Instant::now();
        let observed = observer.sample(proposal)?;
        let timing = ObservationTiming {
            total: started.elapsed().as_secs_f64(),
            force: observed.force_seconds,
            conservative: observed.conservative_seconds,
            transfer_measure: observed.transfer_measure_seconds,
        };
        crate::records::require_no_allocations(region.change())?;
        Ok(Some((
            TimedBalance {
                clock: proposal.clock(),
                sample: observed.balance,
            },
            timing,
        )))
    }
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
    let attempt_json = crate::records::committed(identity, timing, facts);
    let artifact = stage_artifact(output, index, identity, proposal, &attempt_json, observation)?;
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

#[cfg(all(feature = "n384-prep", not(capture_offline)))]
fn stage_artifact(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    attempt_json: &str,
    observation: Option<(TimedBalance, ObservationTiming)>,
) -> AnyResult<StagedArtifact> {
    let observation = match observation {
        Some((balance, timing)) => crate::step_artifact::Observation::Scheduled(NodeRecord {
            identity,
            balance: balance.sample,
            observer_seconds: timing.total,
            force_seconds: timing.force,
            conservative_seconds: timing.conservative,
            transfer_measure_seconds: timing.transfer_measure,
        }),
        None => crate::step_artifact::Observation::NotScheduled { identity },
    };
    crate::step_artifact::stage_step(output, index, proposal, observation, attempt_json)
        .map_err(HarnessError::from)
}

#[cfg(capture_offline)]
fn stage_artifact(
    output: &Path,
    index: usize,
    identity: &str,
    proposal: &SpectralState,
    attempt_json: &str,
    _observation: Option<(TimedBalance, ObservationTiming)>,
) -> AnyResult<StagedArtifact> {
    crate::step_artifact::stage_step(
        output,
        index,
        proposal,
        crate::step_artifact::Observation::Captured {
            identity,
            offline_observer_node: schedule::positive_node(proposal.clock().elapsed()),
        },
        attempt_json,
    )
    .map_err(HarnessError::from)
}
