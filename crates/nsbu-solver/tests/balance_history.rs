//! Pending quadrature and compensated arithmetic survive a diagnostic-history snapshot.
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory, norms::Norms},
    domain::TickClock,
    SolverError,
};

fn clock(t: u128) -> TickClock {
    TickClock::restore(0, 100, t, 100 - t).unwrap()
}
fn sample(work: f64) -> BalanceSample {
    BalanceSample {
        norms: Norms {
            l2: 0.0,
            h1: 0.0,
            vorticity_l2: 0.0,
            divergence_l2: 0.0,
        },
        energy: 0.0,
        enstrophy: 0.0,
        energy_dissipation: 0.0,
        forcing_work: work,
        stretching: work,
        enstrophy_dissipation: 0.0,
        vorticity_forcing: work,
    }
}

#[test]
fn pending_samples_and_compensation_survive_a_snapshot() {
    let mut history = BalanceHistory::new(clock(0), sample(0.0), 7).unwrap();
    assert_eq!(history.samples(), 1);
    assert_eq!(history.integral().unwrap().energy_rhs, 0.0);
    for (start, work) in [(0, 7.5e15), (2, 0.75)] {
        history = history.with_sample(clock(start + 1), sample(work)).unwrap();
        assert!(history.has_pending_midpoint());
        assert_eq!(history.clock(), clock(start + 1));
        assert_eq!(history.integral().unwrap_err(), SolverError::InvalidPayload);
        history = history.with_sample(clock(start + 2), sample(0.0)).unwrap();
        assert!(!history.has_pending_midpoint());
    }
    let snapshot = history;
    let midpoint = snapshot.with_sample(clock(5), sample(-7.5e15)).unwrap();
    let pending_snapshot = midpoint;
    let restored = pending_snapshot.with_sample(clock(6), sample(0.0)).unwrap();
    let uninterrupted = history
        .with_sample(clock(5), sample(-7.5e15))
        .unwrap()
        .with_sample(clock(6), sample(0.0))
        .unwrap();
    for result in [restored, uninterrupted] {
        let integral = result.integral().unwrap();
        assert_eq!(result.samples(), 7);
        assert_eq!(result.clock(), clock(6));
        assert_eq!(integral.energy_rhs, 1.0);
        assert_eq!(integral.enstrophy_rhs, 2.0);
        assert_eq!(integral.energy_defect, -1.0);
        assert_eq!(integral.enstrophy_defect, -2.0);
        assert_eq!(
            result.with_sample(clock(7), sample(0.0)).unwrap_err(),
            SolverError::ResourceLimit
        );
    }
    assert_eq!(snapshot.samples(), 5);
    assert_eq!(pending_snapshot.samples(), 6);
}

#[test]
fn malformed_samples_and_clocks_preserve_the_prior_history() {
    assert_eq!(
        BalanceHistory::new(clock(1), sample(0.0), 3).unwrap_err(),
        SolverError::InvalidClock
    );
    assert_eq!(
        BalanceHistory::new(clock(0), sample(0.0), 2).unwrap_err(),
        SolverError::ResourceLimit
    );
    let history = BalanceHistory::new(clock(0), sample(0.0), 5).unwrap();
    for bad in [
        clock(0),
        TickClock::restore(1, 100, 1, 99).unwrap(),
        TickClock::restore(0, 101, 1, 100).unwrap(),
    ] {
        assert_eq!(
            history.with_sample(bad, sample(0.0)).unwrap_err(),
            SolverError::InvalidClock
        );
    }
    for bad in [
        sample(f64::NAN),
        sample(f64::INFINITY),
        BalanceSample {
            energy: -1.0,
            ..sample(0.0)
        },
        BalanceSample {
            norms: Norms {
                l2: f64::INFINITY,
                ..sample(0.0).norms
            },
            ..sample(0.0)
        },
    ] {
        assert_eq!(
            history.with_sample(clock(1), bad).unwrap_err(),
            SolverError::InvalidPayload
        );
        assert_eq!(
            BalanceHistory::new(clock(0), bad, 3).unwrap_err(),
            SolverError::InvalidPayload
        );
    }
    let midpoint = history.with_sample(clock(1), sample(0.0)).unwrap();
    assert_eq!(
        midpoint.with_sample(clock(3), sample(0.0)).unwrap_err(),
        SolverError::InvalidClock
    );
    assert_eq!(midpoint.clock(), clock(1));
    assert_eq!(history.samples(), 1);
    let huge = history.with_sample(clock(1), sample(f64::MAX)).unwrap();
    assert_eq!(
        huge.with_sample(clock(2), sample(0.0)).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}

#[test]
fn variable_pair_spans_retain_the_initial_balance_values() {
    let initial = BalanceSample {
        energy: 3.0,
        enstrophy: 5.0,
        ..sample(1.5)
    };
    let mut history = BalanceHistory::new(clock(0), initial, 5).unwrap();
    for t in [1, 2, 4, 6] {
        history = history
            .with_sample(
                clock(t),
                BalanceSample {
                    energy: 3.0 + 1.5 * t as f64,
                    enstrophy: 5.0 + 3.0 * t as f64,
                    ..sample(1.5)
                },
            )
            .unwrap();
    }
    let integral = history.integral().unwrap();
    assert_eq!(integral.energy_rhs, 9.0);
    assert_eq!(integral.enstrophy_rhs, 18.0);
    assert_eq!(integral.energy_defect, 0.0);
    assert_eq!(integral.enstrophy_defect, 0.0);
}

#[test]
fn finite_panel_results_cannot_publish_an_overflowing_cumulative_defect() {
    let mut history = BalanceHistory::new(clock(0), sample(0.0), 5).unwrap();
    for t in 1..=3 {
        let work = if t % 2 == 1 { -0.1875 * f64::MAX } else { 0.0 };
        let point = BalanceSample {
            energy: 0.25 * t as f64 * f64::MAX,
            stretching: 0.0,
            vorticity_forcing: 0.0,
            ..sample(work)
        };
        history = history.with_sample(clock(t), point).unwrap();
    }
    let end = BalanceSample {
        energy: f64::MAX,
        ..sample(0.0)
    };
    assert_eq!(
        history.with_sample(clock(4), end).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    assert_eq!(history.clock(), clock(3));
    assert_eq!(history.samples(), 4);
}
