//! Ordered-hessian algebra and mixed force-space diagnostic coverage.

use super::*;

#[test]
fn ordered_hessian_single_oblique_mode_uses_hermitian_weight_and_domain_volume() {
    let domain = Domain::new([4; 3], [2.0, 3.0, 4.0], 0.01).unwrap();
    let mut values = zero_fields(domain.layout());
    let coefficient = [
        Complex64::new(3.0, 4.0),
        Complex64::new(-1.0, 2.0),
        Complex64::new(0.0, 0.0),
    ];
    hermitian_mode(domain.layout(), &mut values, [1, -1, 1], coefficient);

    let measured =
        measure_ordered_hessian(domain, std::array::from_fn(|axis| values[axis].as_slice()))
            .unwrap();
    let k_squared = std::f64::consts::TAU.powi(2)
        * (1.0 / 2.0_f64.powi(2) + 1.0 / 3.0_f64.powi(2) + 1.0 / 4.0_f64.powi(2));
    let coefficient_squared = coefficient
        .into_iter()
        .map(|value| value.re * value.re + value.im * value.im)
        .sum::<f64>();
    let expected_rms = (2.0 * coefficient_squared * k_squared.powi(2)).sqrt();
    close(measured.rms, expected_rms);
    close(measured.l2, expected_rms * (2.0_f64 * 3.0 * 4.0).sqrt());
}

#[test]
fn ordered_hessian_sums_all_27_independent_component_derivative_entries() {
    let domain = Domain::new([4; 3], [1.0, 2.0, 4.0], 0.01).unwrap();
    let mut values = zero_fields(domain.layout());
    hermitian_mode(
        domain.layout(),
        &mut values,
        [1, 0, 0],
        [
            Complex64::new(1.0, 2.0),
            Complex64::new(3.0, -1.0),
            Complex64::new(-2.0, 0.5),
        ],
    );
    hermitian_mode(
        domain.layout(),
        &mut values,
        [1, -1, 1],
        [
            Complex64::new(0.25, -0.5),
            Complex64::new(-1.5, 0.75),
            Complex64::new(2.0, 0.125),
        ],
    );

    let zeros = zero_fields(domain.layout());
    let screen = screen_ordered_hessian(
        domain,
        std::array::from_fn(|axis| zeros[axis].as_slice()),
        domain,
        std::array::from_fn(|axis| values[axis].as_slice()),
    )
    .unwrap();
    let mut direct_sum = 0.0;
    for index in 0..domain.layout().half_len() {
        let position = domain.layout().position(index).unwrap();
        if domain.layout().is_nyquist(position).unwrap() {
            continue;
        }
        let wave = modal::wavevector(domain, domain.layout().mode(position).unwrap()).unwrap();
        let weight = domain.layout().weight(position).unwrap();
        for value in values.iter().map(|component| component[index]) {
            for first in wave {
                for second in wave {
                    let derivative = value * first * second;
                    direct_sum +=
                        weight * (derivative.re * derivative.re + derivative.im * derivative.im);
                }
            }
        }
    }
    close(screen.difference.rms, direct_sum.sqrt());
    close(
        screen.difference.l2,
        direct_sum.sqrt() * (1.0_f64 * 2.0 * 4.0).sqrt(),
    );
    assert_eq!(screen.relative_to_fine.rms, Some(1.0));
    assert_eq!(screen.relative_to_fine.l2, Some(1.0));
}

#[test]
fn ordered_hessian_matches_small_grid_parseval_reconstruction() {
    let domain = Domain::new([4; 3], [1.0; 3], 0.01).unwrap();
    let mode = [1, -1, 1];
    let coefficient = [
        Complex64::new(0.25, 0.125),
        Complex64::new(-0.375, 0.5),
        Complex64::new(0.2, -0.3),
    ];
    let mut values = zero_fields(domain.layout());
    hermitian_mode(domain.layout(), &mut values, mode, coefficient);
    let measured =
        measure_ordered_hessian(domain, std::array::from_fn(|axis| values[axis].as_slice()))
            .unwrap();

    let wave = modal::wavevector(domain, mode).unwrap();
    let mut physical_sum = 0.0;
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let phase =
                    std::f64::consts::TAU * (x as f64 / 4.0 - y as f64 / 4.0 + z as f64 / 4.0);
                for value in coefficient {
                    let physical = 2.0 * (value.re * phase.cos() - value.im * phase.sin());
                    for first in wave {
                        for second in wave {
                            physical_sum += (first * second * physical).powi(2);
                        }
                    }
                }
            }
        }
    }
    let expected_rms = (physical_sum / 64.0).sqrt();
    close(measured.rms, expected_rms);
    close(measured.l2, expected_rms);
}

