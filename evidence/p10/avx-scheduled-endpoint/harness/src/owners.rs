//! Construction of the persistent numerical owners admitted by the frozen resource plan.
use crate::{cache::CachedReducedForce, config, observer::ReducedObserver};
#[cfg(feature = "n384-prep")]
use nsbu_solver::spectral::{W3FftIdentity, W3FftMode};
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

#[cfg(not(feature = "n384-prep"))]
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

#[cfg(feature = "n384-prep")]
fn new_rhs_for(
    domain: Domain,
    samples: Layout,
    catalog: &FftCatalog,
) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let limits =
        CachedReducedForce::preflight(domain, samples, config::WORKERS, catalog.backend(), true)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        config::WORKERS,
        catalog,
        limits.storage_bytes,
        true,
    )?;
    finish_rhs(domain, force, limits, catalog)
}

#[cfg(not(feature = "n384-prep"))]
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

#[cfg(feature = "n384-prep")]
fn finish_rhs(
    domain: Domain,
    force: CachedReducedForce,
    limits: ForceLimits,
    catalog: &FftCatalog,
) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let bytes = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        domain,
        limits,
        catalog.backend(),
    )?;
    let rhs =
        SpectralRhs::new_with_catalog_w3(domain, force, config::ADVECTIVE_LIMIT, catalog, bytes)?;
    validate_w3_identity(domain, &rhs)?;
    Ok(rhs)
}

#[cfg(feature = "n384-prep")]
fn validate_w3_identity(
    domain: Domain,
    rhs: &SpectralRhs<CachedReducedForce>,
) -> Result<(), SolverError> {
    let (operator, force) = actual_identities(rhs)?;
    let (expected_operator, expected_force) = expected_identities(domain)?;
    require_identity(operator, expected_operator)?;
    require_identity(force, expected_force)
}

#[cfg(feature = "n384-prep")]
fn actual_identities(
    rhs: &SpectralRhs<CachedReducedForce>,
) -> Result<(W3FftIdentity, W3FftIdentity), SolverError> {
    let operator = rhs.w3_fft_identity().ok_or(SolverError::InvalidPayload)?;
    let force = rhs
        .provider()
        .w3_identity()
        .ok_or(SolverError::InvalidPayload)?;
    Ok((operator, force))
}

#[cfg(feature = "n384-prep")]
fn expected_identities(domain: Domain) -> Result<(W3FftIdentity, W3FftIdentity), SolverError> {
    Ok((rhs_identity(domain)?, force_identity()?))
}

#[cfg(feature = "n384-prep")]
fn require_identity(actual: W3FftIdentity, expected: W3FftIdentity) -> Result<(), SolverError> {
    if actual != expected {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

#[cfg(feature = "n384-prep")]
fn rhs_identity(domain: Domain) -> Result<W3FftIdentity, SolverError> {
    Ok(W3FftIdentity {
        layout: domain.padded_layout()?,
        backend: FftBackend::RustFft6_4_1AvxFma,
        width: 3,
        mode: W3FftMode::Bidirectional,
        additional_bytes: 9_200_779_136,
    })
}

#[cfg(feature = "n384-prep")]
fn force_identity() -> Result<W3FftIdentity, SolverError> {
    Ok(W3FftIdentity {
        layout: Layout::new([config::M; 3])?,
        backend: FftBackend::RustFft6_4_1AvxFma,
        width: 3,
        mode: W3FftMode::Forward,
        additional_bytes: force_w3_additional_bytes(),
    })
}

#[cfg(all(feature = "n384-prep", not(feature = "n384-m512-piecewise-cadv33")))]
const fn force_w3_additional_bytes() -> usize {
    1_827_942_144
}

#[cfg(feature = "n384-m512-piecewise-cadv33")]
const fn force_w3_additional_bytes() -> usize {
    4_318_334_720
}

fn new_observer(catalog: &FftCatalog) -> Result<ReducedObserver, SolverError> {
    let domain = config::domain()?;
    let samples = Layout::new([config::OBSERVER_M; 3])?;
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

#[cfg(all(test, feature = "n384-prep"))]
mod tests {
    use super::*;

    #[test]
    fn exact_w3_identity_values_accept_and_any_change_refuses() {
        let domain = config::domain().unwrap();
        let expected = rhs_identity(domain).unwrap();
        require_identity(expected, expected).unwrap();
        let changed = W3FftIdentity {
            additional_bytes: expected.additional_bytes - 1,
            ..expected
        };
        assert_eq!(
            require_identity(changed, expected),
            Err(SolverError::InvalidPayload)
        );
        assert_eq!(force_identity().unwrap().mode, W3FftMode::Forward);
    }
}
