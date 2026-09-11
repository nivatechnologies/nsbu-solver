//! Bounded smooth balance observers.

mod kernel;
pub(crate) use kernel::field;
pub use kernel::{BalanceObserverLimits, BalanceObserverWork, ForceBalance};

use crate::smooth::CyclicSine;
use nsbu_solver::{
    domain::{Domain, ResourcePlan},
    integrators::forcing::PrescribedForce,
    SolverError,
};

/// Compatibility name for the original cyclic sine observer.
pub type BalanceObserver = ForceBalance<CyclicSine>;

pub mod reconstruction;
pub mod v2;

impl BalanceObserver {
    /// Declares storage and work for cyclic sine samples.
    pub fn limits(source: Domain, samples: usize) -> Result<BalanceObserverLimits, SolverError> {
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let force = CyclicSine::new(diagnostic)?;
        Self::limits_for(
            source,
            samples,
            force.limits().ok_or(SolverError::UnknownProviderCost)?,
        )
    }
    /// Allocates a cyclic sine observer from a reserved plan.
    pub fn new(plan: ResourcePlan, samples: usize) -> Result<Self, SolverError> {
        let force = CyclicSine::new(
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                plan.domain(),
            )?,
        )?;
        Self::new_with_force(plan, samples, force)
    }
    /// Validates a previously charged cyclic sine ledger.
    pub fn validate_restored(
        plan: ResourcePlan,
        samples: usize,
        work: BalanceObserverWork,
    ) -> Result<(), SolverError> {
        let limits = Self::limits(plan.domain(), samples)?;
        if plan.classes()[6] < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        let force = CyclicSine::new(
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                plan.domain(),
            )?,
        )?;
        kernel::validate_work_public(
            limits,
            force.limits().ok_or(SolverError::UnknownProviderCost)?,
            work,
        )
    }
    /// Restores a cyclic sine observer with its charged ledger.
    pub fn restore(
        plan: ResourcePlan,
        samples: usize,
        work: BalanceObserverWork,
    ) -> Result<Self, SolverError> {
        Self::validate_restored(plan, samples, work)?;
        let force = CyclicSine::new(
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                plan.domain(),
            )?,
        )?;
        Self::restore_with_force(plan, samples, work, force)
    }
}