#[test]
fn ordered_hessian_scaled_squares_keep_large_finite_modes_finite() {
    let domain = Domain::new([4; 3], [1.0; 3], 0.01).unwrap();
    let mut values = zero_fields(domain.layout());
    hermitian_mode(
        domain.layout(),
        &mut values,
        [1, 0, 1],
        [
            Complex64::new(1e200, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    );
    let measured =
        measure_ordered_hessian(domain, std::array::from_fn(|axis| values[axis].as_slice()))
            .unwrap();
    let expected = 2.0_f64.sqrt() * 1e200 * (2.0 * std::f64::consts::TAU.powi(2));
    assert!(measured.rms.is_finite());
    close(measured.rms, expected);
    close(measured.l2, expected);
}

#[test]
fn mixed_force_space_algebra_preserves_sign_closure_and_band_splits() {
    let coarse_domain = Domain::new([4; 3], [1.0; 3], 0.01).unwrap();
    let fine_domain = Domain::new([8; 3], [1.0; 3], 0.01).unwrap();
    let coarse = zero_fields(coarse_domain.layout());
    let mut baseline = zero_fields(fine_domain.layout());
    for (mode, scale) in [([1, 0, 1], 1.0), ([3, 0, 1], 0.25)] {
        hermitian_mode(
            fine_domain.layout(),
            &mut baseline,
            mode,
            [
                Complex64::new(scale, 0.5 * scale),
                Complex64::new(-0.25 * scale, scale),
                Complex64::new(0.5 * scale, -0.75 * scale),
            ],
        );
    }
    let force = baseline.clone().map(|component| {
        component
            .into_iter()
            .map(|value| 3.0 * value)
            .collect::<Vec<_>>()
    });
    let positive = mixed::calculate(
        coarse_domain,
        std::array::from_fn(|axis| coarse[axis].as_slice()),
        fine_domain,
        std::array::from_fn(|axis| baseline[axis].as_slice()),
        fine_domain,
        std::array::from_fn(|axis| force[axis].as_slice()),
    )
    .unwrap();
    for (a, b, c, cross, cosine) in [
        (
            positive.spatial_a.full.l2,
            positive.force_b.full.l2,
            positive.combined_c.full.l2,
            positive.cross_a_b.full.twice_real_inner_product.l2,
            positive.cross_a_b.full.cosine_similarity.l2,
        ),
        (
            positive.spatial_a.common.h1,
            positive.force_b.common.h1,
            positive.combined_c.common.h1,
            positive.cross_a_b.common.twice_real_inner_product.h1,
            positive.cross_a_b.common.cosine_similarity.h1,
        ),
        (
            positive.spatial_a.newly_resolved.vorticity_l2,
            positive.force_b.newly_resolved.vorticity_l2,
            positive.combined_c.newly_resolved.vorticity_l2,
            positive
                .cross_a_b
                .newly_resolved
                .twice_real_inner_product
                .vorticity_l2,
            positive
                .cross_a_b
                .newly_resolved
                .cosine_similarity
                .vorticity_l2,
        ),
    ] {
        close(c * c, a * a + b * b + cross);
        close(cosine.unwrap(), 1.0);
    }

    let zero_force = zero_fields(fine_domain.layout());
    let negative = mixed::calculate(
        coarse_domain,
        std::array::from_fn(|axis| coarse[axis].as_slice()),
        fine_domain,
        std::array::from_fn(|axis| baseline[axis].as_slice()),
        fine_domain,
        std::array::from_fn(|axis| zero_force[axis].as_slice()),
    )
    .unwrap();
    close(negative.combined_c.full.l2, 0.0);
    close(
        negative.cross_a_b.full.twice_real_inner_product.l2,
        -2.0 * negative.spatial_a.full.l2.powi(2),
    );
    close(negative.cross_a_b.full.cosine_similarity.l2.unwrap(), -1.0);
}

#[test]
fn mixed_force_space_small_publication_seam_serializes_bound_output() {
    let root = root("mixed-publication");
    let mut coarse = manifest(root.join("coarse.bin"), 4, "coarse");
    let mut baseline = manifest(root.join("baseline.bin"), 8, "baseline");
    let mut force = manifest(root.join("force.bin"), 8, "force");
    for item in [&mut coarse, &mut baseline, &mut force] {
        item.comparison_kind = ComparisonKind::MixedForceSpaceDiagnostic;
        item.epoch = item.accepted_steps;
        item.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: item.identity.clone(),
        });
        item.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        });
    }
    force.evolution.integration_force_dimensions = [512; 3];
    let coarse_fields = fields(coarse.domain().unwrap().layout(), 1.0);
    let baseline_fields = fields(baseline.domain().unwrap().layout(), 1.5);
    let force_fields = fields(force.domain().unwrap().layout(), 2.0);
    write(&mut coarse, &coarse_fields);
    write(&mut baseline, &baseline_fields);
    write(&mut force, &force_fields);
    let coarse_snapshot = decode::load(&coarse).unwrap();
    let baseline_snapshot = decode::load(&baseline).unwrap();
    let force_snapshot = decode::load(&force).unwrap();
    let metrics = mixed::calculate(
        coarse.domain().unwrap(),
        std::array::from_fn(|axis| coarse_fields[axis].as_slice()),
        baseline.domain().unwrap(),
        std::array::from_fn(|axis| baseline_fields[axis].as_slice()),
        force.domain().unwrap(),
        std::array::from_fn(|axis| force_fields[axis].as_slice()),
    )
    .unwrap();
    let output = mixed::bound_output(
        mixed::BoundSide {
            manifest: &coarse,
            snapshot: &coarse_snapshot,
        },
        mixed::BoundSide {
            manifest: &baseline,
            snapshot: &baseline_snapshot,
        },
        mixed::BoundSide {
            manifest: &force,
            snapshot: &force_snapshot,
        },
        metrics,
        999,
    );
    let json = serde_json::to_string_pretty(&output).unwrap();
    assert!(json.contains("p10-snapshot-mixed-force-space-diagnostic-output-v1"));
    assert!(json.contains("\"comparison_kind\": \"MIXED_FORCE_SPACE_DIAGNOSTIC\""));
    assert!(json.contains("\"cosine_similarity\""));
    assert!(json.contains("\"accepted_windows\": 0"));
    assert!(json.contains("\"admitted_bytes\": 999"));
}

