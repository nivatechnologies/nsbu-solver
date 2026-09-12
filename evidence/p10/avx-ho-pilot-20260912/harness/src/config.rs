use crate::{cache::CachedReducedForce, observer::ReducedObserver, timed_rhs::TimedRhs};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan},
    integrators::{
        attempt::AttemptWorkspace, forcing::ForceLimits, method::Method, rhs::SpectralRhs,
    },
    spectral::{FftBackend, FftCatalog},
    SolverError,
};

pub const N: usize = 384;
pub const M: usize = 384;
pub const WORKERS: usize = 32;
pub const TICKS: u128 = 64;
pub const ADVECTIVE_LIMIT: f64 = 3.3;
pub const CAP: usize = 224 * 1024 * 1024 * 1024;
pub const OUTPUT_CAP: usize = 1024 * 1024;
pub const OVERHEAD: usize = 1_310_736;
pub const EXPECTED_ATTEMPT: usize = 25_954_616_512;
pub const EXPECTED_TOTAL: usize = 198_987_813_712;
pub const EXPECTED_INTEGRATION_WORK: usize = 110_846_361_675;
pub const EXPECTED_OBSERVER_FORCE_WORK: usize = 58_435_043_328;
pub const IDENTITY: &str = concat!(
    "p10-avx-n384-ho-rest-proposal-observer-timing-v1;",
    "numerical_base=aed49b7d7874a0a720dee88b65ba180c7286fa65;",
    "w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;",
    "backend=rustfft-6.4.1-avx-avx2-fma;method=hochbruck-ostermann;",
    "retained=384;force_samples=384;observer_force_samples=768;",
    "observer_conservative=768;sampling_workers=32;rhs_w3_workers=3;",
    "provider_w3_workers=3;ticks=64;advective_limit=3.3;",
    "absolute_tolerances=1e-5,1e-4;relative_tolerances=1e-5,1e-5;",
    "scope=one-rest-attempt-then-unscheduled-proposal-observer;",
    "publication=timing-json-only-no-state-no-balance-no-frontier;",
    "host=local;contention=old-n256-plus-matched-n256;",
    "execution_cap=240518168576;output_cap=1048576;timeout_seconds=1200;",
    "external_stop=pgid-watchdog-v2-starttime-cmdline-deadline"
);

#[derive(Clone, Copy)]
pub struct Reservations {
    pub catalog: usize,
    pub force: ForceLimits,
    pub rhs: usize,
    pub attempt: usize,
    pub observer: usize,
}

pub struct Admission {
    pub resources: ResourcePlan,
    pub reservations: Reservations,
}

pub fn domain() -> Result<Domain, SolverError> {
    Domain::new([N; 3], [1.0; 3], 1.0)
}

pub fn tolerances() -> nsbu_solver::integrators::indicator::Tolerances {
    nsbu_solver::integrators::indicator::Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    }
}

pub fn preflight(cap: usize) -> Result<Admission, SolverError> {
    let domain = domain()?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog = FftCatalog::reservation(backend)?;
    let force =
        CachedReducedForce::preflight(domain, Layout::new([M; 3])?, WORKERS, backend, true)?;
    let rhs =
        SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(domain, force, backend)?;
    let attempt = AttemptWorkspace::reservation_with_method(domain, Method::HochbruckOstermann)?;
    let observer = ReducedObserver::preflight(domain, Layout::new([2 * M; 3])?, WORKERS, backend)?;
    let reservations = Reservations {
        catalog,
        force,
        rhs,
        attempt,
        observer,
    };
    let diagnostics = attempt
        .checked_add(observer)
        .ok_or(SolverError::SizeOverflow)?;
    let resources = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: catalog,
            force: rhs,
            diagnostics,
            overhead: OVERHEAD,
        },
        cap,
        Epoch(0),
    )?;
    validate(&resources, reservations)?;
    Ok(Admission {
        resources,
        reservations,
    })
}

fn validate(resources: &ResourcePlan, r: Reservations) -> Result<(), SolverError> {
    let integration = r
        .force
        .work_units
        .checked_mul(Method::HochbruckOstermann.rhs_calls())
        .ok_or(SolverError::SizeOverflow)?;
    let observer = observer_force_work()?;
    if r.attempt != EXPECTED_ATTEMPT
        || resources.total() != EXPECTED_TOTAL
        || integration != EXPECTED_INTEGRATION_WORK
        || observer != EXPECTED_OBSERVER_FORCE_WORK
        || TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead() > OVERHEAD
    {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

fn observer_force_work() -> Result<usize, SolverError> {
    let source = domain()?;
    let diagnostic =
        nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(source)?;
    Ok(nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        Layout::new([2 * M; 3])?,
        WORKERS,
        FftBackend::RustFft6_4_1AvxFma,
    )?.work_units)
}

pub fn report(a: &Admission) {
    let r = a.reservations;
    println!(
        "preflight source={} identity={} catalog_bytes={} rhs_bytes={} attempt_bytes={} observer_bytes={} overhead_bytes={} total_bytes={} cap_bytes={} output_cap_bytes={} integration_work_bound={} observer_force_work_bound={} rhs_calls=15 physical_force_evaluations=5 cache_hits=10 operator_w3_triplets=45 force_w3_triplets=5 pressure_scalar_ffts=15 qualification=timing_only",
        env!("RUN_SOURCE"), IDENTITY, r.catalog, r.rhs, r.attempt, r.observer, OVERHEAD,
        a.resources.total(), CAP, OUTPUT_CAP, EXPECTED_INTEGRATION_WORK,
        EXPECTED_OBSERVER_FORCE_WORK,
    );
    println!("rhs_timer_identity={}", crate::timed_rhs::IDENTITY);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_resource_and_work_contract_is_closed() {
        let a = preflight(CAP).unwrap();
        assert_eq!(a.resources.total(), EXPECTED_TOTAL);
        assert_eq!(a.reservations.attempt, EXPECTED_ATTEMPT);
        assert_eq!(Method::HochbruckOstermann.rhs_calls(), 15);
        assert_eq!(
            a.resources.classes(),
            [
                16_392_388_608,
                13_759_414_272,
                4_602_396_672,
                1_366_032_384,
                29_362_480,
                38_807_118_904,
                124_029_789_656,
                1_310_736,
            ]
        );
        assert!(matches!(
            preflight(EXPECTED_TOTAL - 1),
            Err(SolverError::ResourceLimit)
        ));
    }

    #[test]
    fn identity_refuses_trajectory_semantics() {
        for required in [
            "method=hochbruck-ostermann",
            "ticks=64",
            "no-state-no-balance-no-frontier",
            "timeout_seconds=1200",
        ] {
            assert!(IDENTITY.contains(required));
        }
    }
}
