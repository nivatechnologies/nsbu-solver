//! Preflighted attempt records and restartable controller/diagnostic history.
use super::control::{Configuration, Controller, Outcome};
use crate::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory},
    domain::TickClock,
    storage::reserved,
    SolverError,
};

/// One actual attempt's starting clock and outcome, including failures without advancement.
#[derive(Debug, Clone, Copy)]
pub struct AttemptRecord {
    /// Exact committed start; the fixed configuration retains the requested tick interval.
    pub start: TickClock,
    /// All available local indicators and the commit/reject/refusal classification.
    pub outcome: Outcome,
    /// Actual accepted-state measurement; absent for rejected/refused attempts.
    pub sample: Option<BalanceSample>,
}

/// Owned bounded records plus all fixed-controller and accumulated balance state.
/// Complete checkpoints also require physical state, artifacts, reconstruction and lineage.
#[derive(Debug)]
pub struct RunHistory {
    controller: Controller,
    balance: BalanceHistory,
    records: Vec<AttemptRecord>,
}
pub(super) struct Update {
    controller: Controller,
    balance: BalanceHistory,
    record: AttemptRecord,
}
impl RunHistory {
    /// Additional inline and attempt-record storage, excluding caller allocator overhead.
    pub fn reservation(configuration: Configuration) -> Result<usize, SolverError> {
        configuration
            .limits
            .maximum_attempts
            .checked_mul(std::mem::size_of::<AttemptRecord>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .filter(|&bytes| bytes <= isize::MAX as usize)
            .ok_or(SolverError::SizeOverflow)
    }
    /// Admit the complete finite history before allocation, starting at exact zero.
    pub fn new(
        clock: TickClock,
        configuration: Configuration,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let controller = Controller::new(clock, configuration)?;
        if Self::reservation(configuration)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let steps = controller.required_commits();
        let balance = BalanceHistory::new(clock, BalanceSample::REST, (steps + 1).max(3))?;
        let records = reserved(configuration.limits.maximum_attempts)?;
        Ok(Self {
            controller,
            balance,
            records,
        })
    }
    /// Complete fixed-step controller state, including terminal refusal/rejection.
    pub fn controller(&self) -> Controller {
        self.controller
    }
    /// Complete compensated balance state, including a pending midpoint.
    pub fn balance(&self) -> BalanceHistory {
        self.balance
    }
    /// Every attempt, without dropping failed or rejected work.
    pub fn records(&self) -> &[AttemptRecord] {
        &self.records
    }
    /// Reconstruct controller and compensated balances in their original operation order.
    /// This validates the log's internal consistency, not its provenance or physical truth.
    /// The recorded execution profile must match for bitwise replay guarantees.
    pub fn replay(
        clock: TickClock,
        configuration: Configuration,
        records: &[AttemptRecord],
        cap: usize,
    ) -> Result<Self, SolverError> {
        if records.len() > configuration.limits.maximum_attempts {
            return Err(SolverError::ResourceLimit);
        }
        let mut history = Self::new(clock, configuration, cap)?;
        for record in records {
            if record.start != history.controller.clock() {
                return Err(SolverError::InvalidClock);
            }
            let update = history.prepare(record.outcome, record.sample)?;
            history.apply(update);
        }
        Ok(history)
    }
    pub(super) fn prepare(
        &self,
        outcome: Outcome,
        sample: Option<BalanceSample>,
    ) -> Result<Update, SolverError> {
        let controller = self.controller.with_outcome(outcome)?;
        let balance = match outcome {
            Outcome::Committed(_) => self.balance.with_sample(
                controller.clock(),
                sample.ok_or(SolverError::InvalidPayload)?,
            )?,
            _ => {
                if sample.is_some() {
                    return Err(SolverError::InvalidPayload);
                }
                self.balance
            }
        };
        Ok(Update {
            controller,
            balance,
            record: AttemptRecord {
                start: self.controller.clock(),
                outcome,
                sample,
            },
        })
    }
    pub(super) fn apply(&mut self, update: Update) {
        // prepare admits at most maximum_attempts; the reserved capacity cannot be exhausted.
        self.records.push(update.record);
        self.controller = update.controller;
        self.balance = update.balance;
    }
}
