//! Attempt-local exact-v2 force cache contracts and real CM/HO macro equivalence.
use nsbu_benchmarks::runtime_force::{AttemptCacheWork, ForceSettings};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        forcing::PrescribedForce,
        indicator::Tolerances,
        method::Method,
        rhs::SpectralRhs,
        transaction::{commit_candidate, CandidateState},
    },
    Complex64, SolverError,
};

fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}

fn settings() -> ForceSettings {
    ForceSettings {
        samples: Layout::new([4; 3]).unwrap(),
        workers: 0,
    }
}

fn output() -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain().layout().half_len()])
}

fn words(values: &[Vec<Complex64>; 3]) -> Vec<(u64, u64)> {
    values
        .iter()
        .flatten()
        .map(|value| (value.re.to_bits(), value.im.to_bits()))
        .collect()
}

#[test]
fn five_exact_slots_match_fresh_force_and_reset_between_attempts() {
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let stages = clock.stages(128).unwrap();
    let limits = settings().attempt_cache_limits(domain()).unwrap();
    let mut cached = settings()
        .build_attempt_cache(domain(), limits.storage_bytes)
        .unwrap();
    let fresh_limits = settings().limits(domain()).unwrap();
    let mut fresh = settings()
        .build(domain(), fresh_limits.storage_bytes)
        .unwrap();
    let mut actual = output();
    assert!(matches!(
        cached.evaluate(stages[0], limits, actual.each_mut().map(Vec::as_mut_slice)),
        Err(SolverError::ProviderBudgetExceeded)
    ));
    cached.begin_attempt(clock, 128, limits).unwrap();
    for index in [0, 2, 4, 0, 1, 3, 4] {
        let mut expected = output();
        fresh
            .evaluate(
                stages[index],
                fresh_limits,
                expected.each_mut().map(Vec::as_mut_slice),
            )
            .unwrap();
        cached
            .evaluate(
                stages[index],
                limits,
                actual.each_mut().map(Vec::as_mut_slice),
            )
            .unwrap();
        assert_eq!(words(&actual), words(&expected));
    }
    let work = cached.work();
    assert_eq!(
        (work.calls, work.lookups, work.clock_comparisons),
        (7, 7, 21)
    );
    assert_eq!(
        (work.provider_evaluations, work.hits, work.misses),
        (5, 2, 5)
    );
    assert_eq!(work.provider_scalar_transforms, 15);
    assert!(work.provider_work_units > 0);
    assert_eq!(
        work.coefficient_words_copied,
        7 * 3 * domain().layout().half_len()
    );
    let first_epoch = cached.epoch();
    cached.begin_attempt(clock, 128, limits).unwrap();
    cached
        .evaluate(stages[0], limits, actual.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    assert_eq!(cached.epoch(), first_epoch + 1);
    assert_eq!(
        (cached.work().provider_evaluations, cached.work().hits),
        (1, 0)
    );
}

#[test]
fn malformed_calls_and_caps_never_publish_a_slot() {
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let stages = clock.stages(128).unwrap();
    let limits = settings().attempt_cache_limits(domain()).unwrap();
    assert!(matches!(
        settings().build_attempt_cache(domain(), limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let mut cached = settings()
        .build_attempt_cache(domain(), limits.storage_bytes)
        .unwrap();
    let mut actual = output();
    cached.begin_attempt(clock, 128, limits).unwrap();
    cached
        .evaluate(stages[0], limits, actual.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    let mut wrong = limits;
    wrong.work_units -= 1;
    assert_eq!(
        cached.begin_attempt(clock, 128, wrong),
        Err(SolverError::ProviderBudgetExceeded)
    );
    assert!(matches!(
        cached.evaluate(stages[0], limits, actual.each_mut().map(Vec::as_mut_slice)),
        Err(SolverError::ProviderBudgetExceeded)
    ));
    cached.begin_attempt(clock, 128, limits).unwrap();
    let mut short = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); 1]);
    assert!(matches!(
        cached.evaluate(stages[2], limits, short.each_mut().map(Vec::as_mut_slice)),
        Err(SolverError::InvalidPayload)
    ));
    assert_eq!(cached.work(), AttemptCacheWork::default());
    let off_stage = TickClock::restore(-20, 8192, 7, 8185).unwrap();
    assert!(matches!(
        cached.evaluate(off_stage, limits, actual.each_mut().map(Vec::as_mut_slice)),
        Err(SolverError::InvalidClock)
    ));
    assert_eq!(
        (cached.work().calls, cached.work().provider_evaluations),
        (1, 0)
    );
    cached
        .evaluate(stages[2], limits, actual.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    assert_eq!(
        (cached.work().misses, cached.work().provider_evaluations),
        (1, 1)
    );
    let incompatible = TickClock::from_rest(-20, 4096).unwrap();
    cached.begin_attempt(incompatible, 128, limits).unwrap();
    let incompatible_stage = incompatible.stages(128).unwrap()[0];
    for expected_misses in 1..=2 {
        assert!(matches!(
            cached.evaluate(
                incompatible_stage,
                limits,
                actual.each_mut().map(Vec::as_mut_slice)
            ),
            Err(SolverError::InvalidClock)
        ));
        assert_eq!(cached.work().provider_evaluations, expected_misses);
        assert_eq!(cached.work().misses, expected_misses);
        assert_eq!(cached.work().hits, 0);
    }
    cached.begin_attempt(clock, 128, limits).unwrap();
    for _ in 0..15 {
        cached
            .evaluate(stages[0], limits, actual.each_mut().map(Vec::as_mut_slice))
            .unwrap();
    }
    assert!(matches!(
        cached.evaluate(stages[0], limits, actual.each_mut().map(Vec::as_mut_slice)),
        Err(SolverError::ProviderBudgetExceeded)
    ));
    assert_eq!(
        (cached.work().provider_evaluations, cached.work().hits),
        (1, 14)
    );
}

fn plan(method: Method, force: usize) -> ResourcePlan {
    ResourcePlan::new(
        domain(),
        ExtraStorage {
            fft: 0,
            force,
            diagnostics: AttemptWorkspace::reservation_with_method(domain(), method).unwrap(),
            overhead: 4096,
        },
        1 << 26,
        Epoch(0),
    )
    .unwrap()
}

fn uncached_attempt(method: Method) -> Vec<(u64, u64)> {
    let force_limits = settings().limits(domain()).unwrap();
    let reservation = SpectralRhs::<nsbu_benchmarks::runtime_force::RunForce>::reservation(
        domain(),
        force_limits,
    )
    .unwrap();
    let resources = plan(method, reservation);
    let force = settings()
        .build(domain(), force_limits.storage_bytes)
        .unwrap();
    let rhs = SpectralRhs::new(domain(), force, 0.3, reservation).unwrap();
    accepted_attempt(method, resources, rhs).0
}

fn cached_attempt(method: Method) -> (Vec<(u64, u64)>, [usize; 3], AttemptCacheWork) {
    let force_limits = settings().attempt_cache_limits(domain()).unwrap();
    let reservation =
        SpectralRhs::<nsbu_benchmarks::runtime_force::AttemptForceCache>::reservation(
            domain(),
            force_limits,
        )
        .unwrap();
    let resources = plan(method, reservation);
    let force = settings()
        .build_attempt_cache(domain(), force_limits.storage_bytes)
        .unwrap();
    let rhs = SpectralRhs::new(domain(), force, 0.3, reservation).unwrap();
    let (bits, rhs) = accepted_attempt(method, resources, rhs);
    (bits, rhs.consumption(), rhs.provider().work())
}

fn accepted_attempt<F: PrescribedForce>(
    method: Method,
    resources: ResourcePlan,
    mut rhs: SpectralRhs<F>,
) -> (Vec<(u64, u64)>, SpectralRhs<F>) {
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let mut state = SpectralState::from_rest(resources, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(resources, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(resources, method).unwrap();
    let attempt = workspace
        .try_advance(
            &state,
            &mut candidate,
            128,
            Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
            &mut rhs,
        )
        .unwrap();
    commit_candidate(
        resources,
        &mut state,
        &mut candidate,
        attempt.accepted.unwrap(),
    )
    .unwrap();
    let bits = (0..3)
        .flat_map(|axis| state.component(axis).unwrap())
        .map(|value| (value.re.to_bits(), value.im.to_bits()))
        .collect();
    (bits, rhs)
}

#[test]
fn real_cm_and_ho_macro_attempts_preserve_every_state_word() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let expected = uncached_attempt(method);
        let (actual, consumption, work) = cached_attempt(method);
        assert_eq!(actual, expected);
        assert_eq!(consumption[0], method.rhs_calls());
        assert_eq!(work.provider_evaluations, 5);
        assert_eq!(work.misses, 5);
        assert_eq!(work.hits, method.rhs_calls() - 5);
        assert_eq!(work.calls, method.rhs_calls());
        assert_eq!(
            consumption[1],
            work.provider_work_units + work.coefficient_words_copied + work.clock_comparisons
        );
        assert_eq!(
            consumption[2],
            work.provider_scalar_transforms + 10 * method.rhs_calls()
        );
    }
}

#[test]
fn rejected_macro_does_not_reuse_stale_force_slots() {
    let method = Method::CoxMatthews;
    let limits = settings().attempt_cache_limits(domain()).unwrap();
    let reservation =
        SpectralRhs::<nsbu_benchmarks::runtime_force::AttemptForceCache>::reservation(
            domain(),
            limits,
        )
        .unwrap();
    let resources = plan(method, reservation);
    let force = settings()
        .build_attempt_cache(domain(), limits.storage_bytes)
        .unwrap();
    let mut rhs = SpectralRhs::new(domain(), force, 0.3, reservation).unwrap();
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let state = SpectralState::from_rest(resources, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(resources, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(resources, method).unwrap();
    let rejected = workspace
        .try_advance(
            &state,
            &mut candidate,
            128,
            Tolerances {
                absolute: [1e-40; 2],
                relative: [0.0; 2],
            },
            &mut rhs,
        )
        .unwrap();
    assert!(rejected.accepted.is_none());
    assert_eq!(rhs.provider().work().provider_evaluations, 5);
    let first_epoch = rhs.provider().epoch();
    let accepted = workspace
        .try_advance(
            &state,
            &mut candidate,
            128,
            Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
            &mut rhs,
        )
        .unwrap();
    assert!(accepted.accepted.is_some());
    assert_eq!(rhs.provider().epoch(), first_epoch + 1);
    assert_eq!(rhs.provider().work().provider_evaluations, 5);
}
