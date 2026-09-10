//! Independent polynomial balance quadrature; these synthetic samples do not qualify a trajectory.
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, norms::Norms, quadrature::simpson},
    domain::TickClock,
    SolverError,
};

fn clock(ticks: u128) -> TickClock {
    TickClock::restore(-8, 1024, ticks, 1024 - ticks).unwrap()
}

fn sample(t: f64, degree: i32) -> BalanceSample {
    let rate = f64::from(degree) * t.powi(degree - 1);
    BalanceSample {
        norms: Norms {
            l2: 0.0,
            h1: 0.0,
            vorticity_l2: 0.0,
            divergence_l2: 0.0,
        },
        energy: t.powi(degree),
        enstrophy: 2.0 * t.powi(degree),
        energy_dissipation: 3.0,
        forcing_work: rate + 3.0,
        stretching: rate + 11.0,
        enstrophy_dissipation: 24.0,
        vorticity_forcing: rate + 13.0,
    }
}

#[test]
fn exact_cubic_rates_and_fourth_order_quadrature_refinement_are_distinct() {
    for degree in 1..=5 {
        for parts in [1_u128, 2, 4, 8] {
            let mut energy_error = 0.0;
            let mut enstrophy_error = 0.0;
            let mut energy_integral = 0.0;
            let mut enstrophy_integral = 0.0;
            let span = 256 / parts;
            for part in 0..parts {
                let ticks = [part * span, part * span + span / 2, (part + 1) * span];
                let result = simpson(
                    ticks.map(clock),
                    ticks.map(|tick| sample(tick as f64 / 256.0, degree)),
                )
                .unwrap();
                energy_error += result.energy_defect;
                enstrophy_error += result.enstrophy_defect;
                energy_integral += result.energy_rhs;
                enstrophy_integral += result.enstrophy_rhs;
            }
            let expected = if degree == 5 {
                -1.0 / (24.0 * (parts as f64).powi(4))
            } else {
                0.0
            };
            assert!((energy_error - expected).abs() < 1e-13);
            assert!((enstrophy_error - 2.0 * expected).abs() < 1e-13);
            assert!((energy_integral - (1.0 - expected)).abs() < 1e-13);
            assert!((enstrophy_integral - (2.0 - 2.0 * expected)).abs() < 1e-13);
        }
    }
}

#[test]
fn unresolved_weights_and_nonfinite_balance_terms_are_refused() {
    let nodes = [clock(0), clock(128), clock(256)];
    let mut samples = [sample(0.0, 1), sample(0.5, 1), sample(1.0, 1)];
    samples[1].forcing_work = f64::INFINITY;
    assert_eq!(
        simpson(nodes, samples).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    let tiny = [0, 1, 2].map(|tick| TickClock::restore(-1074, 4, tick, 4 - tick).unwrap());
    assert_eq!(
        simpson(tiny, [sample(0.0, 1); 3]).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    assert_eq!(
        simpson([clock(0); 3], [sample(0.0, 1); 3]).unwrap_err(),
        SolverError::InvalidClock
    );
}

#[test]
fn nonfinite_enstrophy_rates_and_overflowing_integrals_are_refused() {
    let nodes = [0, 512, 1024].map(|tick| TickClock::restore(-8, 2048, tick, 2048 - tick).unwrap());
    for rate in [f64::INFINITY, f64::MAX] {
        let mut samples = [sample(0.0, 1); 3];
        samples[1].stretching = rate;
        assert_eq!(
            simpson(nodes, samples).unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
    }
}
