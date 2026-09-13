use super::*;
use nsbu_solver::{
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    Complex64,
};

#[test]
fn exact_api_reservation_and_one_byte_under_are_closed() {
    let a = admit(CAP).unwrap();
    assert_eq!(
        a.plan.classes(),
        [
            38_805_700_608,
            32_614_907_904,
            10_899_947_520,
            3_233_808_384,
            29_362_480,
            122_443_729_976,
            30_182_212_728,
            65_552,
        ]
    );
    assert_eq!(a.force.storage_bytes, 59_788_791_240);
    assert_eq!(
        TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead(),
        16
    );
    assert_eq!(a.plan.total(), CAP);
    assert!(matches!(admit(CAP - 1), Err(SolverError::ResourceLimit)));
}

#[test]
fn sole_from_rest_interval_has_exact_clock_and_five_cache_nodes() {
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    assert_eq!(clock.elapsed(), 0);
    assert_eq!(
        clock.stages(TICKS).unwrap().map(TickClock::elapsed),
        [0, 16, 32, 48, 64]
    );
    assert_eq!(Method::CoxMatthews.rhs_calls(), 12);
    assert_eq!(ADVECTIVE_LIMIT, 3.3);
    assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
    assert_eq!(tolerances().relative, [1e-5; 2]);
}

#[test]
fn each_constructor_refuses_its_one_byte_under_budget_before_large_allocation() {
    let a = admit(CAP).unwrap();
    let (domain, samples, backend) = geometry().unwrap();
    assert!(matches!(
        no_allocations(|| FftCatalog::new(backend, a.catalog - 1)),
        Err(SolverError::ResourceLimit)
    ));
    let catalog = FftCatalog::new(backend, a.catalog).unwrap();
    assert!(matches!(
        no_allocations(|| CachedReducedForce::new(
            domain,
            samples,
            WORKERS,
            &catalog,
            a.force.storage_bytes - 1,
            true
        )),
        Err(SolverError::ResourceLimit)
    ));
    let limits = CachedReducedForce::preflight(domain, samples, WORKERS, backend, true).unwrap();
    let declared_rhs =
        SpectralRhs::<DeclaredForce>::reservation_with_w3_fft_backend(domain, limits, backend)
            .unwrap();
    assert!(matches!(
        no_allocations(|| SpectralRhs::new_with_catalog_w3(
            domain,
            DeclaredForce(limits),
            3.3,
            &catalog,
            declared_rhs - 1
        )),
        Err(SolverError::ResourceLimit)
    ));
    let under = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: a.catalog,
            force: a.rhs,
            diagnostics: a.attempt - 1,
            overhead: OVERHEAD,
        },
        usize::MAX,
        Epoch(0),
    )
    .unwrap();
    assert!(matches!(
        no_allocations(|| AttemptWorkspace::new_with_method(under, Method::CoxMatthews)),
        Err(SolverError::ResourceLimit)
    ));
}

#[test]
fn execution_is_explicitly_gated_before_output_creation() {
    std::env::remove_var("NSBU_RUN_N512_M768_ONE_ATTEMPT");
    let path = std::env::temp_dir().join(format!("p10-n512-m768-refusal-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    assert!(gated_run(&path).unwrap_err().contains("missing reviewed"));
    assert!(!path.exists());
}

struct DeclaredForce(ForceLimits);

fn no_allocations<T>(f: impl FnOnce() -> T) -> T {
    let region = Region::new(GLOBAL);
    let value = f();
    let change = region.change();
    assert_eq!(
        (
            change.allocations,
            change.deallocations,
            change.reallocations
        ),
        (0, 0, 0)
    );
    value
}

impl PrescribedForce for DeclaredForce {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.0)
    }

    fn evaluate(
        &mut self,
        _: TickClock,
        _: ForceLimits,
        _: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        unreachable!("one-byte-under RHS construction must refuse before evaluation")
    }
}
