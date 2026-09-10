//! Independent polynomial reconstruction controls; no PDE acceptance is inferred.
use nsbu_solver::{
    diagnostics::hermite::HermiteWeights, domain::TickClock, Complex64, SolverError,
};

fn clock(ticks: u128) -> TickClock {
    TickClock::restore(-8, 1024, ticks, 1024 - ticks).unwrap()
}

fn apply(weights: [f64; 6], samples: [f64; 6]) -> f64 {
    weights.into_iter().zip(samples).map(|(a, b)| a * b).sum()
}

#[test]
fn every_quintic_is_reproduced_in_value_and_physical_derivative() {
    // A shifted interval prevents a hidden dependence on absolute time or unit duration.
    let nodes = [clock(64), clock(128), clock(192)];
    for degree in 0_i32..=5 {
        let samples = std::array::from_fn(|index| {
            let t = nodes[index % 3].elapsed() as f64 / 256.0;
            if index < 3 {
                t.powi(degree)
            } else if degree == 0 {
                0.0
            } else {
                f64::from(degree) * t.powi(degree - 1)
            }
        });
        for ticks in (64..=192).step_by(8) {
            let weights = HermiteWeights::at(nodes, clock(ticks)).unwrap();
            let t = ticks as f64 / 256.0;
            let expected_derivative = if degree == 0 {
                0.0
            } else {
                f64::from(degree) * t.powi(degree - 1)
            };
            assert!((apply(weights.value, samples) - t.powi(degree)).abs() < 1e-13);
            assert!((apply(weights.derivative, samples) - expected_derivative).abs() < 1e-13);
            let spectra = samples.map(|v| [Complex64::new(v, -2.0 * v)]);
            let mut value = [Complex64::new(0.0, 0.0)];
            let mut derivative = value;
            weights
                .apply(
                    spectra.each_ref().map(|v| v.as_slice()),
                    &mut value,
                    &mut derivative,
                )
                .unwrap();
            assert!((value[0].re - t.powi(degree)).abs() < 1e-13);
            assert!((value[0].im + 2.0 * t.powi(degree)).abs() < 2e-13);
            assert!((derivative[0].re - expected_derivative).abs() < 1e-13);
            assert!((derivative[0].im + 2.0 * expected_derivative).abs() < 2e-13);
        }
    }
}

#[test]
fn reconstruction_refuses_incomplete_or_nonfinite_payloads() {
    let weights = HermiteWeights::at([clock(0), clock(128), clock(256)], clock(64)).unwrap();
    let zero = [Complex64::new(0.0, 0.0)];
    let mut value = zero;
    let mut derivative = zero;
    assert_eq!(
        weights.apply([&zero; 6], &mut value, &mut []).unwrap_err(),
        SolverError::InvalidPayload
    );
    let mut samples = [&zero[..]; 6];
    samples[3] = &[];
    assert_eq!(
        weights
            .apply(samples, &mut value, &mut derivative)
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    for invalid in [
        Complex64::new(f64::NAN, 0.0),
        Complex64::new(0.0, f64::INFINITY),
    ] {
        let input = [invalid];
        assert_eq!(
            weights
                .apply([&input; 6], &mut value, &mut derivative)
                .unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
    }
    let mut invalid = weights;
    invalid.value[0] = f64::NAN;
    assert_eq!(
        invalid
            .apply([&zero; 6], &mut value, &mut derivative)
            .unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    let huge = [Complex64::new(f64::MAX, f64::MAX)];
    assert_eq!(
        weights
            .apply([&huge; 6], &mut value, &mut derivative)
            .unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}

#[test]
fn node_only_residuals_hide_a_defect_revealed_between_nodes() {
    for span in [256_u128, 128] {
        let nodes = [clock(0), clock(span / 2), clock(span)];
        let samples = std::array::from_fn(|index| {
            let t = nodes[index % 3].elapsed() as f64 / 256.0;
            if index < 3 {
                t.powi(6)
            } else {
                6.0 * t.powi(5)
            }
        });
        for node in nodes {
            let weights = HermiteWeights::at(nodes, node).unwrap();
            let t = node.elapsed() as f64 / 256.0;
            assert_eq!(apply(weights.derivative, samples) - 6.0 * t.powi(5), 0.0);
        }
        for (numerator, sign) in [(1, 1.0), (3, -1.0)] {
            let probe = clock(numerator * span / 4);
            let weights = HermiteWeights::at(nodes, probe).unwrap();
            let t = probe.elapsed() as f64 / 256.0;
            let defect = apply(weights.derivative, samples) - 6.0 * t.powi(5);
            let expected = sign * 3.0 / 512.0 * (span as f64 / 256.0).powi(5);
            assert_eq!(defect, expected);
        }
    }
}

#[test]
fn inconsistent_or_degenerate_clocks_are_refused() {
    assert_eq!(
        HermiteWeights::at([clock(0); 3], clock(0)).unwrap_err(),
        SolverError::InvalidClock
    );
    let valid = [clock(0), clock(128), clock(256)];
    for nodes in [
        [clock(0), clock(0), clock(0)],
        [clock(256), clock(128), clock(0)],
        [clock(0), clock(0), clock(256)],
        [clock(128), clock(0), clock(256)],
        [clock(0), clock(256), clock(128)],
        [clock(0), clock(64), clock(256)],
    ] {
        assert_eq!(
            HermiteWeights::at(nodes, clock(64)).unwrap_err(),
            SolverError::InvalidClock
        );
    }
    for probe in [
        clock(257),
        TickClock::from_rest(-7, 1024).unwrap(),
        TickClock::from_rest(-8, 512).unwrap(),
    ] {
        assert_eq!(
            HermiteWeights::at(valid, probe).unwrap_err(),
            SolverError::InvalidClock
        );
    }
    assert_eq!(
        HermiteWeights::at([clock(64), clock(128), clock(192)], clock(63)).unwrap_err(),
        SolverError::InvalidClock
    );
}
