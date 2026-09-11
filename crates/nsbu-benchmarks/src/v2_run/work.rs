//! Coherence checks for the bounded per-attempt work ledger.
use super::Plan;
use crate::smooth_observer::{v2::V2Observer, BalanceObserverWork};
use nsbu_solver::{
    domain::SpectralState,
    experiment::{control::Outcome, log::RunHistory},
    SolverError,
};

/// Charged integration and independent observation work for a single attempted interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptWork {
    pub(super) integration: [usize; 3],
    pub(super) observation: BalanceObserverWork,
}
impl AttemptWork {
    /// RHS calls, provider work units, scalar transforms; failed calls retain their charge.
    pub fn integration(self) -> [usize; 3] {
        self.integration
    }
    /// Diagnostic charge incurred by this attempt, separate from its integration work.
    pub fn observation(self) -> BalanceObserverWork {
        self.observation
    }
}
pub(super) fn reservation(attempts: usize) -> Result<usize, SolverError> {
    attempts
        .checked_mul(std::mem::size_of::<AttemptWork>())
        .and_then(|n| n.checked_add(std::mem::size_of::<Vec<AttemptWork>>()))
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or(SolverError::SizeOverflow)
}
pub(super) fn storage(plan: Plan) -> Result<Vec<AttemptWork>, SolverError> {
    let mut work = Vec::new();
    work.try_reserve_exact(plan.settings().configuration.limits.maximum_attempts)
        .map_err(|_| SolverError::AllocationFailed)?;
    Ok(work)
}
pub(super) fn validate(
    plan: Plan,
    state: &SpectralState,
    history: &RunHistory,
    work: &[AttemptWork],
    observation: BalanceObserverWork,
) -> Result<(), SolverError> {
    let settings = plan.settings();
    // Both callers constructed/decoded the physical payload under this exact resource plan.
    if state.clock().target() != settings.initial_clock.target()
        || state.clock().exponent() != settings.initial_clock.exponent()
        || state.clock() != history.controller().clock()
        || state.accepted_steps() != history.controller().committed() as u128
        || state.epoch().0 != state.accepted_steps()
        || work.len() != history.records().len()
        || !same_configuration(settings.configuration, history.controller().configuration())
    {
        return Err(SolverError::InvalidPayload);
    }
    V2Observer::validate_restored(
        settings.domain,
        settings.force,
        plan.observer_samples(),
        observation,
    )?;
    let mut sum = BalanceObserverWork::default();
    for (charge, record) in work.iter().zip(history.records()) {
        validate_attempt(plan, *charge, record.outcome)?;
        sum.samples = sum
            .samples
            .checked_add(charge.observation.samples)
            .ok_or(SolverError::SizeOverflow)?;
        sum.work_units = sum
            .work_units
            .checked_add(charge.observation.work_units)
            .ok_or(SolverError::SizeOverflow)?;
        sum.scalar_transforms = sum
            .scalar_transforms
            .checked_add(charge.observation.scalar_transforms)
            .ok_or(SolverError::SizeOverflow)?;
    }
    if sum != observation {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}
fn validate_attempt(plan: Plan, charge: AttemptWork, outcome: Outcome) -> Result<(), SolverError> {
    let settings = plan.settings();
    let limits = settings.force.limits(settings.domain)?;
    let [calls, units, transforms] = charge.integration;
    let lower = settings
        .force
        .samples
        .real_len()
        .checked_mul(calls)
        .ok_or(SolverError::SizeOverflow)?;
    let upper = limits
        .work_units
        .checked_mul(calls)
        .ok_or(SolverError::SizeOverflow)?;
    let expected = limits
        .scalar_transforms
        .checked_add(10)
        .and_then(|n| n.checked_mul(calls))
        .ok_or(SolverError::SizeOverflow)?;
    if calls > settings.configuration.method.rhs_calls()
        || units < lower
        || units > upper
        || transforms != expected
    {
        return Err(SolverError::InvalidPayload);
    }
    V2Observer::validate_restored(settings.domain, settings.force, 1, charge.observation)?;
    validate_outcome(
        settings.configuration.method.rhs_calls(),
        calls,
        charge.observation.samples,
        outcome,
    )
}

fn validate_outcome(
    expected_calls: usize,
    calls: usize,
    samples: usize,
    outcome: Outcome,
) -> Result<(), SolverError> {
    match outcome {
        Outcome::Committed(_) => {
            if calls != expected_calls || samples != 1 {
                return Err(SolverError::InvalidPayload);
            }
        }
        Outcome::Rejected(_) => {
            if calls != expected_calls || samples != 0 {
                return Err(SolverError::InvalidPayload);
            }
        }
        Outcome::Refused { indicators, .. } => {
            if (indicators.is_none() && samples != 0)
                || (indicators.is_some() && calls != expected_calls)
            {
                return Err(SolverError::InvalidPayload);
            }
        }
    }
    Ok(())
}
pub(super) fn same_configuration(
    left: nsbu_solver::experiment::control::Configuration,
    right: nsbu_solver::experiment::control::Configuration,
) -> bool {
    left.method == right.method
        && left.limits.endpoint == right.limits.endpoint
        && left.limits.step_ticks == right.limits.step_ticks
        && left.limits.maximum_attempts == right.limits.maximum_attempts
        && left.tolerances.absolute.map(f64::to_bits) == right.tolerances.absolute.map(f64::to_bits)
        && left.tolerances.relative.map(f64::to_bits) == right.tolerances.relative.map(f64::to_bits)
}
