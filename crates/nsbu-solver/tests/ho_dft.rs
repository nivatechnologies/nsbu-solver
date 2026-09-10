//! Independent full-band direct-DFT fixture for a committed HO full/two-half attempt.
mod etd_fixture_support;
use etd_fixture_support::CyclicSource;
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::{
        attempt::AttemptWorkspace, forcing::PrescribedForce, indicator::Tolerances, method::Method,
        rhs::SpectralRhs, transaction::commit_candidate,
    },
    Complex64,
};

#[test]
fn ho_transaction_matches_independent_fine_step_spectrum() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let source = CyclicSource(domain);
    let source_bytes =
        SpectralRhs::<CyclicSource>::reservation(domain, source.limits().unwrap()).unwrap();
    let method = Method::HochbruckOstermann;
    let diagnostics = AttemptWorkspace::reservation_with_method(domain, method).unwrap();
    let plan = etd_fixture_support::plan(domain, source_bytes, diagnostics);
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    let (mut state, mut candidate) = etd_fixture_support::payloads(plan, clock);
    let mut rhs = SpectralRhs::new(domain, source, 0.3, source_bytes).unwrap();
    let full = full_step(&state, &mut rhs);
    let mut workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
    assert_eq!(workspace.method(), method);
    let report = workspace
        .try_advance(
            &state,
            &mut candidate,
            8,
            Tolerances {
                absolute: [1.0; 2],
                relative: [0.0; 2],
            },
            &mut rhs,
        )
        .unwrap();
    assert_eq!(report.rhs_calls, 15);
    assert_eq!(rhs.consumption(), [15, 15 * 151, 150]);
    commit_candidate(plan, &mut state, &mut candidate, report.accepted.unwrap()).unwrap();
    assert_eq!(state.clock().elapsed(), 8);
    assert_eq!(state.accepted_steps(), 1);
    etd_fixture_support::compare(
        domain.layout(),
        &full,
        &state,
        include_str!("fixtures/ho-step.tsv"),
    );
}

fn full_step(
    state: &nsbu_solver::domain::SpectralState,
    rhs: &mut dyn nsbu_solver::integrators::kernel::RightHandSide,
) -> [Vec<Complex64>; 3] {
    use nsbu_solver::integrators::{
        ho_coefficients::HoCoefficients, ho_kernel::HoWorkspace, time::binary_duration,
    };
    let domain = state.plan().domain();
    let clock = state.clock();
    let dt = binary_duration(8, clock.exponent()).unwrap();
    let mut kernel = HoWorkspace::new(domain.layout().half_len(), state.plan().total()).unwrap();
    let tables: Vec<_> = etd_fixture_support::arguments(domain, dt)
        .into_iter()
        .map(|z| HoCoefficients::new(z).unwrap())
        .collect();
    let mut output =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let stages = clock.stages(8).unwrap();
    rhs.begin_attempt(clock, 8).unwrap();
    let [a, b, c] = &mut output;
    kernel
        .step(
            etd_fixture_support::components(state),
            [stages[0], stages[2], stages[4]],
            dt,
            &tables,
            rhs,
            [a, b, c],
        )
        .unwrap();
    output
}
