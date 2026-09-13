use super::*;
use nsbu_solver::{
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    integrators::kernel::RightHandSide,
    Complex64,
};

#[test]
fn small_actual_force_and_rhs_parallel_factories_match_legacy_w3_bits() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6; 3]).unwrap();
    let catalog_cap = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_cap).unwrap();
    let sampling_workers = 3;

    let make_force = |parallel: bool| {
        let limits = CachedReducedForce::preflight(
            domain,
            samples,
            sampling_workers,
            backend,
            true,
            parallel.then_some(FFT_WORKERS),
        )
        .unwrap();
        CachedReducedForce::new(
            domain,
            samples,
            sampling_workers,
            &catalog,
            limits.storage_bytes,
            true,
            parallel.then_some(FFT_WORKERS),
        )
        .unwrap()
    };

    let baseline_force = make_force(false);
    let parallel_force = make_force(true);
    assert!(baseline_force.parallel_fft_identity().is_none());
    let force_identity = parallel_force.parallel_fft_identity().unwrap();
    assert_eq!(force_identity.helper_workers, FFT_WORKERS);
    assert_eq!(force_identity.persistent_callers, 3);

    let baseline_limits = baseline_force.limits().unwrap();
    let parallel_limits = parallel_force.limits().unwrap();
    let baseline_cap = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        domain,
        baseline_limits,
        backend,
    )
    .unwrap();
    let parallel_cap = SpectralRhs::<CachedReducedForce>::reservation_with_parallel_w3_fft_backend(
        domain,
        parallel_limits,
        backend,
        FFT_WORKERS,
    )
    .unwrap();
    let declared_parallel_cap =
        SpectralRhs::<DeclaredForce>::reservation_with_parallel_w3_fft_backend(
            domain,
            parallel_limits,
            backend,
            FFT_WORKERS,
        )
        .unwrap();
    assert!(matches!(
        SpectralRhs::new_with_catalog_parallel_w3(
            domain,
            DeclaredForce(parallel_limits),
            ADVECTIVE_LIMIT,
            &catalog,
            FFT_WORKERS,
            declared_parallel_cap - 1,
        ),
        Err(SolverError::ResourceLimit)
    ));

    let mut baseline = SpectralRhs::new_with_catalog_w3(
        domain,
        baseline_force,
        ADVECTIVE_LIMIT,
        &catalog,
        baseline_cap,
    )
    .unwrap();
    let mut parallel = SpectralRhs::new_with_catalog_parallel_w3(
        domain,
        parallel_force,
        ADVECTIVE_LIMIT,
        &catalog,
        FFT_WORKERS,
        parallel_cap,
    )
    .unwrap();
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    baseline.begin_attempt(clock, TICKS).unwrap();
    parallel.begin_attempt(clock, TICKS).unwrap();
    let state: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut left: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut right = left.clone();
    let state_refs = [&state[0][..], &state[1][..], &state[2][..]];
    let [left0, left1, left2] = &mut left;
    baseline
        .evaluate(state_refs, clock, [left0, left1, left2])
        .unwrap();
    let [right0, right1, right2] = &mut right;
    parallel
        .evaluate(state_refs, clock, [right0, right1, right2])
        .unwrap();
    assert!(left
        .iter()
        .flatten()
        .zip(right.iter().flatten())
        .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits()));
}

#[test]
fn exact_api_reservation_and_one_byte_under_are_closed() {
    let a = admit(BASE_CAP, false).unwrap();
    assert_eq!(
        a.plan.classes(),
        [
            38_805_700_608,
            32_614_907_904,
            10_899_947_520,
            3_233_808_384,
            29_362_480,
            91_810_835_512,
            30_182_212_728,
            65_552,
        ]
    );
    assert_eq!(a.force.storage_bytes, 29_155_601_864);
    assert_eq!(
        TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead(),
        16
    );
    assert_eq!(a.plan.total(), BASE_CAP);
    assert!(matches!(
        admit(BASE_CAP - 1, false),
        Err(SolverError::ResourceLimit)
    ));
    let parallel_cap = admit(usize::MAX, true).unwrap().plan.total();
    assert!(parallel_cap > BASE_CAP);
    assert_eq!(
        admit(parallel_cap, true).unwrap().plan.total(),
        parallel_cap
    );
    assert!(matches!(
        admit(parallel_cap - 1, true),
        Err(SolverError::ResourceLimit)
    ));
}

#[test]
fn candidate_hash_uses_component_order_and_little_endian_binary64_words() {
    let a = [Complex64::new(1.0, -2.0), Complex64::new(0.5, -0.0)];
    let b = [Complex64::new(3.25, 4.5)];
    assert_eq!(
        coefficient_sha256_components([&a, &b, &[]]).unwrap(),
        "7756e4b9b28758c9eecd0ba6dfb48d0072624596dfb7b365b34be0bd51968e59"
    );
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
    let a = admit(BASE_CAP, false).unwrap();
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
            true,
            None
        )),
        Err(SolverError::ResourceLimit)
    ));
    let limits =
        CachedReducedForce::preflight(domain, samples, WORKERS, backend, true, None).unwrap();
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
    std::env::remove_var("NSBU_RUN_N512_M512_PARALLEL_FFT_ONE_ATTEMPT");
    let path = std::env::temp_dir().join(format!("p10-n512-m512-refusal-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    assert!(gated_run(&path, true)
        .unwrap_err()
        .contains("missing reviewed"));
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
