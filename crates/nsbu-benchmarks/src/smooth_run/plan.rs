//! Complete resource admission for each supported smooth observation profile.
use super::{ledger_bytes, observation::Observation, OwnedRun};
use crate::smooth::CyclicSine;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, TickClock},
    experiment::{
        control::{Configuration, Controller},
        log::RunHistory,
    },
    integrators::{attempt::AttemptWorkspace, forcing::PrescribedForce, rhs::SpectralRhs},
    SolverError,
};
const ALLOCATOR_ALLOWANCE: usize = 4096;

/// Allocation-free admission evidence for a bounded [`OwnedRun`] from rest.
#[derive(Debug)]
pub struct OwnedPlan<O: Observation> {
    observation: std::marker::PhantomData<O>,
    plan: ResourcePlan,
    configuration: Configuration,
    observer_samples: usize,
    integration_calls: usize,
    integration_work_units: usize,
    integration_scalar_transforms: usize,
}

impl<O: Observation> OwnedPlan<O> {
    /// Validate the same complete configuration and resource ledger used by construction.
    pub fn from_rest(
        domain: Domain,
        clock: TickClock,
        configuration: Configuration,
        observer_samples: usize,
        advective_limit: f64,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let rest = TickClock::from_rest(clock.exponent(), clock.target())?;
        if clock != rest {
            return Err(SolverError::InvalidClock);
        }
        let controller = Controller::new(clock, configuration)?;
        if !advective_limit.is_finite() || advective_limit <= 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let source = CyclicSine::new(domain)?;
        let limits = source.limits().ok_or(SolverError::UnknownProviderCost)?;
        let integration_calls = configuration
            .method
            .rhs_calls()
            .checked_mul(controller.required_commits())
            .ok_or(SolverError::SizeOverflow)?;
        let integration_work_units = limits
            .work_units
            .checked_mul(integration_calls)
            .ok_or(SolverError::SizeOverflow)?;
        let integration_scalar_transforms = limits
            .scalar_transforms
            .checked_add(10)
            .and_then(|value| value.checked_mul(integration_calls))
            .ok_or(SolverError::SizeOverflow)?;
        Ok(Self {
            plan: plan::<O>(domain, configuration, observer_samples, cap)?,
            observation: std::marker::PhantomData,
            configuration,
            observer_samples,
            integration_calls,
            integration_work_units,
            integration_scalar_transforms,
        })
    }

    /// Checked resource ledger without numerical-state allocation.
    pub fn resources(self) -> ResourcePlan {
        self.plan
    }
    /// Frozen run policy admitted by this plan.
    pub fn configuration(self) -> Configuration {
        self.configuration
    }
    /// Number of independently measured accepted states admitted by the plan.
    pub fn observer_samples(self) -> usize {
        self.observer_samples
    }
    /// Declared integration RHS calls at the endpoint.
    pub fn integration_calls(self) -> usize {
        self.integration_calls
    }
    /// Declared integration provider work units at the endpoint.
    pub fn integration_work_units(self) -> usize {
        self.integration_work_units
    }
    /// Declared integration scalar transforms at the endpoint.
    pub fn integration_scalar_transforms(self) -> usize {
        self.integration_scalar_transforms
    }
}

impl<O: Observation> Copy for OwnedPlan<O> {}
impl<O: Observation> Clone for OwnedPlan<O> {
    fn clone(&self) -> Self {
        *self
    }
}

fn plan<O: Observation>(
    domain: Domain,
    configuration: Configuration,
    observer_samples: usize,
    cap: usize,
) -> Result<ResourcePlan, SolverError> {
    let source = CyclicSine::new(domain)?;
    let limits = source.limits().ok_or(SolverError::UnknownProviderCost)?;
    let force = SpectralRhs::<CyclicSine>::reservation(domain, limits)?;
    let diagnostics = AttemptWorkspace::reservation_with_method(domain, configuration.method)?
        .checked_add(O::reservation(domain, observer_samples)?)
        .ok_or(SolverError::SizeOverflow)?;
    let overhead = RunHistory::reservation(configuration)?
        .checked_add(ledger_bytes(configuration.limits.maximum_attempts)?)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<OwnedRun<O>>()))
        .and_then(|bytes| bytes.checked_add(ALLOCATOR_ALLOWANCE))
        .ok_or(SolverError::SizeOverflow)?;
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force,
            diagnostics,
            overhead,
        },
        cap,
        Epoch(0),
    )
}
