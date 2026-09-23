use super::*;
use nsbu_solver::Complex64;

const TAU: f64 = std::f64::consts::TAU;

fn zero_pair() -> ([Vec<Complex64>; 3], [Vec<Complex64>; 3]) {
    (
        zero_fields(domain([4; 3]).layout()),
        zero_fields(domain([8; 3]).layout()),
    )
}

fn run_core(coarse: &[Vec<Complex64>; 3], fine: &[Vec<Complex64>; 3]) -> pair::CoreMetrics {
    pair::compare_core(
        domain([4; 3]),
        domain([8; 3]),
        std::array::from_fn(|axis| coarse[axis].as_slice()),
        std::array::from_fn(|axis| fine[axis].as_slice()),
    )
    .unwrap()
}

fn common_is_zero(value: &pair::CoreMetrics) -> bool {
    [
        value.common.l2,
        value.common.h1,
        value.common.vorticity_l2,
        value.common.divergence_l2,
    ]
    .iter()
    .all(|entry| *entry == 0.0)
}

/// Fourier mapping: a fine state that contains exactly the coarse modes at
/// the located fine slots has a zero common-band difference and no newly
/// resolved contribution.
#[test]
fn fourier_mapping_copy_has_zero_band_difference() {
    let (mut coarse, mut fine) = zero_pair();
    hermitian_mode(
        domain([4; 3]).layout(),
        &mut coarse,
        [1, 0, 0],
        [
            Complex64::new(0.5, -0.25),
            Complex64::new(-0.125, 0.375),
            Complex64::new(0.0, 0.0625),
        ],
    );
    hermitian_mode(
        domain([4; 3]).layout(),
        &mut coarse,
        [0, 0, 1],
        [
            Complex64::new(0.25, 0.25),
            Complex64::new(0.0, -0.5),
            Complex64::new(-0.0625, 0.0),
        ],
    );
    for (axis, component) in coarse.iter_mut().enumerate() {
        component[0] = Complex64::new(0.75 + 0.0625 * (axis + 1) as f64, 0.0);
    }
    copy_coarse_to_fine(domain([4; 3]), domain([8; 3]), &coarse, &mut fine);
    let metrics = run_core(&coarse, &fine);
    for value in [
        metrics.full.l2,
        metrics.full.h1,
        metrics.full.vorticity_l2,
        metrics.full.divergence_l2,
        metrics.common.l2,
        metrics.common.h1,
        metrics.newly_resolved.l2,
        metrics.newly_resolved.h1,
    ] {
        assert_eq!(value, 0.0);
    }
    assert_eq!(metrics.mean_error, [0.0; 3]);
    assert!(metrics.fine_absolute.l2 > 0.0);
}

fn axis_pair_expectation(amplitude: f64, imaginary: f64, mode: i32) -> (f64, f64, f64) {
    let magnitude_squared = amplitude * amplitude + imaginary * imaginary;
    let l2_squared = 2.0 * magnitude_squared;
    let wave_squared = (TAU * f64::from(mode)) * (TAU * f64::from(mode));
    (
        l2_squared.sqrt(),
        (l2_squared * (1.0 + wave_squared)).sqrt(),
        (l2_squared * wave_squared).sqrt(),
    )
}

