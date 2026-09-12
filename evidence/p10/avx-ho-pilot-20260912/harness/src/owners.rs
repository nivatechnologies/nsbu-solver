use crate::{cache::CachedReducedForce, config, observer::ReducedObserver, timed_rhs::TimedRhs};
use nsbu_solver::{
    domain::{Epoch, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace, method::Method, rhs::SpectralRhs, transaction::CandidateState,
    },
    spectral::{FftBackend, FftCatalog, W3FftIdentity, W3FftMode},
    SolverError,
};

pub struct Owners {
    pub resources: ResourcePlan,
    pub state: SpectralState,
    pub candidate: CandidateState,
    pub attempt: AttemptWorkspace,
    pub rhs: TimedRhs<SpectralRhs<CachedReducedForce>>,
    pub observer: ReducedObserver,
}

pub fn construct(admission: config::Admission) -> Result<Owners, SolverError> {
    let resources = admission.resources;
    let domain = resources.domain();
    let catalog = FftCatalog::new(FftBackend::RustFft6_4_1AvxFma, resources.classes()[4])?;
    let samples = Layout::new([config::M; 3])?;
    let limits =
        CachedReducedForce::preflight(domain, samples, config::WORKERS, catalog.backend(), true)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        config::WORKERS,
        &catalog,
        limits.storage_bytes,
        true,
    )?;
    let rhs_bytes = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        domain,
        limits,
        catalog.backend(),
    )?;
    let rhs = SpectralRhs::new_with_catalog_w3(
        domain,
        force,
        config::ADVECTIVE_LIMIT,
        &catalog,
        rhs_bytes,
    )?;
    validate_w3(domain, &rhs)?;
    let observer_bytes = ReducedObserver::preflight(
        domain,
        Layout::new([2 * config::M; 3])?,
        config::WORKERS,
        catalog.backend(),
    )?;
    let observer = ReducedObserver::new(
        domain,
        Layout::new([2 * config::M; 3])?,
        config::WORKERS,
        &catalog,
        observer_bytes,
    )?;
    let clock = TickClock::from_rest(-20, 8192)?;
    let state = SpectralState::from_rest(resources, clock, Epoch(0))?;
    let candidate = CandidateState::new(resources, clock, Epoch(0))?;
    let attempt = AttemptWorkspace::new_with_method(resources, Method::HochbruckOstermann)?;
    Ok(Owners {
        resources,
        state,
        candidate,
        attempt,
        rhs: TimedRhs::new(rhs),
        observer,
    })
}

fn validate_w3(
    domain: nsbu_solver::domain::Domain,
    rhs: &SpectralRhs<CachedReducedForce>,
) -> Result<(), SolverError> {
    let operator = rhs.w3_fft_identity().ok_or(SolverError::InvalidPayload)?;
    let force = rhs
        .provider()
        .w3_identity()
        .ok_or(SolverError::InvalidPayload)?;
    let expected_operator = W3FftIdentity {
        layout: domain.padded_layout()?,
        backend: FftBackend::RustFft6_4_1AvxFma,
        width: 3,
        mode: W3FftMode::Bidirectional,
        additional_bytes: 9_200_779_136,
    };
    let expected_force = W3FftIdentity {
        layout: Layout::new([config::M; 3])?,
        backend: FftBackend::RustFft6_4_1AvxFma,
        width: 3,
        mode: W3FftMode::Forward,
        additional_bytes: 1_827_942_144,
    };
    if operator != expected_operator || force != expected_force {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

pub fn is_rest(state: &SpectralState) -> Result<bool, SolverError> {
    if state.clock().elapsed() != 0 || state.epoch() != Epoch(0) || state.accepted_steps() != 0 {
        return Ok(false);
    }
    for axis in 0..3 {
        if state
            .component(axis)?
            .iter()
            .any(|v| v.re != 0.0 || v.im != 0.0)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_identity_checks_metadata_and_payload() {
        let d = nsbu_solver::domain::Domain::new([12; 3], [1.0; 3], 1.0).unwrap();
        let p = ResourcePlan::new(
            d,
            nsbu_solver::domain::ExtraStorage {
                fft: 1,
                force: 1,
                diagnostics: 1,
                overhead: 1,
            },
            usize::MAX,
            Epoch(0),
        )
        .unwrap();
        let state = SpectralState::from_rest(p, TickClock::from_rest(-20, 8192).unwrap(), Epoch(0))
            .unwrap();
        assert!(is_rest(&state).unwrap());
    }
}
