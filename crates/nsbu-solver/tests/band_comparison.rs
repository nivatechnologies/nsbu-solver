//! Full-band negative controls, physical derivatives, multiplicities and stable norm arithmetic.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan, domain::Domain, Complex64, SolverError,
};

#[test]
fn a_new_divergence_free_mode_cannot_hide_behind_common_band_agreement() {
    for line in include_str!("fixtures/full-band-norms.tsv").lines() {
        let expected: Vec<f64> = line.split('\t').map(|v| v.parse().unwrap()).collect();
        let lengths = [expected[0], 1.0, 1.0];
        let coarse = Domain::new([4; 3], lengths, 1.0).unwrap();
        let fine = Domain::new([8; 3], lengths, 1.0).unwrap();
        let left = zero(coarse.layout());
        let mut right = zero(fine.layout());
        mode(
            fine.layout(),
            &mut right,
            [2, 0, 0],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, -0.5),
                Complex64::new(0.0, 0.0),
            ],
        );
        let plan = ComparisonPlan::new(coarse, fine).unwrap();
        assert_eq!(plan.work_units(), 1424);
        let report = plan.compare(slices(&left), slices(&right)).unwrap();
        close(report.full.l2, expected[1]);
        close(report.full.h1, expected[2]);
        close(report.full.vorticity_l2, expected[3]);
        close(report.full.l2 * expected[0].sqrt(), expected[4]);
        assert_eq!(report.full.divergence_l2, 0.0);
        assert_eq!(report.common.l2, 0.0);
        assert_eq!(report.full, report.newly_resolved);
        assert_eq!(report.mean_error, [0.0; 3]);
    }
}

#[test]
fn mean_and_transverse_and_longitudinal_errors_have_separate_norms() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let left = zero(domain.layout());
    let mut right = zero(domain.layout());
    for (values, mean) in right.iter_mut().zip([1.0, -2.0, 0.5]) {
        values[0] = Complex64::new(mean, 0.0);
    }
    let z = Complex64::new(0.0, 0.0);
    mode(
        domain.layout(),
        &mut right,
        [0, 0, 1],
        [Complex64::new(0.0, -1.0), z, z],
    );
    mode(
        domain.layout(),
        &mut right,
        [1, 0, 0],
        [Complex64::new(0.0, -1.5), z, z],
    );
    let plan = ComparisonPlan::new(domain, domain).unwrap();
    let report = plan.compare(slices(&left), slices(&right)).unwrap();
    close(report.full.l2, 11.75_f64.sqrt());
    close(
        report.full.h1,
        (11.75 + 26.0 * std::f64::consts::PI.powi(2)).sqrt(),
    );
    close(
        report.full.vorticity_l2,
        2.0_f64.sqrt() * std::f64::consts::TAU,
    );
    close(
        report.full.divergence_l2,
        4.5_f64.sqrt() * std::f64::consts::TAU,
    );
    assert_eq!(report.full, report.common);
    assert_eq!(report.newly_resolved.l2, 0.0);
    assert_eq!(report.mean_error, [1.0, -2.0, 0.5]);
    let same = plan.compare(slices(&right), slices(&right)).unwrap();
    assert_eq!(same.full.l2, 0.0);
    assert_eq!(same.mean_error, [0.0; 3]);
    let scaled = right
        .clone()
        .map(|values| values.into_iter().map(|value| 1.5 * value).collect());
    close(
        plan.compare(slices(&right), slices(&scaled))
            .unwrap()
            .full
            .h1,
        report.full.h1 / 2.0,
    );
}

#[test]
fn oblique_complex_modes_exercise_all_derivative_contractions() {
    let coarse = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let fine = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let left = zero(coarse.layout());
    for (vector, divergence, curl) in [
        ([1.0, -2.0, 0.0], 0.0_f64, 30.0_f64),
        ([1.0, 2.0, 3.0], 49.0, 35.0),
    ] {
        let mut right = zero(fine.layout());
        mode(
            fine.layout(),
            &mut right,
            [2, 1, 1],
            vector.map(|v| Complex64::new(0.25 * v, 0.5 * v)),
        );
        let report = ComparisonPlan::new(coarse, fine)
            .unwrap()
            .compare(slices(&left), slices(&right))
            .unwrap();
        let factor = 0.625_f64;
        let k = std::f64::consts::TAU;
        let l2_squared = factor * vector.into_iter().map(|v| v * v).sum::<f64>();
        close(report.full.l2, l2_squared.sqrt());
        close(report.full.h1, (l2_squared * (1.0 + 6.0 * k * k)).sqrt());
        close(report.full.divergence_l2, (factor * divergence).sqrt() * k);
        close(report.full.vorticity_l2, (factor * curl).sqrt() * k);
    }
}

#[test]
fn finite_large_mean_norms_do_not_overflow_when_squares_would() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let left = zero(domain.layout());
    let mut right = zero(domain.layout());
    right[0][0] = Complex64::new(1e200, 0.0);
    let plan = ComparisonPlan::new(domain, domain).unwrap();
    let report = plan.compare(slices(&left), slices(&right)).unwrap();
    assert_eq!(report.full.l2, 1e200);
    assert_eq!(report.full.h1, 1e200);
    right[1][0] = Complex64::new(f64::MAX, 0.0);
    right[2][0] = right[1][0];
    assert_eq!(
        plan.compare(slices(&left), slices(&right)).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}
