//! Production FFT/CM steps compared with independent 120-digit direct-DFT evolution from rest.
use nsbu_solver::domain::{Domain, TickClock};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace,
    coefficients::CmCoefficients,
    forcing::PrescribedForce,
    indicator::Tolerances,
    kernel::{CmWorkspace, RightHandSide},
    rhs::SpectralRhs,
    time::binary_duration,
    transaction::commit_candidate,
};
use nsbu_solver::Complex64;
mod etd_fixture_support;
use etd_fixture_support::CyclicSource;

#[test]
fn full_and_two_half_steps_match_independent_full_band_fixtures() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let source_bytes =
        SpectralRhs::<CyclicSource>::reservation(domain, CyclicSource(domain).limits().unwrap())
            .unwrap();
    let plan = etd_fixture_support::plan(
        domain,
        source_bytes,
        AttemptWorkspace::reservation(domain).unwrap(),
    );
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    let (mut state, mut candidate) = etd_fixture_support::payloads(plan, clock);
    let mut rhs = SpectralRhs::new(domain, CyclicSource(domain), 0.3, source_bytes).unwrap();
    let mut kernel = CmWorkspace::new(layout.half_len(), plan.total()).unwrap();
    let dt = binary_duration(8, clock.exponent()).unwrap();
    let coefficients: Vec<_> = etd_fixture_support::arguments(domain, dt)
        .into_iter()
        .map(|z| CmCoefficients::new(z).unwrap())
        .collect();
    let mut full =
        std::array::from_fn::<_, 3, _>(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
    let stages = clock.stages(8).unwrap();
    let [a, b, c] = &mut full;
    rhs.begin_attempt(clock, 8).unwrap();
    kernel
        .step(
            etd_fixture_support::components(&state),
            [stages[0], stages[2], stages[4]],
            dt,
            &coefficients,
            &mut rhs,
            [a, b, c],
        )
        .unwrap();
    let mut work = AttemptWorkspace::new(plan).unwrap();
    let result = work
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
    commit_candidate(plan, &mut state, &mut candidate, result.accepted.unwrap()).unwrap();
    etd_fixture_support::compare(layout, &full, &state, include_str!("fixtures/cm-step.tsv"));
}
