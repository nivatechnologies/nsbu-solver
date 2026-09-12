//! Independent signed-sum controls for self-conjugate-plane derivative staging.
use nsbu_solver::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    domain::{Domain, Layout},
    Complex64,
};

fn domain() -> Domain {
    Domain::new([8; 3], [2.0, 3.0, 4.0], 1.0).unwrap()
}

#[test]
fn admitted_roundoff_matches_projected_signed_pair_on_a_padded_grid() {
    let domain = domain();
    let samples = Layout::new([12, 16, 12]).unwrap();
    let mut input = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
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
fn vertical_derivative_of_huge_real_plane_pair_is_exact_zero() {
    let domain = domain();
    let mut input = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
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
