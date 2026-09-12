//! Joint stream admission and complete shared-owner/handle bounds.
use super::{
    call_schedule, SharedForceAdapter, SharedForceAdapterBounds, SharedForceAdapterWork,
    SharedForceAttempt, SharedForceStream, MAXIMUM_CALLS,
};
use crate::v2_experiment::shared_force::{
    SharedForceCopy, SharedForceError, SharedForceTable, SharedForceTablePlan, SharedForceWork,
};
use nsbu_solver::{integrators::forcing::ForceLimits, SolverError};
use std::cell::{Cell, Ref, RefCell};

const HANDLE_ALLOWANCE: usize = 64;
const SET_ALLOWANCE: usize = 64;

/// Joint admission for one table and every borrowed integration-force handle.
#[derive(Debug, Clone, Copy)]
pub struct SharedForceAdapterSetPlan<'table, 'streams> {
    table: SharedForceTablePlan<'table>,
    streams: &'streams [SharedForceStream<'streams>],
    bounds: SharedForceAdapterBounds,
    limits: ForceLimits,
}
impl<'table, 'streams> SharedForceAdapterSetPlan<'table, 'streams> {
    /// Require exact equality between stream-derived calls and table copy allowances.
    pub fn new(
        table: SharedForceTablePlan<'table>,
        streams: &'streams [SharedForceStream<'streams>],
        joint_cap: usize,
    ) -> Result<Self, SolverError> {
        if streams.is_empty() {
            return Err(SolverError::InvalidPayload);
        }
        let admission_schedule_visits = admission_visits(table, streams)?;
        validate_streams(table, streams)?;
        let limits = adapter_limits(table)?;
        let bounds = adapter_bounds(table, streams, limits, admission_schedule_visits)?;
        if bounds.joint_peak_bytes > joint_cap {
            return Err(SolverError::ResourceLimit);
        }
        Ok(Self {
            table,
            streams,
            bounds,
            limits,
        })
    }
    /// Shared immutable table admission counted exactly once.
    pub fn table(self) -> SharedForceTablePlan<'table> {
        self.table
    }
    /// Complete ordered set of independently consumed attempt streams.
    pub fn streams(self) -> &'streams [SharedForceStream<'streams>] {
        self.streams
    }
    /// Shared owner, borrowed manifests, handle storage and work bounds.
    pub fn bounds(self) -> SharedForceAdapterBounds {
        self.bounds
    }
}

/// One admitted shared table owner that issues every planned adapter exactly once.
pub struct SharedForceAdapterSet<'table, 'streams> {
    plan: SharedForceAdapterSetPlan<'table, 'streams>,
    table: RefCell<SharedForceTable<'table>>,
    next_stream: Cell<usize>,
}
impl<'table, 'streams> SharedForceAdapterSet<'table, 'streams> {
    /// Construct the identity-matched table only after joint set admission.
    pub fn new(
        plan: SharedForceAdapterSetPlan<'table, 'streams>,
    ) -> Result<Self, SharedForceError> {
        Ok(Self {
            plan,
            table: RefCell::new(SharedForceTable::new(plan.table)?),
            next_stream: Cell::new(0),
        })
    }
    /// Issue the next ordered stream handle once without allocating.
    pub fn adapter(
        &self,
        index: usize,
    ) -> Result<SharedForceAdapter<'table, 'streams, '_>, SolverError> {
        if index != self.next_stream.get() {
            return Err(SolverError::InvalidIndex);
        }
        let stream = *self
            .plan
            .streams
            .get(index)
            .ok_or(SolverError::InvalidIndex)?;
        self.next_stream
            .set(index.checked_add(1).ok_or(SolverError::SizeOverflow)?);
        Ok(SharedForceAdapter {
            table: &self.table,
            binding: self
                .plan
                .table
                .binding(stream.domain_index)
                .ok_or(SolverError::InvalidPayload)?,
            stream,
            limits: self.plan.limits,
            next_attempt: 0,
            expected: [None; MAXIMUM_CALLS],
            next_call: 0,
            expected_calls: 0,
            charged: SharedForceAdapterWork::default(),
            failed: false,
        })
    }
    /// Fallibly borrow immutable table reports and remaining-copy counters.
    pub fn table(&self) -> Result<Ref<'_, SharedForceTable<'table>>, SolverError> {
        self.table
            .try_borrow()
            .map_err(|_| SolverError::ProviderBudgetExceeded)
    }
    /// Last complete shared copy, retained if a later request fails.
    pub fn current(&self) -> Result<Option<SharedForceCopy>, SolverError> {
        Ok(self.table()?.current())
    }
    /// Shared construction and copy charges counted once.
    pub fn table_work(&self) -> Result<SharedForceWork, SolverError> {
        Ok(self.table()?.charged_work())
    }
    /// Whether a failed table copy terminated the shared owner.
    pub fn is_terminated(&self) -> Result<bool, SolverError> {
        Ok(self.table()?.is_terminated())
    }
}

