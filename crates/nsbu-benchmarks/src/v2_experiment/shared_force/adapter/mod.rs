//! Fallible borrowed adapters from an admitted shared table to the generic spectral RHS.
use nsbu_solver::{domain::TickClock, integrators::method::Method, SolverError};

mod plan;
mod provider;

pub use plan::{SharedForceAdapterSet, SharedForceAdapterSetPlan};
pub use provider::SharedForceAdapter;

const MAXIMUM_CALLS: usize = 15;

/// One exact attempted interval and integration method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedForceAttempt {
    start: TickClock,
    ticks: u128,
    method: Method,
}
impl SharedForceAttempt {
    /// Admit a positive interval with integral quarter-stage clocks.
    pub fn new(start: TickClock, ticks: u128, method: Method) -> Result<Self, SolverError> {
        start.stages(ticks)?;
        Ok(Self {
            start,
            ticks,
            method,
        })
    }
    /// Exact attempt start.
    pub fn start(self) -> TickClock {
        self.start
    }
    /// Exact attempted interval.
    pub fn ticks(self) -> u128 {
        self.ticks
    }
    /// Fixed integration method and source-call schedule.
    pub fn method(self) -> Method {
        self.method
    }
}

/// One trajectory's borrowed attempt manifest and retained-domain selection.
#[derive(Debug, Clone, Copy)]
pub struct SharedForceStream<'a> {
    domain_index: usize,
    attempts: &'a [SharedForceAttempt],
    maximum_calls: usize,
}
impl<'a> SharedForceStream<'a> {
    /// Bind a nonempty exact attempt manifest to one of the table's three domains.
    pub fn new(
        domain_index: usize,
        attempts: &'a [SharedForceAttempt],
    ) -> Result<Self, SolverError> {
        if domain_index >= 3 || attempts.is_empty() {
            return Err(SolverError::InvalidPayload);
        }
        let maximum_calls = attempts.iter().try_fold(0usize, |sum, attempt| {
            sum.checked_add(attempt.method.rhs_calls())
                .ok_or(SolverError::SizeOverflow)
        })?;
        Ok(Self {
            domain_index,
            attempts,
            maximum_calls,
        })
    }
    /// Index in the table plan's ordered retained domains.
    pub fn domain_index(self) -> usize {
        self.domain_index
    }
    /// Complete exact attempt sequence for this handle.
    pub fn attempts(self) -> &'a [SharedForceAttempt] {
        self.attempts
    }
    /// Total force calls derived once from every admitted method.
    pub fn maximum_calls(self) -> usize {
        self.maximum_calls
    }
}

/// Runtime handle charges, separate from the table's construction and copy ledger.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SharedForceAdapterWork {
    /// Attempts charged before exact interval validation.
    pub attempts: usize,
    /// Calls charged before exact clock, output and table validation.
    pub calls: usize,
    /// Exact expected-call clock comparisons.
    pub clock_comparisons: usize,
    /// Conservative handle limit, identity and attempt checks.
    pub binding_checks: usize,
    /// Abstract shared-table lookup, binding and strict-transfer work exposed to the RHS.
    pub table_work_units: usize,
    /// Fixed schedule slots charged before each attempt is validated or constructed.
    pub schedule_visits: usize,
}

/// Shared owner, caller manifests, handles and bounded work admitted as one set.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SharedForceAdapterBounds {
    /// Persistent table plus all adapter headers and allocator allowances.
    pub storage_bytes: usize,
    /// Storage reserved inside each generic RHS for one borrowed adapter header.
    pub handle_storage_bytes: usize,
    /// Set header and allocator-metadata allowance outside the table owner.
    pub set_storage_bytes: usize,
    /// Borrowed table, stream and attempt manifest storage retained by the caller.
    pub caller_manifest_bytes: usize,
    /// Construction peak including the one original provider and every adapter header.
    pub construction_peak_bytes: usize,
    /// Maximum simultaneous owner, caller manifests, handles and one caller output.
    pub joint_peak_bytes: usize,
    /// Conservative schedule writes/comparisons and stream filters during admission.
    pub admission_schedule_visits: usize,
    /// Complete runtime handle allowance; table work remains separately queryable.
    pub work: SharedForceAdapterWork,
}

fn call_schedule(
    attempt: SharedForceAttempt,
) -> Result<[Option<TickClock>; MAXIMUM_CALLS], SolverError> {
    let stages = attempt.start.stages(attempt.ticks)?;
    let indices: &[usize] = match attempt.method {
        Method::CoxMatthews => &[0, 2, 2, 4, 0, 1, 1, 2, 2, 3, 3, 4],
        Method::HochbruckOstermann => &[0, 2, 2, 4, 2, 0, 1, 1, 2, 1, 2, 3, 3, 4, 3],
    };
    let mut result = [None; MAXIMUM_CALLS];
    for (target, index) in result.iter_mut().zip(indices) {
        *target = Some(stages[*index]);
    }
    Ok(result)
}