/// A difference confined to one common-band mode yields the exact analytic
/// L2, H1 and divergence channels on the common band only.
#[test]
fn common_band_h1_matches_the_closed_form() {
    let (coarse, mut fine) = zero_pair();
    hermitian_mode(
        domain([8; 3]).layout(),
        &mut fine,
        [1, 0, 0],
        [
            Complex64::new(0.5, 0.25),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    );
    let metrics = run_core(&coarse, &fine);
    let (l2, h1, divergence) = axis_pair_expectation(0.5, 0.25, 1);
    close(metrics.common.l2, l2);
    close(metrics.common.h1, h1);
    close(metrics.common.divergence_l2, divergence);
    assert_eq!(metrics.common.vorticity_l2, 0.0);
    assert_eq!(metrics.newly_resolved.l2, 0.0);
    assert_eq!(metrics.newly_resolved.h1, 0.0);
    assert_eq!(metrics.full.l2, metrics.common.l2);
    assert_eq!(metrics.full.h1, metrics.common.h1);
    assert_eq!(metrics.mean_error, [0.0; 3]);
    close(metrics.fine_absolute.h1, h1);
}

/// A fine mode outside the strict coarse band contributes exactly to the
/// newly-resolved shell and nothing to the common band.
#[test]
fn newly_resolved_shell_is_exact_and_isolated() {
    let (coarse, mut fine) = zero_pair();
    hermitian_mode(
        domain([8; 3]).layout(),
        &mut fine,
        [3, 0, 0],
        [
            Complex64::new(0.25, -0.125),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    );
    let metrics = run_core(&coarse, &fine);
    assert!(common_is_zero(&metrics));
    let (l2, h1, divergence) = axis_pair_expectation(0.25, -0.125, 3);
    close(metrics.newly_resolved.l2, l2);
    close(metrics.newly_resolved.h1, h1);
    close(metrics.newly_resolved.divergence_l2, divergence);
    assert_eq!(metrics.newly_resolved.vorticity_l2, 0.0);
    assert_eq!(metrics.full.l2, metrics.newly_resolved.l2);
    assert_eq!(metrics.full.h1, metrics.newly_resolved.h1);
}

/// The full-fine-band norm decomposes quadratically over the disjoint common
/// and newly resolved bands, including the mean mode.
#[test]
fn norm_decomposition_is_quadratically_exact() {
    let (coarse, mut fine) = zero_pair();
    hermitian_mode(
        domain([8; 3]).layout(),
        &mut fine,
        [1, 0, 0],
        [
            Complex64::new(0.0, 0.5),
            Complex64::new(-0.25, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    );
    hermitian_mode(
        domain([8; 3]).layout(),
        &mut fine,
        [2, 0, 0],
        [
            Complex64::new(0.125, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-0.0625, 0.0),
        ],
    );
    fine[0][0] = Complex64::new(0.25, 0.0);
    let metrics = run_core(&coarse, &fine);
    for channel in [
        (
            metrics.full.l2,
            metrics.common.l2,
            metrics.newly_resolved.l2,
        ),
        (
            metrics.full.h1,
            metrics.common.h1,
            metrics.newly_resolved.h1,
        ),
        (
            metrics.full.vorticity_l2,
            metrics.common.vorticity_l2,
            metrics.newly_resolved.vorticity_l2,
        ),
        (
            metrics.full.divergence_l2,
            metrics.common.divergence_l2,
            metrics.newly_resolved.divergence_l2,
        ),
    ] {
        let (full, common, shell) = channel;
        assert!(full > 0.0 && common > 0.0 && shell > 0.0);
        close(full * full, common * common + shell * shell);
    }
    close(metrics.mean_error[0], 0.25);
    assert_eq!(metrics.mean_error[1], 0.0);
    assert_eq!(metrics.mean_error[2], 0.0);
    // The mode-2 pair is outside the strict coarse band of a 4^3 grid.
    assert!(metrics.newly_resolved.h1 > 0.0);
}

#[test]
fn comparison_refuses_reversed_grid_order() {
    let (coarse, fine) = zero_pair();
    let error = pair::compare_core(
        domain([8; 3]),
        domain([4; 3]),
        std::array::from_fn(|axis| fine[axis].as_slice()),
        std::array::from_fn(|axis| coarse[axis].as_slice()),
    )
    .unwrap_err();
    assert!(error.contains("InvalidDomain"), "{error}");
}

#[test]
fn comparison_refuses_nonhermitian_and_nyquist_artifacts() {
    let (coarse, mut fine) = zero_pair();
    let layout = domain([8; 3]).layout();
    let index = layout.index([1, 0, 0]).unwrap();
    fine[0][index] = Complex64::new(0.5, 0.5);
    let error = run_core_error(&coarse, &fine);
    assert!(error.contains("InvalidSpectrum"), "{error}");

    let (coarse, mut fine) = zero_pair();
    let index = layout.index([4, 0, 0]).unwrap();
    fine[0][index] = Complex64::new(0.5, 0.0);
    let error = run_core_error(&coarse, &fine);
    assert!(error.contains("InvalidSpectrum"), "{error}");
}

fn run_core_error(coarse: &[Vec<Complex64>; 3], fine: &[Vec<Complex64>; 3]) -> String {
    pair::compare_core(
        domain([4; 3]),
        domain([8; 3]),
        std::array::from_fn(|axis| coarse[axis].as_slice()),
        std::array::from_fn(|axis| fine[axis].as_slice()),
    )
    .unwrap_err()
}

#[test]
fn mean_only_difference_reports_mean_error() {
    let (mut coarse, mut fine) = zero_pair();
    coarse[0][0] = Complex64::new(1.0, 0.0);
    fine[0][0] = Complex64::new(1.25, 0.0);
    let metrics = run_core(&coarse, &fine);
    close(metrics.mean_error[0], 0.25);
    close(metrics.common.l2, 0.25);
    close(metrics.common.h1, 0.25);
    assert_eq!(metrics.common.vorticity_l2, 0.0);
    assert_eq!(metrics.common.divergence_l2, 0.0);
    assert_eq!(metrics.newly_resolved.l2, 0.0);
}
