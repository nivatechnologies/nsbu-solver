//! Allocation-free prescribed-force handle over a fallibly borrowed table.
use super::{call_schedule, SharedForceAdapterWork, SharedForceStream, MAXIMUM_CALLS};
use crate::v2_experiment::shared_force::{SharedForceBinding, SharedForceTable};
use nsbu_solver::{
    domain::TickClock,
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    Complex64, SolverError,
};
use std::cell::RefCell;

/// Allocation-free prescribed-force view of one externally owned shared table.
pub struct SharedForceAdapter<'table, 'streams, 'owner> {
    pub(super) table: &'owner RefCell<SharedForceTable<'table>>,
    pub(super) binding: SharedForceBinding,
    pub(super) stream: SharedForceStream<'streams>,
    pub(super) limits: ForceLimits,
    pub(super) next_attempt: usize,
    pub(super) expected: [Option<TickClock>; MAXIMUM_CALLS],
    pub(super) next_call: usize,
    pub(super) expected_calls: usize,
    pub(super) charged: SharedForceAdapterWork,
    pub(super) failed: bool,
}
impl SharedForceAdapter<'_, '_, '_> {
    /// Complete charged handle work, excluding table construction.
    pub fn charged_work(&self) -> SharedForceAdapterWork {
        self.charged
    }
    /// Whether this handle permanently refused a malformed, conflicting or exhausted request.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }

    fn charge_attempt(&mut self) -> Result<(), SolverError> {
        if self.charged.attempts == self.stream.attempts.len() {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charged.attempts = self
            .charged
            .attempts
            .checked_add(1)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.binding_checks = self
            .charged
            .binding_checks
            .checked_add(8)
            .ok_or(SolverError::SizeOverflow)?;
        let calls = self.stream.attempts[self.next_attempt].method.rhs_calls();
        self.charged.schedule_visits = self
            .charged
            .schedule_visits
            .checked_add(calls)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }

    fn open(&mut self, clock: TickClock, ticks: u128) -> Result<(), SolverError> {
        if self.next_call != self.expected_calls {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.expected.fill(None);
        self.next_call = 0;
        self.expected_calls = 0;
        let attempt = self
            .stream
            .attempts
            .get(self.next_attempt)
            .copied()
            .ok_or(SolverError::ProviderBudgetExceeded)?;
        if attempt.start != clock || attempt.ticks != ticks {
            return Err(SolverError::InvalidClock);
        }
        self.expected = call_schedule(attempt)?;
        self.expected_calls = attempt.method.rhs_calls();
        self.next_attempt += 1;
        Ok(())
    }

    fn request(
        &mut self,
        clock: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        self.charge_call()?;
        self.validate_request(clock, limit, &output)?;
        self.copy(clock, output)?;
        self.complete_call()
    }

    fn charge_call(&mut self) -> Result<(), SolverError> {
        if self.charged.calls == self.stream.maximum_calls {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charged.calls = self
            .charged
            .calls
            .checked_add(1)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.clock_comparisons = self
            .charged
            .clock_comparisons
            .checked_add(1)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.binding_checks = self
            .charged
            .binding_checks
            .checked_add(4)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }

    fn validate_request(
        &self,
        clock: TickClock,
        limit: ForceLimits,
        output: &[&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        if limit != self.limits
            || self.next_call >= self.expected_calls
            || self.expected[self.next_call] != Some(clock)
            || output
                .iter()
                .any(|values| values.len() != self.binding.domain.layout().half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        Ok(())
    }

    fn copy(&self, clock: TickClock, output: [&mut [Complex64]; 3]) -> Result<(), SolverError> {
        let mut table = self
            .table
            .try_borrow_mut()
            .map_err(|_| SolverError::ProviderBudgetExceeded)?;
        table
            .copy(self.binding, clock, output)
            .map_err(|_| SolverError::ProviderBudgetExceeded)?;
        Ok(())
    }

    fn complete_call(&mut self) -> Result<ForceWork, SolverError> {
        self.next_call += 1;
        self.charged.table_work_units = self
            .charged
            .table_work_units
            .checked_add(self.limits.work_units)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(ForceWork {
            work_units: self.limits.work_units,
            scalar_transforms: 0,
        })
    }
}

impl PrescribedForce for SharedForceAdapter<'_, '_, '_> {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }

    fn begin_attempt(
        &mut self,
        clock: TickClock,
        ticks: u128,
        limit: ForceLimits,
    ) -> Result<(), SolverError> {
        if self.failed {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        if let Err(error) = self.charge_attempt() {
            self.failed = true;
            return Err(error);
        }
        if limit != self.limits {
            self.failed = true;
            return Err(SolverError::ProviderBudgetExceeded);
        }
        if let Err(error) = self.open(clock, ticks) {
            self.failed = true;
            return Err(error);
        }
        Ok(())
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if self.failed {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        match self.request(clock, limit, output) {
            Ok(report) => Ok(report),
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
}
