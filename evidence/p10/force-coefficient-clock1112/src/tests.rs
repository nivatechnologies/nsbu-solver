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
    assert_eq!(shell_index([127, 0, 0]), Some(5));
    assert_eq!(shell_index([128, 0, 0]), Some(6));
    assert_eq!(shell_index([191, 0, 0]), Some(6));
    assert_eq!(shell_index([192, 0, 0]), None);
}

#[test]
fn independently_validated_exact_modal_fixtures_match_all_channels_and_shells() {
    let fixtures = [
        (
            [2, 0, 0],
            [[0, 0], [3, 4], [0, 0]],
            2.0,
            [2, 0, 0],
            0,
            [50, 250, 200, 0],
        ),
        (
            [3, 0, 0],
            [[2, -1], [0, 0], [0, 0]],
            1.0,
            [3, 0, 0],
            0,
            [5, 50, 0, 45],
        ),
        (
            [1, 2, 3],
            [[1, 1], [2, -1], [-1, 2]],
            2.0,
            [1, 2, 3],
            0,
            [24, 360, 278, 58],
        ),
        (
            [0, 0, 0],
            [[1, -2], [3, 0], [-1, 1]],
            1.0,
            [0, 0, 0],
            0,
            [16, 16, 0, 0],
        ),
        (
            [24, 0, 0],
            [[0, 0], [1, 0], [0, 0]],
            2.0,
            [24, 0, 0],
            1,
            [2, 1154, 1152, 0],
        ),
        (
            [96, 0, 0],
            [[0, 0], [1, 0], [0, 0]],
            2.0,
            [96, 0, 0],
            5,
            [2, 18434, 18432, 0],
        ),
        (
            [128, 0, 0],
            [[0, 0], [1, 0], [0, 0]],
            2.0,
            [128, 0, 0],
            6,
            [2, 32770, 32768, 0],
        ),
        (
            [5, -7, 0],
            [[1, 2], [-2, 1], [3, -1]],
            2.0,
            [5, -7, 0],
            0,
            [40, 3000, 2220, 740],
        ),
    ];
    let mut transverse = None;
    for (k, u, weight, mode, expected_shell, expected) in fixtures {
        let mut sums = ShellSums::default();
        sums.push(
            k.map(f64::from),
            u.map(|[re, im]| Complex64::new(f64::from(re), f64::from(im))),
            weight,
        )
        .unwrap();
        let norm = sums.finish().unwrap();
        assert_close(norm.l2, (expected[0] as f64).sqrt());
        assert_close(norm.h1, (expected[1] as f64).sqrt());
        assert_close(norm.vorticity_l2, (expected[2] as f64).sqrt());
        assert_close(norm.divergence_l2, (expected[3] as f64).sqrt());
        assert_eq!(shell_index(mode), Some(expected_shell));
        if mode == [2, 0, 0] {
            transverse = Some(norm);
        }
    }

    let empty = ShellSums::default().finish().unwrap();
    let transverse = transverse.unwrap();
    let shells = [transverse, transverse, empty, empty, empty, empty, empty];
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
