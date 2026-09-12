//! Independent trigonometric derivatives, complete-band retention and reusable refusal paths.
use nsbu_solver::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    domain::{Domain, Layout},
    Complex64,
};

fn domain() -> Domain {
    Domain::new([8; 3], [2.0, 3.0, 4.0], 1.0).unwrap()
}
fn zero(domain: Domain) -> Vec<Complex64> {
    vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]
}
fn orders() -> [[u8; 3]; 10] {
    [
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
    ]
}

#[test]
fn all_first_and_second_partials_match_an_independent_physical_wave() {
    let domain = domain();
    let samples = Layout::new([12, 16, 12]).unwrap();
    let mut input = zero(domain);
    input[0] = Complex64::new(2.0, 0.0);
    input[domain.layout().locate([3, -2, 1]).unwrap().0] = Complex64::new(0.4, -0.7);
    let before = input.clone();
    let mut workspace = DerivativeWorkspace::new(domain, samples, 1 << 20).unwrap();
    for order in orders() {
        let derivative = Derivative::new(order).unwrap();
        assert_eq!(derivative.orders(), order);
        let result = workspace.sample(&input, derivative).unwrap();
        assert_eq!(result.layout, samples);
        assert_eq!(result.derivative, derivative);
        for (index, &value) in result.values.iter().enumerate() {
            let [x, y, z] = result.point(index).unwrap();
            let phase = std::f64::consts::TAU * (3.0 * x / 2.0 - 2.0 * y / 3.0 + z / 4.0);
            let expected = physical_partial(phase, order);
            assert!(
                (value - expected).abs() < 2e-11,
                "{order:?} {index}: {value} {expected}"
            );
        }
        assert!(result.point(samples.real_len()).is_err());
    }
    assert_eq!(input, before);
}

// Direct differentiation of 2 + 0.8*cos(theta) + 1.4*sin(theta).
fn physical_partial(phase: f64, order: [u8; 3]) -> f64 {
    let wave = std::f64::consts::TAU;
    let frequency = [1.5 * wave, -2.0 * wave / 3.0, wave / 4.0];
    let total = order.into_iter().sum::<u8>();
    let scale = frequency
        .into_iter()
        .zip(order)
        .map(|(k, power)| k.powi(i32::from(power)))
        .product::<f64>();
    match total {
        0 => 2.0 + 0.8 * phase.cos() + 1.4 * phase.sin(),
        1 => scale * (-0.8 * phase.sin() + 1.4 * phase.cos()),
        2 => -scale * (0.8 * phase.cos() + 1.4 * phase.sin()),
        _ => panic!("fixture has only two derivatives"),
    }
}

#[test]
fn real_plane_high_modes_and_mean_are_not_dropped_or_realigned() {
    let domain = domain();
    let mut input = zero(domain);
    let coefficient = Complex64::new(0.25, 0.125);
    input[domain.layout().locate([-3, 2, 0]).unwrap().0] = coefficient;
    input[domain.layout().locate([3, -2, 0]).unwrap().0] = coefficient.conj();
    input[0] = Complex64::new(-3.0, 0.0);
    let mut workspace = DerivativeWorkspace::new(domain, domain.layout(), 1 << 20).unwrap();
    let output = workspace
        .sample(&input, Derivative::new([0; 3]).unwrap())
        .unwrap();
    for (i, &value) in output.values.iter().enumerate() {
        let [x, y, _] = output.point(i).unwrap();
        let phase = std::f64::consts::TAU * (-3.0 * x / 2.0 + 2.0 * y / 3.0);
        assert!((value - (-3.0 + 0.5 * phase.cos() - 0.25 * phase.sin())).abs() < 2e-14);
    }
    let vertical = workspace
        .sample(&input, Derivative::new([0, 0, 1]).unwrap())
        .unwrap();
    assert!(vertical.values.iter().all(|&v| v == 0.0));
}

