//! Narrow precommit diagnostic interface; observers receive no mutable physical state.
use crate::{diagnostics::balances::BalanceSample, domain::SpectralState, SolverError};

/// Declared finite costs for one diagnostic evaluation.
#[derive(Debug, Clone, Copy)]
pub struct ObserverBounds {
    /// Complete observer-owned scratch and fixed metadata reservation.
    pub storage_bytes: usize,
    /// Positive upper bound for one evaluation in the provider's declared work units.
    pub work_units: usize,
}

/// Bounded, allocation-free evaluation on the proposed accepted physical field.
/// Implementations must honor their declared cost and use only the stated mathematical inputs.
pub trait BalanceObserver {
    /// None means unbounded/undeclared and is refused before an integration attempt.
    fn bounds(&self) -> Option<ObserverBounds>;
    /// Measure and privately stage diagnostics for a proposed accepted field.
    ///
    /// A successful call opens one pending observer transaction. The runner subsequently calls
    /// exactly one of [`Self::commit_pending`] or [`Self::discard_pending`]. A failed call may
    /// also have staged private work and is followed by `discard_pending`. Implementations enter
    /// each recorded step with no pending transaction.
    fn measure(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError>;

    /// Publish the diagnostics staged by the preceding successful measurement.
    ///
    /// The runner calls this only after its physical payload and `RunHistory` update have both
    /// committed. This callback must be infallible, allocation-free, and covered by
    /// [`ObserverBounds::work_units`].
    fn commit_pending(&mut self) {}

    /// Abandon private diagnostics for a measured proposal.
    ///
    /// The runner calls this exactly once after every measured proposal that cannot commit,
    /// including a failed measurement. This callback must be infallible and allocation-free.
    fn discard_pending(&mut self) {}
}
