//! Bounded fixed-interval trajectory orchestration; every numerical update uses a swap commit.
use super::{
    attempt::AttemptWorkspace,
    indicator::Tolerances,
    kernel::RightHandSide,
    transaction::{commit_candidate, CandidateState},
};
use crate::{
    domain::{SpectralState, TickClock},
    SolverError,
};

/// Explicit fixed-grid diagnostic work budget. This is not an adaptive convergence policy.
#[derive(Debug, Clone, Copy)]
pub struct RunLimits {
    /// Exact elapsed tick at the requested endpoint, strictly before the clock target.
    pub endpoint: u128,
    /// Fixed macro interval, including its full/two-half local comparison.
    pub step_ticks: u128,
    /// Maximum attempts admitted before starting; insufficient budgets are refused.
    pub maximum_attempts: usize,
}

impl RunLimits {
    /// Require an integral number of quarter-resolved macro steps and a finite attempt budget.
    pub fn attempts(self, clock: TickClock) -> Result<usize, SolverError> {
        if self.endpoint <= clock.elapsed() || self.endpoint >= clock.target() {
            return Err(SolverError::InvalidClock);
        }
        if self.step_ticks == 0 || !self.step_ticks.is_multiple_of(4) {
            return Err(SolverError::InvalidStep);
        }
        let distance = self.endpoint - clock.elapsed();
        if !distance.is_multiple_of(self.step_ticks) {
            return Err(SolverError::InvalidStep);
        }
        let attempts =
            usize::try_from(distance / self.step_ticks).map_err(|_| SolverError::SizeOverflow)?;
        if attempts > self.maximum_attempts || self.maximum_attempts > usize::MAX / 12 {
            return Err(SolverError::RetryLimit);
        }
        Ok(attempts)
    }
}

/// Why a finite run stopped; reaching an endpoint does not qualify a PDE window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Every requested macro step was committed.
    EndpointReached,
    /// A full/two-half comparison exceeded the local tolerance.
    LocalErrorRejected,
    /// An attempted operation was refused; the last committed state remains available.
    Refused(SolverError),
}

/// Fixed-size execution evidence, including partial progress on failure.
#[derive(Debug, Clone, Copy)]
pub struct RunReport {
    /// Actual stopping reason.
    pub reason: StopReason,
    /// Clock of the last committed state.
    pub clock: TickClock,
    /// Number of attempts begun during this invocation.
    pub attempted: usize,
    /// Number of transactional commits during this invocation.
    pub committed: usize,
    /// Largest completed-attempt local ratios; neither is a global error estimate.
    pub maximum_local_ratios: [f64; 2],
}

/// Borrow already preflighted independent workspaces; this orchestrator owns no heap buffers.
/// The source interface receives evolving fields for its RHS, never analytical reference access.
pub struct FixedRun<'a> {
    state: &'a mut SpectralState,
    candidate: &'a mut CandidateState,
    workspace: &'a mut AttemptWorkspace,
    rhs: &'a mut dyn RightHandSide,
}

impl<'a> FixedRun<'a> {
    /// Assemble a run over caller-owned buffers. Each attempt validates their common plan.
    pub fn new(
        state: &'a mut SpectralState,
        candidate: &'a mut CandidateState,
        workspace: &'a mut AttemptWorkspace,
        rhs: &'a mut dyn RightHandSide,
    ) -> Self {
        Self {
            state,
            candidate,
            workspace,
            rhs,
        }
    }

    /// Execute the finite fixed-step policy, stopping at the first local rejection or refusal.
    /// Invalid run configuration is refused before any attempt. Earlier successful commits
    /// survive a later failure; the returned clock and counters describe that partial progress.
    pub fn execute(
        &mut self,
        limits: RunLimits,
        tolerances: Tolerances,
    ) -> Result<RunReport, SolverError> {
        tolerances.validate()?;
        let attempts = limits.attempts(self.state.clock())?;
        let mut report = RunReport {
            reason: StopReason::EndpointReached,
            clock: self.state.clock(),
            attempted: 0,
            committed: 0,
            maximum_local_ratios: [0.0; 2],
        };
        for _ in 0..attempts {
            report.attempted += 1;
            report.reason = self.advance(
                limits.step_ticks,
                tolerances,
                &mut report.maximum_local_ratios,
            );
            if report.reason != StopReason::EndpointReached {
                break;
            }
            report.committed += 1;
            report.clock = self.state.clock();
        }
        Ok(report)
    }

    fn advance(
        &mut self,
        ticks: u128,
        tolerances: Tolerances,
        maximum: &mut [f64; 2],
    ) -> StopReason {
        let attempt = match self.workspace.try_advance(
            self.state,
            self.candidate,
            ticks,
            tolerances,
            self.rhs,
        ) {
            Ok(value) => value,
            Err(error) => return StopReason::Refused(error),
        };
        for (value, ratio) in maximum.iter_mut().zip(attempt.indicators.ratios) {
            *value = value.max(ratio);
        }
        let Some(token) = attempt.accepted else {
            return StopReason::LocalErrorRejected;
        };
        commit_candidate(self.state.plan(), self.state, self.candidate, token)
            .map_or_else(StopReason::Refused, |_| StopReason::EndpointReached)
    }
}