#[test]
fn admitted_roundoff_on_real_plane_is_projected_before_differentiation() {
    let domain = domain();
    let samples = Layout::new([12, 16, 12]).unwrap();
    let mut input = zero(domain);
    let mode = [3, -2, 0];
    let partner = [-3, 2, 0];
    let a = Complex64::new(0.4 + 2.0e-13, -0.7 - 3.0e-13);
    let b = Complex64::new(0.4 - 2.0e-13, 0.7 - 3.0e-13);
    input[domain.layout().locate(mode).unwrap().0] = a;
    input[domain.layout().locate(partner).unwrap().0] = b;
    input[0] = Complex64::new(-3.0, 4.0e-13);
    let before = input.clone();
    let projected = Complex64::new(a.re.midpoint(b.re), a.im.midpoint(-b.im));
    let mut workspace = DerivativeWorkspace::new(domain, samples, 1 << 20).unwrap();
    for order in [[0, 0, 0], [1, 0, 0], [1, 1, 0]] {
        let result = workspace
            .sample(&input, Derivative::new(order).unwrap())
            .unwrap();
        for (index, &value) in result.values.iter().enumerate() {
            let [x, y, _] = result.point(index).unwrap();
            let phase = std::f64::consts::TAU * (3.0 * x / 2.0 - 2.0 * y / 3.0);
            let wave = [
                3.0 * std::f64::consts::TAU / 2.0,
                -2.0 * std::f64::consts::TAU / 3.0,
                0.0,
            ];
            let mut coefficient = projected;
            for (frequency, count) in wave.into_iter().zip(order) {
                for _ in 0..count {
                    coefficient *= Complex64::new(0.0, frequency);
                }
            }
            let pair = 2.0 * (coefficient.re * phase.cos() - coefficient.im * phase.sin());
            let expected = if order == [0, 0, 0] { pair - 3.0 } else { pair };
            let allowance = 128.0
                * f64::EPSILON
                * (1.0 + expected.abs() + coefficient.re.hypot(coefficient.im));
            assert!((value - expected).abs() <= allowance, "{order:?} {index}");
        }
    }
    assert_eq!(input, before);
}

#[test]
fn orders_grid_backend_and_memory_are_admitted_before_allocation() {
    assert!(Derivative::new([255, 255, 255]).is_err());
    assert!(Derivative::new([1, 1, 1]).is_err());
    let domain = domain();
    assert!(DerivativeWorkspace::reservation(domain, Layout::new([4, 8, 8]).unwrap()).is_err());
    assert!(DerivativeWorkspace::reservation(domain, Layout::new([10; 3]).unwrap()).is_err());
    let bytes = DerivativeWorkspace::reservation(domain, domain.layout()).unwrap();
    assert!(DerivativeWorkspace::new(domain, domain.layout(), bytes - 1).is_err());
    assert!(DerivativeWorkspace::new(domain, domain.layout(), bytes).is_ok());
}

#[test]
fn malformed_spectra_are_refused_and_scratch_recovers_without_input_changes() {
    let domain = domain();
    let good = zero(domain);
    let mut workspace = DerivativeWorkspace::new(domain, domain.layout(), 1 << 20).unwrap();
    let derivative = Derivative::new([1, 1, 0]).unwrap();
    let mut invalid = [good.clone(), good.clone(), good.clone()];
    invalid[0][0].re = f64::NAN;
    invalid[1][domain.layout().locate([1, 0, 0]).unwrap().0].im = 1.0;
    // Native storage includes the forbidden Nyquist plane even though locate excludes it.
    invalid[2][domain.layout().index([4, 0, 0]).unwrap()].re = 1.0;
    assert!(workspace.sample(&good[..1], derivative).is_err());
    for input in invalid {
        let words: Vec<_> = input
            .iter()
            .map(|z| (z.re.to_bits(), z.im.to_bits()))
            .collect();
        assert!(workspace.sample(&input, derivative).is_err());
        assert_eq!(
            input
                .iter()
                .map(|z| (z.re.to_bits(), z.im.to_bits()))
                .collect::<Vec<_>>(),
            words
        );
        assert!(workspace
            .sample(&good, derivative)
            .unwrap()
            .values
            .iter()
            .all(|&v| v == 0.0));
    }
}

#[test]
fn derivative_overflow_is_refused_and_later_calls_start_with_clean_scratch() {
    let domain = domain();
    let mut input = zero(domain);
    input[domain.layout().locate([3, 0, 1]).unwrap().0] = Complex64::new(f64::MAX / 2.0, 0.0);
    let before = input.clone();
    let mut workspace = DerivativeWorkspace::new(domain, domain.layout(), 1 << 20).unwrap();
    assert!(workspace
        .sample(&input, Derivative::new([2, 0, 0]).unwrap())
        .is_err());
    assert_eq!(input, before);
    input.fill(Complex64::new(0.0, 0.0));
    input[0].re = 2.0;
    assert!(workspace
        .sample(&input, Derivative::new([0; 3]).unwrap())
        .unwrap()
        .values
        .iter()
        .all(|&v| v == 2.0));
}

#[test]
fn vertical_derivative_of_huge_real_plane_pair_is_exact_zero() {
    let domain = domain();
    let mut input = zero(domain);
    let value = Complex64::new(f64::MAX / 4.0, -f64::MAX / 8.0);
    input[domain.layout().locate([3, -2, 0]).unwrap().0] = value;
    input[domain.layout().locate([-3, 2, 0]).unwrap().0] = value.conj();
    let before = input.clone();
    let mut workspace = DerivativeWorkspace::new(domain, domain.layout(), 1 << 20).unwrap();
    let samples = workspace
        .sample(&input, Derivative::new([1, 0, 1]).unwrap())
        .unwrap();
    assert!(samples.values.iter().all(|&value| value == 0.0));
    assert_eq!(input, before);
}
