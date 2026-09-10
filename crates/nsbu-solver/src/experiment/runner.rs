//! One recorded fixed-step attempt, with private diagnostic proposals before a physical commit.
use super::{control::Outcome, log::RunHistory, observer::BalanceObserver};
use crate::{
    domain::{SpectralState, TickClock},
    integrators::{
        attempt::{AttemptResult, AttemptWorkspace},
        indicator::Indicators,
        kernel::RightHandSide,
        transaction::{prepare_commit, CandidateState},
    },
    SolverError,
};

/// Perform one bounded attempt and retain its outcome, including integration/diagnostic failures.
/// An Err is preflight failure with no attempted physical work. An Ok may contain a refusal;
/// inspect Outcome and the controller's terminal status. No result qualifies a PDE window.
pub fn recorded_step(
    state: &mut SpectralState,
    candidate: &mut CandidateState,
    workspace: &mut AttemptWorkspace,
    rhs: &mut dyn RightHandSide,
    observer: &mut dyn BalanceObserver,
    history: &mut RunHistory,
) -> Result<Outcome, SolverError> {
    let endpoint = admit(state, workspace, observer, history)?;
    let configuration = history.controller().configuration();
    match workspace.try_advance(
        state,
        candidate,
        configuration.limits.step_ticks,
        configuration.tolerances,
        rhs,
    ) {
        Err(cause) => refuse(history, cause, None),
        Ok(result) => completed(state, candidate, observer, history, result, endpoint),
    }
}
fn admit(
    state: &SpectralState,
    workspace: &AttemptWorkspace,
    observer: &dyn BalanceObserver,
    history: &RunHistory,
) -> Result<TickClock, SolverError> {
    let controller = history.controller();
    let endpoint = controller.next_clock()?;
    let configuration = controller.configuration();
    if controller.clock() != state.clock()
        || controller.committed() as u128 != state.accepted_steps()
        || configuration.method != workspace.method()
    {
        return Err(SolverError::InvalidPayload);
    }
    let bounds = observer.bounds().ok_or(SolverError::UnknownProviderCost)?;
    let workspace_bytes =
        AttemptWorkspace::reservation_with_method(state.plan().domain(), configuration.method)?;
    let storage = workspace_bytes
        .checked_add(bounds.storage_bytes)
        .ok_or(SolverError::SizeOverflow)?;
    if storage > state.plan().classes()[6]
        || bounds.work_units == 0
        || RunHistory::reservation(configuration)? > state.plan().classes()[7]
    {
        return Err(SolverError::ResourceLimit);
    }
    bounds
        .work_units
        .checked_mul(configuration.limits.maximum_attempts)
        .ok_or(SolverError::SizeOverflow)?;
    Ok(endpoint)
}
pub(super) fn completed(
    state: &mut SpectralState,
    candidate: &mut CandidateState,
    observer: &mut dyn BalanceObserver,
    history: &mut RunHistory,
    result: AttemptResult,
    endpoint: TickClock,
) -> Result<Outcome, SolverError> {
    let Some(token) = result.accepted else {
        let outcome = Outcome::Rejected(result.indicators);
        let update = history.prepare(outcome, None)?;
        history.apply(update);
        return Ok(outcome);
    };
    let transaction = match prepare_commit(state.plan(), state, candidate, token) {
        Ok(transaction) => transaction,
        Err(cause) => return refuse(history, cause, Some(result.indicators)),
    };
    let proposed = transaction.proposal();
    let sample = if proposed.clock() != endpoint
        || proposed.accepted_steps() != history.controller().committed() as u128 + 1
    {
        Err(SolverError::InvalidPayload)
    } else {
        observer.measure(proposed)
    };
    let outcome = Outcome::Committed(result.indicators);
    let update = match sample.and_then(|sample| history.prepare(outcome, Some(sample))) {
        Ok(update) => update,
        Err(cause) => return refuse(history, cause, Some(result.indicators)),
    };
    transaction.commit();
    history.apply(update);
    Ok(outcome)
}
fn refuse(
    history: &mut RunHistory,
    cause: SolverError,
    indicators: Option<Indicators>,
) -> Result<Outcome, SolverError> {
    let outcome = Outcome::Refused { cause, indicators };
    let update = history.prepare(outcome, None)?;
    history.apply(update);
    Ok(outcome)
}
