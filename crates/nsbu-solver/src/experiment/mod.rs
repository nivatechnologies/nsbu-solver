//! Experiment-level transactions join physical commits to separately measured history.
pub mod control;
pub mod log;
pub mod observer;
pub mod runner;
use crate::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory},
    domain::SpectralState,
    integrators::transaction::{prepare_commit, AcceptedAttempt, CandidateState},
    SolverError,
};

/// Commit a physical proposal and its balance history together, after every fallible check.
/// The supplied sample must be measured from `CandidateState::proposal`; this function
/// checks transaction and history identities, not the correctness of an external measurement.
/// Failure consumes the token but changes neither committed state nor diagnostic history.
pub fn commit_balanced(
    state: &mut SpectralState,
    candidate: &mut CandidateState,
    accepted: AcceptedAttempt,
    history: &mut BalanceHistory,
    sample: BalanceSample,
) -> Result<(), SolverError> {
    if history.clock() != state.clock() || (history.samples() - 1) as u128 != state.accepted_steps()
    {
        return Err(SolverError::InvalidPayload);
    }
    let transaction = prepare_commit(state.plan(), state, candidate, accepted)?;
    let diagnostic = history.with_sample(transaction.proposal().clock(), sample)?;
    transaction.commit();
    *history = diagnostic;
    Ok(())
}

#[cfg(test)]
mod tests;
