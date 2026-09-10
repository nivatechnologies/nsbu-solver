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
    /// Measure a private proposal without changing physical or committed diagnostic state.
    fn measure(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError>;
}
