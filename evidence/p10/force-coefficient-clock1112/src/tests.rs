use super::*;

#[test]
fn max_mode_shells_are_disjoint_and_cover_strict_n384() {
    assert_eq!(shell_index([0, 0, 0]), Some(0));
    assert_eq!(shell_index([23, -23, 1]), Some(0));
    assert_eq!(shell_index([24, 0, 0]), Some(1));
    assert_eq!(shell_index([31, 0, 0]), Some(1));
    assert_eq!(shell_index([32, 0, 0]), Some(2));
    assert_eq!(shell_index([95, 0, 0]), Some(4));
    assert_eq!(shell_index([96, 0, 0]), Some(5));
    assert_eq!(shell_index([191, 0, 0]), Some(5));
    assert_eq!(shell_index([192, 0, 0]), None);
}

#[test]
fn exact_transverse_mode_norms_and_orthogonal_shell_combination_match() {
    let mut sums = ShellSums::default();
    sums.push(
        [2.0, 0.0, 0.0],
        [
            Complex64::new(0.0, 0.0),
            Complex64::new(3.0, 4.0),
            Complex64::new(0.0, 0.0),
        ],
        2.0,
    )
    .unwrap();
    let norm = sums.finish().unwrap();
    assert_close(norm.l2, 50.0_f64.sqrt());
    assert_close(norm.h1, 250.0_f64.sqrt());
    assert_close(norm.vorticity_l2, 200.0_f64.sqrt());
    assert_eq!(norm.divergence_l2, 0.0);

    let zero = ShellSums::default().finish().unwrap();
    let shells = [norm, norm, zero, zero, zero, zero];
    let full = combine(&shells).unwrap();
    assert_close(full.l2, 100.0_f64.sqrt());
    assert_close(full.h1, 500.0_f64.sqrt());
    assert_close(full.vorticity_l2, 400.0_f64.sqrt());
    assert_eq!(full.divergence_l2, 0.0);
}

#[test]
fn projection_and_stokes_factor_have_expected_gradient_and_zero_mode_behavior() {
    let k = [2.0, 0.0, 0.0];
    assert_eq!(
        modal::project(
            k,
            [
                Complex64::new(3.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
            ],
        )
        .unwrap(),
        [Complex64::new(0.0, 0.0); 3]
    );
    let time = CLOCK as f64 * 2_f64.powi(EXPONENT);
    assert_eq!(
        stokes_factor([0.0; 3], time).unwrap().to_bits(),
        time.to_bits()
    );
    assert!(stokes_factor(k, time).unwrap() > 0.0);
}

#[test]
fn dry_run_rejects_unreviewed_sample_grids_before_allocation() {
    assert_eq!(
        sample(640, "unused", true),
        Err(SolverError::InvalidPayload)
    );
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 8.0 * f64::EPSILON * expected.max(1.0));
}
