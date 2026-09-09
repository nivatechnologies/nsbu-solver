//! Finite external local-error retry policy; the one-attempt numerical core never retries.
use super::{
    attempt::{AttemptResult, AttemptWorkspace},
    indicator::Tolerances,
    kernel::RightHandSide,
    transaction::CandidateState,
};
use crate::{domain::SpectralState, SolverError};

/// Try successively halved exact intervals, stopping on acceptance or a finite retry cap.
/// Only empirical local-error rejection is retryable. Arithmetic, input and resource errors propagate.
pub fn retry_local(
    work: &mut AttemptWorkspace,
    committed: &SpectralState,
    candidate: &mut CandidateState,
    initial_ticks: u128,
    tolerances: Tolerances,
    max_attempts: u32,
    rhs: &mut dyn RightHandSide,
) -> Result<AttemptResult, SolverError> {
    if max_attempts == 0 || max_attempts > 64 {
        return Err(SolverError::InvalidStep);
    }
    let mut ticks = initial_ticks;
    for _ in 0..max_attempts {
        let result = work.try_advance(committed, candidate, ticks, tolerances, rhs)?;
        if result.accepted.is_some() {
            return Ok(result);
        }
        ticks /= 2;
        if !ticks.is_multiple_of(4) {
            return Err(SolverError::ClockCapacityExceeded);
        }
    }
    Err(SolverError::RetryLimit)
}
