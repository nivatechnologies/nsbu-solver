//! Construction of the persistent numerical owners admitted by the frozen resource plan.
use crate::{cache::CachedReducedForce, config, observer::ReducedObserver};
use nsbu_solver::{
    domain::{Domain, Epoch, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace, forcing::ForceLimits, method::Method, rhs::SpectralRhs,
        transaction::CandidateState,
    },
    spectral::{FftBackend, FftCatalog},
    SolverError,
};

pub struct ExecutionOwners {
    pub rhs: SpectralRhs<CachedReducedForce>,
    pub observer: ReducedObserver,
}

pub struct StateOwners {
    pub state: SpectralState,
    pub candidate: CandidateState,
    pub attempts: AttemptWorkspace,
}

pub fn execution(resources: ResourcePlan) -> Result<ExecutionOwners, SolverError> {
    let catalog = FftCatalog::new(FftBackend::RustFft6_4_1AvxFma, resources.classes()[4])?;
    let rhs = new_rhs(&catalog)?;
    let observer = new_observer(&catalog)?;
    Ok(ExecutionOwners { rhs, observer })
}

fn new_rhs(catalog: &FftCatalog) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let domain = config::domain()?;
    let samples = Layout::new([config::M; 3])?;
    new_rhs_for(domain, samples, catalog)
}

fn new_rhs_for(
    domain: Domain,
    samples: Layout,
    catalog: &FftCatalog,
) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let limits =
        CachedReducedForce::preflight(domain, samples, config::WORKERS, catalog.backend())?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        config::WORKERS,
        catalog,
        limits.storage_bytes,
    )?;
    finish_rhs(domain, force, limits, catalog)
}

fn finish_rhs(
    domain: Domain,
    force: CachedReducedForce,
    limits: ForceLimits,
    catalog: &FftCatalog,
) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let bytes =
        SpectralRhs::<CachedReducedForce>::reservation_with_catalog(domain, limits, catalog)?;
    SpectralRhs::new_with_catalog(domain, force, config::ADVECTIVE_LIMIT, catalog, bytes)
}

fn new_observer(catalog: &FftCatalog) -> Result<ReducedObserver, SolverError> {
    let domain = config::domain()?;
    let samples = Layout::new([2 * config::M; 3])?;
    new_observer_for(domain, samples, catalog)
}

fn new_observer_for(
    domain: Domain,
    samples: Layout,
    catalog: &FftCatalog,
) -> Result<ReducedObserver, SolverError> {
    let bytes = ReducedObserver::preflight(domain, samples, config::WORKERS, catalog.backend())?;
    ReducedObserver::new(domain, samples, config::WORKERS, catalog, bytes)
}

pub fn states(resources: ResourcePlan) -> Result<StateOwners, SolverError> {
    let (state, candidate) = state_pair(resources)?;
    let attempts = AttemptWorkspace::new_with_method(resources, Method::CoxMatthews)?;
    Ok(StateOwners {
        state,
        candidate,
        attempts,
    })
}

fn state_pair(resources: ResourcePlan) -> Result<(SpectralState, CandidateState), SolverError> {
    let clock = TickClock::from_rest(-20, 8192)?;
    let state = SpectralState::from_rest(resources, clock, Epoch(0))?;
    let candidate = CandidateState::new(resources, clock, Epoch(0))?;
    Ok((state, candidate))
}
