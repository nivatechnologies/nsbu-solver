//! Closed observation profiles supported by the owned smooth runtime.
use crate::smooth_observer::{
    reconstruction::{ReconstructionObserver, ReconstructionSnapshot},
    BalanceObserver, BalanceObserverWork,
};
use nsbu_solver::{
    domain::{Domain, ResourcePlan, SpectralState},
    experiment::observer::BalanceObserver as Contract,
    SolverError,
};

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::BalanceObserver {}
    impl Sealed for super::ReconstructionObserver {}
}

/// Observation ownership contract for the two built-in smooth runtime profiles.
///
/// This trait is sealed: callers select a provided run alias. Numerical observer callbacks
/// remain substitutable through the solver's public observer contract; this extension also
/// requires trusted snapshots and a complete construction reservation.
pub trait Observation: sealed::Sealed + Contract + std::fmt::Debug + Sized {
    /// Owned accepted diagnostics and spent work, excluding replaceable scratch.
    type Snapshot: std::fmt::Debug;
    /// Complete fixed storage before construction.
    fn reservation(domain: Domain, samples: usize) -> Result<usize, SolverError>;
    /// Construct from the actual zero state after reservation.
    fn from_rest(
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
    ) -> Result<Self, SolverError>;
    /// Charged provider/transform work; extra modal work stays on the concrete observer.
    fn consumption(&self) -> BalanceObserverWork;
    /// Snapshot-owned storage, before allocating its fields.
    fn snapshot_reservation(domain: Domain) -> Result<usize, SolverError>;
    /// Capture a trusted snapshot without changing accepted state or budget.
    fn capture(&self, cap: usize) -> Result<Self::Snapshot, SolverError>;
    /// Restore accepted diagnostics and fresh scratch without evaluating a new sample.
    fn restore(
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
        snapshot: Self::Snapshot,
    ) -> Result<Self, SolverError>;
}

impl Observation for BalanceObserver {
    type Snapshot = BalanceObserverWork;
    fn reservation(domain: Domain, samples: usize) -> Result<usize, SolverError> {
        Ok(Self::limits(domain, samples)?.storage_bytes)
    }
    fn from_rest(
        plan: ResourcePlan,
        samples: usize,
        _: &SpectralState,
    ) -> Result<Self, SolverError> {
        Self::new(plan, samples)
    }
    fn consumption(&self) -> BalanceObserverWork {
        self.consumption()
    }
    fn snapshot_reservation(_: Domain) -> Result<usize, SolverError> {
        Ok(0)
    }
    fn capture(&self, _: usize) -> Result<Self::Snapshot, SolverError> {
        Ok(self.consumption())
    }
    fn restore(
        plan: ResourcePlan,
        samples: usize,
        _: &SpectralState,
        snapshot: Self::Snapshot,
    ) -> Result<Self, SolverError> {
        Self::restore(plan, samples, snapshot)
    }
}

impl Observation for ReconstructionObserver {
    type Snapshot = ReconstructionSnapshot;
    fn reservation(domain: Domain, samples: usize) -> Result<usize, SolverError> {
        Ok(Self::limits(domain, samples)?.storage_bytes)
    }
    fn from_rest(
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
    ) -> Result<Self, SolverError> {
        Self::new(plan, samples, state)
    }
    fn consumption(&self) -> BalanceObserverWork {
        self.consumption()
    }
    fn snapshot_reservation(domain: Domain) -> Result<usize, SolverError> {
        Self::snapshot_reservation(domain)
    }
    fn capture(&self, cap: usize) -> Result<Self::Snapshot, SolverError> {
        self.snapshot(cap)
    }
    fn restore(
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
        snapshot: Self::Snapshot,
    ) -> Result<Self, SolverError> {
        Self::restore(plan, samples, state, snapshot)
    }
}
