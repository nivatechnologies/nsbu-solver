//! Actual CM/HO rest trajectories feed physical derivatives without reference assignments.
use nsbu_benchmarks::smooth_run::{ReconstructedPlan, ReconstructedRun};
use nsbu_solver::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    domain::{Domain, Layout, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};

fn evolve(method: Method) -> (ReconstructedRun, DerivativeWorkspace) {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let clock = TickClock::from_rest(-16, 512).unwrap();
    let configuration = Configuration {
        method,
        limits: RunLimits {
            endpoint: 128,
            step_ticks: 16,
            maximum_attempts: 8,
        },
        tolerances: Tolerances {
            absolute: [1e-2; 2],
            relative: [0.0; 2],
        },
    };
    let layout = Layout::new([8; 3]).unwrap();
    let samples = DerivativeWorkspace::reservation(domain, layout).unwrap();
    let plan = ReconstructedPlan::from_rest(domain, clock, configuration, 9, 0.3, 1 << 24).unwrap();
    let total = plan.resources().total().checked_add(samples).unwrap();
    assert!(total < 1 << 24);
    let mut run = ReconstructedRun::from_rest(
        domain,
        clock,
        configuration,
        9,
        0.3,
        plan.resources().total(),
    )
    .unwrap();
    let workspace = DerivativeWorkspace::new(domain, layout, samples).unwrap();
    for _ in 0..8 {
        assert!(matches!(run.step().unwrap(), Outcome::Committed(_)));
    }
    (run, workspace)
}

#[test]
fn independent_rest_evolution_retains_nonzero_derivative_errors_for_both_methods() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let (run, mut workspace) = evolve(method);
        let state = run.state();
        assert_eq!(state.clock().elapsed(), 128);
        assert_eq!(run.history().controller().committed(), 8);
        let before: Vec<_> = (0..3)
            .map(|c| state.component(c).unwrap().to_vec())
            .collect();
        let errors = measure(&run, &mut workspace);
        assert!(errors.into_iter().all(|v| v > 0.0 && v < 1e-9));
        for (component, values) in before.iter().enumerate() {
            assert_eq!(state.component(component).unwrap(), values);
        }
        println!("{method:?} actual physical derivative maximum errors: value={} gradient={} hessian={}; accepted_pde_windows=0", errors[0], errors[1], errors[2]);
    }
}

fn measure(run: &ReconstructedRun, workspace: &mut DerivativeWorkspace) -> [f64; 3] {
    let mut maximum = [0.0_f64; 3];
    let orders = [
        [0, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [2, 0, 0],
        [0, 2, 0],
        [0, 0, 2],
        [1, 1, 0],
        [1, 0, 1],
        [0, 1, 1],
    ];
    for component in 0..3 {
        let input = run.state().component(component).unwrap();
        for order in orders {
            let result = workspace
                .sample(input, Derivative::new(order).unwrap())
                .unwrap();
            let total = order.into_iter().sum::<u8>() as usize;
            for (index, &value) in result.values.iter().enumerate() {
                let point = result.point(index).unwrap();
                let reference = analytical(component, point, order);
                maximum[total] = maximum[total].max((value - reference).abs());
            }
        }
    }
    maximum
}

fn analytical(component: usize, point: [f64; 3], order: [u8; 3]) -> f64 {
    let coordinate = (component + 1) % 3;
    if order
        .iter()
        .enumerate()
        .any(|(axis, &power)| axis != coordinate && power != 0)
    {
        return 0.0;
    }
    let frequency = [13.0_f64, 17.0, 19.0][component];
    let amplitude = (frequency / 512.0).sin();
    let k = std::f64::consts::TAU;
    let phase = k * point[coordinate];
    match order[coordinate] {
        0 => amplitude * phase.sin(),
        1 => amplitude * k * phase.cos(),
        2 => -amplitude * k * k * phase.sin(),
        _ => panic!("fixture requests at most two derivatives"),
    }
}