fn validate_streams(
    table: SharedForceTablePlan<'_>,
    streams: &[SharedForceStream<'_>],
) -> Result<(), SolverError> {
    let mut calls = 0usize;
    let mut matched = 0usize;
    for stream in streams {
        if stream.domain_index >= 3 || stream.attempts.is_empty() {
            return Err(SolverError::InvalidPayload);
        }
        for attempt in stream.attempts {
            let schedule = call_schedule(*attempt)?;
            calls = calls
                .checked_add(attempt.method.rhs_calls())
                .ok_or(SolverError::SizeOverflow)?;
            for clock in schedule
                .into_iter()
                .flatten()
                .take(attempt.method.rhs_calls())
            {
                if clock.exponent() != table.manifest()[0].clock().exponent()
                    || clock.target() != table.manifest()[0].clock().target()
                {
                    return Err(SolverError::InvalidClock);
                }
            }
        }
    }
    if calls != table.maximum_attempts() {
        return Err(SolverError::InvalidPayload);
    }
    for request in table.manifest() {
        for domain_index in 0..3 {
            let mut expected = 0usize;
            for stream in streams
                .iter()
                .filter(|stream| stream.domain_index == domain_index)
            {
                for attempt in stream.attempts {
                    expected = expected
                        .checked_add(
                            call_schedule(*attempt)?
                                .into_iter()
                                .flatten()
                                .take(attempt.method.rhs_calls())
                                .filter(|clock| *clock == request.clock())
                                .count(),
                        )
                        .ok_or(SolverError::SizeOverflow)?;
                }
            }
            if expected != request.maximum_copies()[domain_index] {
                return Err(SolverError::InvalidPayload);
            }
            matched = matched
                .checked_add(expected)
                .ok_or(SolverError::SizeOverflow)?;
        }
    }
    if matched != calls {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

fn admission_visits(
    table: SharedForceTablePlan<'_>,
    streams: &[SharedForceStream<'_>],
) -> Result<usize, SolverError> {
    let calls = streams.iter().try_fold(0usize, |sum, stream| {
        sum.checked_add(stream.maximum_calls)
            .ok_or(SolverError::SizeOverflow)
    })?;
    let schedule_passes = table
        .manifest()
        .len()
        .checked_add(1)
        .and_then(|passes| calls.checked_mul(passes))
        .and_then(|visits| visits.checked_mul(2))
        .ok_or(SolverError::SizeOverflow)?;
    let filters = table
        .manifest()
        .len()
        .checked_mul(3)
        .and_then(|visits| visits.checked_mul(streams.len()))
        .ok_or(SolverError::SizeOverflow)?;
    schedule_passes
        .checked_add(filters)
        .ok_or(SolverError::SizeOverflow)
}

fn adapter_limits(table: SharedForceTablePlan<'_>) -> Result<ForceLimits, SolverError> {
    let work = table.bounds().work;
    let per_attempt = |total: usize| {
        total
            .checked_div(work.copy_attempts)
            .ok_or(SolverError::SizeOverflow)
    };
    let work_units = [
        table.manifest().len(),
        per_attempt(work.binding_checks)?,
        per_attempt(work.coefficient_words_copied)?,
        per_attempt(work.transfer_visits)?,
    ]
    .into_iter()
    .try_fold(0usize, |sum, value| {
        sum.checked_add(value).ok_or(SolverError::SizeOverflow)
    })?;
    Ok(ForceLimits {
        storage_bytes: std::mem::size_of::<SharedForceAdapter<'static, 'static, 'static>>()
            .checked_add(HANDLE_ALLOWANCE)
            .ok_or(SolverError::SizeOverflow)?,
        work_units,
        scalar_transforms: 0,
        remaining_divisor: table.provider().remaining_divisor,
    })
}

fn adapter_bounds(
    table: SharedForceTablePlan<'_>,
    streams: &[SharedForceStream<'_>],
    limits: ForceLimits,
    admission_schedule_visits: usize,
) -> Result<SharedForceAdapterBounds, SolverError> {
    let handles = limits
        .storage_bytes
        .checked_mul(streams.len())
        .ok_or(SolverError::SizeOverflow)?;
    let attempt_count = streams.iter().try_fold(0usize, |sum, stream| {
        sum.checked_add(stream.attempts.len())
            .ok_or(SolverError::SizeOverflow)
    })?;
    let calls = table.maximum_attempts();
    let stream_manifest = streams
        .len()
        .checked_mul(std::mem::size_of::<SharedForceStream<'_>>())
        .and_then(|bytes| {
            attempt_count
                .checked_mul(std::mem::size_of::<SharedForceAttempt>())
                .and_then(|attempts| bytes.checked_add(attempts))
        })
        .ok_or(SolverError::SizeOverflow)?;
    let caller_manifest_bytes = table
        .bounds()
        .caller_manifest_bytes
        .checked_add(stream_manifest)
        .ok_or(SolverError::SizeOverflow)?;
    let set = std::mem::size_of::<SharedForceAdapterSet<'_, '_>>()
        .checked_add(SET_ALLOWANCE)
        .ok_or(SolverError::SizeOverflow)?;
    let storage_bytes = table
        .bounds()
        .storage_bytes
        .checked_add(handles)
        .and_then(|bytes| bytes.checked_add(set))
        .ok_or(SolverError::SizeOverflow)?;
    let work = SharedForceAdapterWork {
        attempts: attempt_count,
        calls,
        clock_comparisons: calls,
        binding_checks: attempt_count
            .checked_mul(8)
            .and_then(|value| {
                calls
                    .checked_mul(4)
                    .and_then(|calls| value.checked_add(calls))
            })
            .ok_or(SolverError::SizeOverflow)?,
        table_work_units: limits
            .work_units
            .checked_mul(calls)
            .ok_or(SolverError::SizeOverflow)?,
        schedule_visits: calls,
    };
    Ok(SharedForceAdapterBounds {
        storage_bytes,
        handle_storage_bytes: limits.storage_bytes,
        set_storage_bytes: set,
        caller_manifest_bytes,
        construction_peak_bytes: table
            .bounds()
            .construction_peak_bytes
            .checked_add(handles)
            .and_then(|bytes| bytes.checked_add(set))
            .ok_or(SolverError::SizeOverflow)?,
        joint_peak_bytes: table
            .bounds()
            .joint_peak_bytes
            .checked_add(handles)
            .and_then(|bytes| bytes.checked_add(set))
            .and_then(|bytes| bytes.checked_add(stream_manifest))
            .ok_or(SolverError::SizeOverflow)?,
        admission_schedule_visits,
        work,
    })
}
