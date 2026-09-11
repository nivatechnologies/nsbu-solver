//! Transactional accepted-node reconstruction for the exact-v2 owner.

use nsbu_benchmarks::{
    provider::V2Force,
    runtime_force::ForceSettings,
    v2_run::{Origin, Plan, ReconstructedPlan, ReconstructedRun, Run, Settings},
};
use nsbu_solver::{
    diagnostics::conservative::ConservativeWorkspace,
    domain::{Domain, Layout, SpectralState, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{
        forcing::PrescribedForce, indicator::Tolerances, method::Method, trajectory::RunLimits,
    },
    spectral::{modal, transfer},
    Complex64, SolverError,
};

const CAP: usize = 64 * 1024 * 1024;

fn settings(method: Method, tolerance: f64) -> Settings {
    Settings {
        domain: Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method,
            limits: RunLimits {
                endpoint: 384,
                step_ticks: 128,
                maximum_attempts: 3,
            },
            tolerances: Tolerances {
                absolute: [tolerance, 10.0 * tolerance],
                relative: [0.0; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn state_words(state: &SpectralState) -> Vec<[u64; 2]> {
    (0..3)
        .flat_map(|axis| state.component(axis).unwrap())
        .map(|value| [value.re.to_bits(), value.im.to_bits()])
        .collect()
}

fn assert_same_run(default: &Run, reconstructed: &ReconstructedRun) {
    assert_eq!(
        state_words(default.state()),
        state_words(reconstructed.state())
    );
    assert_eq!(default.state().clock(), reconstructed.state().clock());
    assert_eq!(
        default.state().accepted_steps(),
        reconstructed.state().accepted_steps()
    );
    assert_eq!(
        default.history().records().len(),
        reconstructed.history().records().len()
    );
    for (left, right) in default
        .history()
        .records()
        .iter()
        .zip(reconstructed.history().records())
    {
        assert_eq!(
            (left.start, left.outcome, left.sample),
            (right.start, right.outcome, right.sample)
        );
    }
    assert_eq!(default.work(), reconstructed.work());
}

#[test]
fn cm_and_ho_match_default_runs_and_retain_actual_endpoint_rhs_nodes() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let config = settings(method, 1e-5);
        let mut default = Run::from_rest(Plan::from_rest(config, CAP).unwrap()).unwrap();
        let plan = ReconstructedPlan::from_rest(config, CAP).unwrap();
        assert_eq!(plan.observer_samples(), 4);
        let mut reconstructed = ReconstructedRun::from_rest(plan).unwrap();
        assert_eq!(reconstructed.origin(), Origin::InternalFromRest);
        let initial = reconstructed.observer_work();
        let observer_limits = reconstructed.observer().limits_declared();
        assert_eq!(initial.samples, 1);
        assert!(initial.work_units > 0 && initial.work_units <= observer_limits.work_units / 4);
        assert_eq!(
            initial.scalar_transforms,
            observer_limits.scalar_transforms / 4
        );
        assert_eq!(observer_limits.samples, 4);
        assert_eq!(
            observer_limits.modal_visits,
            12 * config.domain.layout().half_len()
        );
        assert_eq!(
            reconstructed.observer().modal_visits(),
            3 * config.domain.layout().half_len()
        );
        assert!(reconstructed.observer().last_accepted_clocks().is_none());

        assert_eq!(default.step().unwrap(), reconstructed.step().unwrap());
        let middle = std::array::from_fn(|axis| default.state().component(axis).unwrap().to_vec());
        assert_eq!(default.step().unwrap(), reconstructed.step().unwrap());
        assert_same_run(&default, &reconstructed);
        assert_eq!(
            reconstructed
                .observer()
                .last_accepted_clocks()
                .unwrap()
                .map(|clock| clock.elapsed()),
            [0, 128, 256]
        );
        assert_eq!(
            reconstructed.observer_work().samples,
            default.observer_work().samples + 1
        );
        assert_eq!(
            reconstructed.observer_work().scalar_transforms,
            default.observer_work().scalar_transforms + initial.scalar_transforms
        );
        assert_eq!(reconstructed.work(), default.work());
        assert_eq!(
            reconstructed.observer().modal_visits(),
            9 * config.domain.layout().half_len()
        );

        let clock = TickClock::restore(-20, 8192, 128, 8064).unwrap();
        let expected_derivative = direct_rhs(config.domain, clock, &middle);
        for axis in 0..3 {
            let mut value = vec![Complex64::new(0.0, 0.0); config.domain.layout().half_len()];
            let mut derivative = value.clone();
            reconstructed
                .observer()
                .reconstruct(clock, axis, &mut value, &mut derivative)
                .unwrap();
            assert_eq!(value, middle[axis]);
            for (actual, expected) in derivative.iter().zip(&expected_derivative[axis]) {
                assert!((*actual - *expected).l1_norm() < 2e-12);
            }
        }
    }
}

#[test]
fn rejection_and_refusal_preserve_rest_state_and_accepted_ring() {
    for mut config in [
        settings(Method::CoxMatthews, 1e-40),
        settings(Method::CoxMatthews, 1e-5),
    ] {
        if config.configuration.tolerances.absolute[0] == 1e-5 {
            config.advective_limit = 1e-30;
        }
        let mut run =
            ReconstructedRun::from_rest(ReconstructedPlan::from_rest(config, CAP).unwrap())
                .unwrap();
        let state = state_words(run.state());
        let observation = run.observer_work();
        let outcome = run.step().unwrap();
        assert!(matches!(
            outcome,
            Outcome::Rejected(_) | Outcome::Refused { .. }
        ));
        assert_eq!(state_words(run.state()), state);
        assert_eq!(run.state().clock().elapsed(), 0);
        assert!(run.observer().last_accepted_clocks().is_none());
        assert_eq!(run.observer_work(), observation);
        assert_eq!(
            run.observer().modal_visits(),
            3 * config.domain.layout().half_len()
        );
        assert_eq!(run.work()[0].observation().samples, 0);
    }
}

#[test]
fn admission_refuses_caps_overflow_and_non_rest_construction_has_no_interface() {
    let valid = settings(Method::CoxMatthews, 1e-5);
    let plan = ReconstructedPlan::from_rest(valid, CAP).unwrap();
    assert!(ReconstructedPlan::from_rest(valid, plan.resources().total() - 1).is_err());
    let mut overflow = valid;
    overflow.configuration.limits.maximum_attempts = usize::MAX;
    assert!(matches!(
        ReconstructedPlan::from_rest(overflow, usize::MAX),
        Err(SolverError::RetryLimit | SolverError::SizeOverflow)
    ));
    assert_eq!(
        ReconstructedRun::from_rest(plan)
            .unwrap()
            .state()
            .clock()
            .elapsed(),
        0
    );
}

fn direct_rhs(
    domain: Domain,
    clock: TickClock,
    velocity: &[Vec<Complex64>; 3],
) -> [Vec<Complex64>; 3] {
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let limits = V2Force::preflight(diagnostic, diagnostic.layout()).unwrap();
    let mut provider = V2Force::new(diagnostic, diagnostic.layout(), limits.storage_bytes).unwrap();
    let m = diagnostic.layout().half_len();
    let mut force: [Vec<Complex64>; 3] = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); m]);
    provider
        .evaluate(clock, limits, force.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    let mut products =
        ConservativeWorkspace::new(domain, ConservativeWorkspace::reservation(domain).unwrap())
            .unwrap();
    let mut conservative: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); m]);
    let mut pressure = vec![Complex64::new(0.0, 0.0); m];
    products
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            force.each_ref().map(Vec::as_slice),
            conservative.each_mut().map(Vec::as_mut_slice),
            &mut pressure,
        )
        .unwrap();
    std::array::from_fn(|axis| {
        let mut rhs = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
        transfer(
            diagnostic.layout(),
            domain.layout(),
            &conservative[axis],
            &mut rhs,
        )
        .unwrap();
        for (index, value) in rhs.iter_mut().enumerate() {
            let position = domain.layout().position(index).unwrap();
            if domain.layout().is_nyquist(position).unwrap() {
                *value = Complex64::new(0.0, 0.0);
            } else {
                let wave =
                    modal::wavevector(domain, domain.layout().mode(position).unwrap()).unwrap();
                *value = -*value
                    - domain.viscosity()
                        * wave.iter().map(|k| k * k).sum::<f64>()
                        * velocity[axis][index];
            }
        }
        rhs
    })
}