#[test]
fn mixed_force_space_rejects_every_non_force_or_spatial_contract_change() {
    let root = root("mixed-contract");
    let mut coarse = manifest(root.join("coarse.bin"), 256, "coarse");
    let mut baseline = manifest(root.join("baseline.bin"), 384, "baseline");
    let mut force = manifest(root.join("force.bin"), 384, "force");
    for item in [&mut coarse, &mut baseline, &mut force] {
        item.comparison_kind = ComparisonKind::MixedForceSpaceDiagnostic;
        item.epoch = item.accepted_steps;
        item.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: item.identity.clone(),
        });
        item.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        });
    }
    force.evolution.integration_force_dimensions = [512; 3];
    mixed::validate_manifests(&coarse, &baseline, &force).unwrap();
    assert_eq!(
        decode::admitted_bytes_three(&coarse, &baseline, &force).unwrap(),
        3_138_912_256
    );

    let mut changed = force.clone();
    changed.evolution.viscosity = 0.02;
    assert!(mixed::validate_manifests(&coarse, &baseline, &changed).is_err());
    changed = force.clone();
    changed.evolution.schedule = vec![
        ScheduleSegment {
            from_inclusive: 0,
            until_exclusive: 32,
            step_ticks: 16,
        },
        ScheduleSegment {
            from_inclusive: 32,
            until_exclusive: 64,
            step_ticks: 32,
        },
    ];
    assert!(mixed::validate_manifests(&coarse, &baseline, &changed).is_err());
    changed = force.clone();
    changed.evolution.method = "hochbruck-ostermann".into();
    assert!(mixed::validate_manifests(&coarse, &baseline, &changed).is_err());
    changed = force.clone();
    changed.dimensions = [512; 3];
    assert!(mixed::validate_manifests(&coarse, &baseline, &changed).is_err());
    changed = force.clone();
    changed.accepted_steps = 49;
    changed.epoch = 49;
    assert!(mixed::validate_manifests(&coarse, &baseline, &changed).is_err());
}
