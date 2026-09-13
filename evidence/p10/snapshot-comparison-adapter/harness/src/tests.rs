use super::{compare, decode, mixed};
use crate::hessian::{measure_ordered_hessian, screen_ordered_hessian};
use crate::model::{
    AdmissionGuard, ArithmeticControl, ArithmeticReview, ArithmeticSide, ClockHeader,
    ComparisonKind, Evolution, Manifest, MeasuredControl, MeasuredSide, ProfileBinding,
    ProfileBindingKind, ReviewedLineage, ScheduleSegment,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Domain, Layout},
    spectral::modal,
    Complex64,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "p10-snapshot-comparison-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn manifest(path: PathBuf, n: usize, identity: &str) -> Manifest {
    let plan = path.with_file_name(format!("plan-{identity}.json"));
    fs::write(&plan, b"{\"reviewed\":true}\n").unwrap();
    Manifest {
        schema: "p10-snapshot-comparison-input-v1".into(),
        comparison_kind: ComparisonKind::MatchedSpatial,
        snapshot: path,
        plan,
        identity: identity.into(),
        source_commit: "a".repeat(40),
        plan_sha256: format!("{:x}", Sha256::digest(b"{\"reviewed\":true}\n")),
        coefficient_sha256: String::new(),
        file_sha256: String::new(),
        backend: "fixture-backend".into(),
        execution: format!("fixture-{n}"),
        dimensions: [n; 3],
        evolution: Evolution {
            case_sha256: "c".repeat(64),
            quantum_exponent: -20,
            clock_target: 64,
            comparison_endpoint: 64,
            lengths: [1.0; 3],
            viscosity: 0.01,
            method: "cox-matthews".into(),
            integration_force_dimensions: [384; 3],
            schedule: vec![ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 64,
                step_ticks: 32,
            }],
            absolute_tolerances: [1e-5, 1e-4],
            relative_tolerances: [1e-5, 1e-5],
        },
        elapsed: 64,
        target: 64,
        epoch: 1,
        accepted_steps: 2,
        profile: None,
        admission_guard: None,
        arithmetic_control: None,
    }
}

fn enable_time(left: &mut Manifest, right: &mut Manifest, root: &std::path::Path) {
    left.comparison_kind = ComparisonKind::TimeDiagnostic;
    right.comparison_kind = ComparisonKind::TimeDiagnostic;
    left.epoch = 2;
    left.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "n4-m384-h32-cadv045-serial".into(),
    });
    right.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "n4-m384-piecewise-cadv33-w3".into(),
    });
    left.identity = format!("fixture;profile={}", left.profile.as_ref().unwrap().value);
    right.identity = format!("fixture;profile={}", right.profile.as_ref().unwrap().value);
    left.admission_guard = Some(AdmissionGuard {
        advective_limit: 0.45,
        maximum_attempts: 2,
    });
    right.evolution.schedule = vec![
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
    right.epoch = 3;
    right.accepted_steps = 3;
    right.admission_guard = Some(AdmissionGuard {
        advective_limit: 3.3,
        maximum_attempts: 3,
    });
    let review = ArithmeticReview {
        schema: "p10-time-arithmetic-review-v1".into(),
        conclusion: "reviewed-equivalence-supported-by-controls".into(),
        case_sha256: left.evolution.case_sha256.clone(),
        method: left.evolution.method.clone(),
        integration_force_dimensions: left.evolution.integration_force_dimensions,
        measured_control: MeasuredControl {
            outcome: "successful-exact-bit".into(),
            serial: MeasuredSide {
                source_commit: "8".repeat(40),
                backend: "measured-serial-backend".into(),
                execution: "measured-serial-execution".into(),
                configuration: "measured-n4-m384-serial-81bd".into(),
            },
            w3: MeasuredSide {
                source_commit: "f".repeat(40),
                backend: "measured-w3-backend".into(),
                execution: "measured-w3-execution".into(),
                configuration: "measured-n4-m384-w3-f13".into(),
            },
        },
        reviewed_lineage: ReviewedLineage {
            status: "reviewed-unchanged-kernel-lineage".into(),
            left_control_role: "serial".into(),
            right_control_role: "w3".into(),
            left: ArithmeticSide {
                source_commit: left.source_commit.clone(),
                backend: left.backend.clone(),
                execution: left.execution.clone(),
                profile: left.profile.clone().unwrap(),
            },
            right: ArithmeticSide {
                source_commit: right.source_commit.clone(),
                backend: right.backend.clone(),
                execution: right.execution.clone(),
                profile: right.profile.clone().unwrap(),
            },
        },
    };
    let evidence = root.join("serial-w3-arithmetic-review.json");
    let mut evidence_bytes = serde_json::to_vec_pretty(&review).unwrap();
    evidence_bytes.push(b'\n');
    fs::write(&evidence, &evidence_bytes).unwrap();
    let control = ArithmeticControl {
        evidence: PathBuf::from("serial-w3-arithmetic-review.json"),
        evidence_sha256: format!("{:x}", Sha256::digest(&evidence_bytes)),
        review,
    };
    left.arithmetic_control = Some(control.clone());
    right.arithmetic_control = Some(control);
}

fn enable_fixed_diagnostic(left: &mut Manifest, right: &mut Manifest, kind: ComparisonKind) {
    left.comparison_kind = kind;
    right.comparison_kind = kind;
    left.dimensions = [384; 3];
    right.dimensions = [384; 3];
    for manifest in [&mut *left, &mut *right] {
        manifest.evolution.clock_target = 256;
        manifest.evolution.comparison_endpoint = 256;
        manifest.evolution.schedule[0].until_exclusive = 256;
        manifest.elapsed = 256;
        manifest.target = 256;
        manifest.epoch = 8;
        manifest.accepted_steps = 8;
    }
    left.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "left-n384-profile".into(),
    });
    right.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "right-n384-profile".into(),
    });
    left.identity = "fixture;profile=left-n384-profile".into();
    right.identity = "fixture;profile=right-n384-profile".into();
    left.admission_guard = Some(AdmissionGuard {
        advective_limit: 0.45,
        maximum_attempts: 48,
    });
    right.admission_guard = Some(AdmissionGuard {
        advective_limit: 3.3,
        maximum_attempts: 48,
    });
    left.arithmetic_control = None;
    right.arithmetic_control = None;
    match kind {
        ComparisonKind::ForceResolutionDiagnostic => {
            right.evolution.integration_force_dimensions = [512; 3];
        }
        ComparisonKind::MethodDiagnostic => {
            right.evolution.method = "hochbruck-ostermann".into();
        }
        _ => unreachable!(),
    }
}

fn fields(layout: Layout, scale: f64) -> [Vec<Complex64>; 3] {
    let mut result = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
    for (axis, component) in result.iter_mut().enumerate() {
        component[0] = Complex64::new(scale * (axis + 1) as f64, 0.0);
        let a = layout.index([1, 0, 0]).unwrap();
        let b = layout.index([layout.dimensions()[0] - 1, 0, 0]).unwrap();
        component[a] = Complex64::new(scale, scale * 0.25);
        component[b] = component[a].conj();
    }
    result
}

fn zero_fields(layout: Layout) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()])
}

fn hermitian_mode(
    layout: Layout,
    fields: &mut [Vec<Complex64>; 3],
    mode: [isize; 3],
    values: [Complex64; 3],
) {
    for (mode, values) in [
        (mode, values),
        (mode.map(|m| -m), values.map(|value| value.conj())),
    ] {
        let (index, conjugate) = layout.locate(mode).unwrap();
        for (component, value) in fields.iter_mut().zip(values) {
            component[index] = if conjugate { value.conj() } else { value };
        }
    }
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 64.0 * f64::EPSILON * (1.0 + expected.abs()),
        "{actual:e} vs {expected:e}"
    );
}

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

fn encode(manifest: &Manifest, fields: &[Vec<Complex64>; 3]) -> Vec<u8> {
    let mut bytes = b"P10AVXSNAP1\0".to_vec();
    bytes.extend_from_slice(&(manifest.identity.len() as u64).to_le_bytes());
    bytes.extend_from_slice(manifest.identity.as_bytes());
    for value in [
        manifest.elapsed,
        manifest.target,
        manifest.epoch,
        manifest.accepted_steps,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let payload_start = bytes.len();
    for component in fields {
        for value in component {
            bytes.extend_from_slice(&value.re.to_bits().to_le_bytes());
            bytes.extend_from_slice(&value.im.to_bits().to_le_bytes());
        }
    }
    bytes.extend_from_slice(&Sha256::digest(&bytes[payload_start..]));
    bytes
}

fn write(manifest: &mut Manifest, fields: &[Vec<Complex64>; 3]) {
    let bytes = encode(manifest, fields);
    let payload = 12 + 8 + manifest.identity.len() + 4 * 16;
    manifest.coefficient_sha256 =
        format!("{:x}", Sha256::digest(&bytes[payload..bytes.len() - 32]));
    manifest.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&manifest.snapshot, bytes).unwrap();
}

#[test]
fn streamed_adapter_matches_comparison_plan() {
    let root = root("reference");
    let mut coarse = manifest(root.join("coarse.bin"), 4, "coarse");
    let mut fine = manifest(root.join("fine.bin"), 8, "fine");
    let coarse_fields = fields(coarse.domain().unwrap().layout(), 1.0);
    let fine_fields = fields(fine.domain().unwrap().layout(), 1.5);
    write(&mut coarse, &coarse_fields);
    write(&mut fine, &fine_fields);
    let left = decode::load(&coarse).unwrap();
    let right = decode::load(&fine).unwrap();
    let output = compare::compare(&coarse, &left, &fine, &right, 123).unwrap();
    let direct = ComparisonPlan::new(coarse.domain().unwrap(), fine.domain().unwrap())
        .unwrap()
        .compare(
            std::array::from_fn(|axis| coarse_fields[axis].as_slice()),
            std::array::from_fn(|axis| fine_fields[axis].as_slice()),
        )
        .unwrap();
    assert_eq!(output.full.l2, direct.full.l2);
    assert_eq!(output.full.h1, direct.full.h1);
    assert_eq!(output.full.vorticity_l2, direct.full.vorticity_l2);
    assert_eq!(output.full.divergence_l2, direct.full.divergence_l2);
    assert_eq!(output.common.l2, direct.common.l2);
    assert_eq!(output.newly_resolved.l2, direct.newly_resolved.l2);
    assert_eq!(output.mean_error, direct.mean_error);
    assert_ne!(left.coefficient_sha256, left.file_sha256);
    let zeros: [Vec<Complex64>; 3] = std::array::from_fn(|_| {
        vec![Complex64::new(0.0, 0.0); fine.domain().unwrap().layout().half_len()]
    });
    let absolute = ComparisonPlan::new(fine.domain().unwrap(), fine.domain().unwrap())
        .unwrap()
        .compare(
            std::array::from_fn(|axis| zeros[axis].as_slice()),
            std::array::from_fn(|axis| fine_fields[axis].as_slice()),
        )
        .unwrap();
    assert_eq!(output.fine_absolute.l2, absolute.full.l2);
    assert_eq!(output.fine_absolute.h1, absolute.full.h1);
    assert_eq!(
        output.fine_absolute.vorticity_l2,
        absolute.full.vorticity_l2
    );
    assert_eq!(
        output.fine_absolute.divergence_l2,
        absolute.full.divergence_l2
    );
}

#[test]
fn rejects_truncated_and_wrong_magic_without_state_allocation() {
    let root = root("framing");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    let bytes = fs::read(&item.snapshot).unwrap();
    fs::write(&item.snapshot, &bytes[..bytes.len() - 1]).unwrap();
    assert!(decode::load(&item).unwrap_err().contains("length mismatch"));
    let mut wrong = bytes;
    wrong[0] ^= 1;
    fs::write(&item.snapshot, wrong).unwrap();
    assert!(decode::load(&item).unwrap_err().contains("magic mismatch"));
}

#[test]
fn rejects_identity_hash_nonfinite_and_nyquist() {
    let root = root("payload");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let mut values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    item.identity = "different".into();
    assert!(decode::load(&item).unwrap_err().contains("length mismatch"));
    item.identity = "fixture".into();
    let mut damaged = fs::read(&item.snapshot).unwrap();
    let payload = 12 + 8 + item.identity.len() + 4 * 16;
    damaged[payload] ^= 1;
    fs::write(&item.snapshot, damaged).unwrap();
    assert!(decode::load(&item)
        .unwrap_err()
        .contains("trailer mismatch"));

    write(&mut item, &values);
    item.identity = "FIxture".into();
    assert!(decode::load(&item)
        .unwrap_err()
        .contains("identity mismatch"));
    item.identity = "fixture".into();

    values[0][0] = Complex64::new(f64::NAN, 0.0);
    write(&mut item, &values);
    assert!(decode::load(&item).unwrap_err().contains("InvalidSpectrum"));
    values[0][0] = Complex64::new(1.0, 0.0);
    let nyquist = item.domain().unwrap().layout().index([2, 0, 0]).unwrap();
    values[0][nyquist] = Complex64::new(1.0, 0.0);
    write(&mut item, &values);
    assert!(decode::load(&item).unwrap_err().contains("InvalidSpectrum"));
}

#[test]
fn manifest_and_command_enforce_schema_and_cap() {
    let root = root("command");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 8, "right");
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.0);
    write(&mut left, &lf);
    write(&mut right, &rf);
    left.snapshot = PathBuf::from("left.bin");
    right.snapshot = PathBuf::from("right.bin");
    left.plan = PathBuf::from("plan-left.json");
    right.plan = PathBuf::from("plan-right.json");
    let left_path = root.join("left.json");
    let right_path = root.join("right.json");
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    let cap = decode::admitted_bytes(&left, &right).unwrap();
    let args = [
        left_path.clone().into_os_string(),
        right_path.into_os_string(),
        cap.to_string().into(),
    ];
    let output = super::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-comparison-output-v1"));
    let mut refused = args.clone();
    refused[2] = "1".into();
    assert!(super::run(&refused).unwrap_err().contains("exceeds cap"));
    assert!(super::run(&[]).unwrap_err().contains("usage"));

    left.schema = "wrong".into();
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&left_path)
        .unwrap_err()
        .contains("manifest binding"));
    left.schema = "p10-snapshot-comparison-input-v1".into();
    left.evolution.schedule[0].step_ticks = 0;
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&left_path)
        .unwrap_err()
        .contains("piecewise schedule"));
    left.evolution.schedule[0].step_ticks = 32;
    left.plan_sha256 = "0".repeat(64);
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&left_path)
        .unwrap_err()
        .contains("plan SHA-256 mismatch"));
}

#[test]
fn ordered_hessian_option_serializes_bound_output_and_rejects_clock_or_profile_gaps() {
    let root = root("hessian-command");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    for item in [&mut left, &mut right] {
        item.epoch = 2;
        item.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: item.identity.clone(),
        });
        item.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 2,
        });
    }
    let left_fields = fields(left.domain().unwrap().layout(), 1.0);
    let right_fields = fields(right.domain().unwrap().layout(), 1.5);
    write(&mut left, &left_fields);
    write(&mut right, &right_fields);
    let left_path = root.join("left.json");
    let right_path = root.join("right.json");
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    let cap = decode::admitted_bytes(&left, &right).unwrap();
    let args = [
        left_path.clone().into_os_string(),
        right_path.clone().into_os_string(),
        cap.to_string().into(),
        "--ordered-hessian".into(),
    ];
    let output = super::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-ordered-hessian-diagnostic-output-v1"));
    assert!(output.contains("\"status\": \"not_assessed\""));

    right.epoch += 1;
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(super::run(&args)
        .unwrap_err()
        .contains("matched spatial clocks"));
    right.epoch = left.epoch;
    right.profile = None;
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(super::run(&args)
        .unwrap_err()
        .contains("exact profile binding"));

    right.profile = left.profile.clone();
    right.evolution.viscosity = 0.02;
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(super::run(&args)
        .unwrap_err()
        .contains("evolution semantics mismatch"));
    right.evolution.viscosity = left.evolution.viscosity;
    right.evolution.schedule = vec![
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
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(super::run(&args)
        .unwrap_err()
        .contains("evolution semantics mismatch"));
}

#[test]
fn rejects_profile_clock_domain_direction_and_cap() {
    let root = root("binding");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 8, "right");
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.0);
    write(&mut left, &lf);
    write(&mut right, &rf);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();
    right.evolution.quantum_exponent = -19;
    assert!(compare::compare(&left, &ls, &right, &rs, 0)
        .unwrap_err()
        .contains("semantics mismatch"));
    right.evolution = left.evolution.clone();
    let mut bad_clock = ClockHeader::from_manifest(&right);
    bad_clock.elapsed += 1;
    let bad = crate::model::Snapshot {
        clock: bad_clock,
        ..rs
    };
    assert!(compare::compare(&left, &ls, &right, &bad, 0)
        .unwrap_err()
        .contains("clock mismatch"));
    assert!(ComparisonPlan::new(right.domain().unwrap(), left.domain().unwrap()).is_err());
    assert!(decode::admitted_bytes(&left, &right).unwrap() > 1);
}

#[test]
fn matched_spatial_reports_different_guards_without_gating() {
    let root = root("spatial-guards");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 8, "right");
    left.admission_guard = Some(AdmissionGuard {
        advective_limit: 0.45,
        maximum_attempts: 128,
    });
    right.admission_guard = Some(AdmissionGuard {
        advective_limit: 3.3,
        maximum_attempts: 48,
    });
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.0);
    write(&mut left, &lf);
    write(&mut right, &rf);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();
    let output = compare::compare(&left, &ls, &right, &rs, 0).unwrap();
    assert_eq!(output.left_admission_guard.unwrap().advective_limit, 0.45);
    assert_eq!(output.right_admission_guard.unwrap().advective_limit, 3.3);
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("left_admission_guard"));
    assert!(json.contains("right_admission_guard"));

    right.admission_guard = None;
    let output = compare::compare(&left, &ls, &right, &rs, 0).unwrap();
    assert!(output.right_admission_guard.is_none());
}

#[test]
fn time_diagnostic_accepts_h32_vs_piecewise_and_reports_scope() {
    let root = root("time-positive");
    let mut left = manifest(root.join("left.bin"), 4, "left-h32");
    let mut right = manifest(root.join("right.bin"), 4, "right-piecewise");
    right.source_commit = "b".repeat(40);
    right.backend = "fixture-w3-backend".into();
    right.execution = "fixture-w3-execution".into();
    enable_time(&mut left, &mut right, &root);
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.25);
    write(&mut left, &lf);
    write(&mut right, &rf);
    left.snapshot = PathBuf::from("left.bin");
    right.snapshot = PathBuf::from("right.bin");
    left.plan = PathBuf::from("plan-left-h32.json");
    right.plan = PathBuf::from("plan-right-piecewise.json");
    let left_path = root.join("left.json");
    let right_path = root.join("right.json");
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    let cap = decode::admitted_bytes(&left, &right).unwrap();
    let args = [
        left_path.into_os_string(),
        right_path.into_os_string(),
        cap.to_string().into(),
    ];
    let output = super::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-time-diagnostic-output-v1"));
    assert!(output.contains("\"comparison_kind\": \"TIME_DIAGNOSTIC\""));
    assert!(output.contains("\"status\": \"not_assessed\""));
    assert!(output.contains("\"accepted_windows\": 0"));
    assert!(output.contains("\"left_epoch\": 2"));
    assert!(output.contains("\"right_epoch\": 3"));
    assert!(output.contains("n4-m384-h32-cadv045-serial"));
    assert!(output.contains("n4-m384-piecewise-cadv33-w3"));
    assert!(output.contains("measured-n4-m384-serial-81bd"));
    assert!(output.contains("measured-n4-m384-w3-f13"));
    assert!(output.contains("reviewed-equivalence-supported-by-controls"));
    let mut under_cap = args.clone();
    under_cap[2] = "1".into();
    assert!(super::run(&under_cap).unwrap_err().contains("exceeds cap"));
    let mut corrupted = fs::read(root.join("left.bin")).unwrap();
    corrupted[100] ^= 1;
    fs::write(root.join("left.bin"), corrupted).unwrap();
    assert!(super::run(&args).is_err());
}

#[test]
fn time_diagnostic_rejects_every_immutable_semantic_change() {
    let root = root("time-immutable");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();
    let rejects =
        |candidate: &Manifest| compare::time_diagnostic(&left, &ls, candidate, &rs, 0).is_err();

    let mut changed = right.clone();
    changed.evolution.case_sha256 = "d".repeat(64);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.quantum_exponent += 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.clock_target += 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.comparison_endpoint -= 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.lengths[0] = f64::from_bits(1.0_f64.to_bits() + 1);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.viscosity = f64::from_bits(0.01_f64.to_bits() + 1);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.method = "other".into();
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.integration_force_dimensions = [383; 3];
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.absolute_tolerances[0] = 2e-5;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.relative_tolerances[1] = 2e-5;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.dimensions = [6; 3];
    assert!(rejects(&changed));
}

#[test]
fn time_diagnostic_rejects_mode_headers_guards_and_provenance() {
    let root = root("time-policy");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();

    let mut changed = right.clone();
    changed.comparison_kind = ComparisonKind::MatchedSpatial;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("kind mismatch"));
    changed = right.clone();
    changed.accepted_steps -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("derivation mismatch"));
    changed = right.clone();
    changed.epoch -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("derivation mismatch"));
    changed = right.clone();
    changed.evolution.schedule[1].from_inclusive -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("invalid time-diagnostic schedule"));
    changed = right.clone();
    changed.admission_guard.as_mut().unwrap().maximum_attempts -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("derivation mismatch"));
    changed = right.clone();
    changed
        .arithmetic_control
        .as_mut()
        .unwrap()
        .review
        .reviewed_lineage
        .right
        .source_commit = "e".repeat(40);
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("binding mismatch"));
    changed = right.clone();
    changed.backend.push_str("-changed");
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("side binding mismatch"));
    changed = right.clone();
    changed.execution.push_str("-changed");
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("side binding mismatch"));
    changed = right.clone();
    changed.profile.as_mut().unwrap().value.push_str("-changed");
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("profile does not match"));
    changed = right.clone();
    changed.arithmetic_control.as_mut().unwrap().evidence_sha256 = "f".repeat(64);
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("binding mismatch"));

    left.snapshot = PathBuf::from("left.bin");
    left.plan = PathBuf::from("plan-left.json");
    let manifest_path = root.join("left.json");
    left.admission_guard.as_mut().unwrap().advective_limit = 0.0;
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("admission guard"));
    left.admission_guard.as_mut().unwrap().advective_limit = 0.45;
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 = "0".repeat(64);
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("arithmetic-control SHA-256 mismatch"));

    let evidence_path = root.join("serial-w3-arithmetic-review.json");
    let arbitrary = b"{\"arbitrary\":true}\n";
    fs::write(&evidence_path, arbitrary).unwrap();
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 =
        format!("{:x}", Sha256::digest(arbitrary));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("evidence schema"));

    let control = left.arithmetic_control.as_mut().unwrap();
    control.review.case_sha256 = "d".repeat(64);
    let mut semantic = serde_json::to_vec_pretty(&control.review).unwrap();
    semantic.push(b'\n');
    fs::write(&evidence_path, &semantic).unwrap();
    control.evidence_sha256 = format!("{:x}", Sha256::digest(&semantic));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    let reviewed_left = decode::read_manifest(&manifest_path).unwrap();
    let mut semantic_right = right.clone();
    semantic_right.arithmetic_control = reviewed_left.arithmetic_control.clone();
    assert!(
        compare::validate_manifest_pair(&reviewed_left, &semantic_right)
            .unwrap_err()
            .contains("physical contract mismatch")
    );
}

#[test]
fn arithmetic_review_accepts_64_kib_and_rejects_one_byte_more() {
    const CAP: usize = 64 * 1024;
    let root = root("arithmetic-cap");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    left.snapshot = PathBuf::from("left.bin");
    left.plan = PathBuf::from("plan-left.json");
    let manifest_path = root.join("left.json");
    let evidence_path = root.join("serial-w3-arithmetic-review.json");

    let control = left.arithmetic_control.as_mut().unwrap();
    let mut exact = serde_json::to_vec(&control.review).unwrap();
    exact.resize(CAP, b' ');
    fs::write(&evidence_path, &exact).unwrap();
    control.evidence_sha256 = format!("{:x}", Sha256::digest(&exact));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path).is_ok());

    exact.push(b' ');
    fs::write(&evidence_path, &exact).unwrap();
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 =
        format!("{:x}", Sha256::digest(&exact));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("exceeds 64 KiB bound"));
}

#[test]
fn legacy_profile_binding_requires_the_exact_complete_identity() {
    let root = root("legacy-profile");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    left.identity = "source=legacy;case=fixture;n=4;m=384;step=32".into();
    left.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::LegacyFullIdentity,
        value: left.identity.clone(),
    });
    let control = left.arithmetic_control.as_mut().unwrap();
    control.review.reviewed_lineage.left.profile = left.profile.clone().unwrap();
    right.arithmetic_control = Some(control.clone());
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());

    left.profile
        .as_mut()
        .unwrap()
        .value
        .push_str(";invented=true");
    assert!(compare::validate_manifest_pair(&left, &right)
        .unwrap_err()
        .contains("profile does not match"));
    left.profile.as_mut().unwrap().value = left.identity.clone();
    left.profile.as_mut().unwrap().kind = ProfileBindingKind::IdentityProfileField;
    assert!(compare::validate_manifest_pair(&left, &right)
        .unwrap_err()
        .contains("profile does not match"));
}

#[test]
fn force_resolution_diagnostic_is_closed_and_directional() {
    let root = root("force-diagnostic");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    enable_fixed_diagnostic(
        &mut left,
        &mut right,
        ComparisonKind::ForceResolutionDiagnostic,
    );
    assert_manifest_decodes(&root, "force-left.json", &left);
    assert_manifest_decodes(&root, "force-right.json", &right);
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());
    assert_ne!(left.admission_guard, right.admission_guard);
    assert_eq!(left.accepted_steps, 8);
    assert_eq!(left.admission_guard.as_ref().unwrap().maximum_attempts, 48);

    let mut exhausted = right.clone();
    exhausted.admission_guard.as_mut().unwrap().maximum_attempts = 7;
    assert!(compare::validate_manifest_pair(&left, &exhausted).is_err());

    let mut changed = right.clone();
    changed.evolution.integration_force_dimensions = [384; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    let mut reversed_left = left.clone();
    let mut reversed_right = right.clone();
    reversed_left.evolution.integration_force_dimensions = [512; 3];
    reversed_right.evolution.integration_force_dimensions = [384; 3];
    assert!(compare::validate_manifest_pair(&reversed_left, &reversed_right).is_err());
    changed = right.clone();
    changed.evolution.method = "hochbruck-ostermann".into();
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    changed = right.clone();
    changed.dimensions = [383; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    assert_other_physics_rejected(&left, &right);
}

#[test]
fn method_diagnostic_is_closed_and_directional() {
    let root = root("method-diagnostic");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    enable_fixed_diagnostic(&mut left, &mut right, ComparisonKind::MethodDiagnostic);
    assert_manifest_decodes(&root, "method-left.json", &left);
    assert_manifest_decodes(&root, "method-right.json", &right);
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());
    assert_ne!(left.admission_guard, right.admission_guard);
    assert_eq!(right.accepted_steps, 8);
    assert_eq!(right.admission_guard.as_ref().unwrap().maximum_attempts, 48);

    let mut exhausted = right.clone();
    exhausted.admission_guard.as_mut().unwrap().maximum_attempts = 7;
    assert!(compare::validate_manifest_pair(&left, &exhausted).is_err());

    let mut changed = right.clone();
    changed.evolution.method = "cox-matthews".into();
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    let mut reversed_left = left.clone();
    let mut reversed_right = right.clone();
    reversed_left.evolution.method = "hochbruck-ostermann".into();
    reversed_right.evolution.method = "cox-matthews".into();
    assert!(compare::validate_manifest_pair(&reversed_left, &reversed_right).is_err());
    changed = right.clone();
    changed.evolution.integration_force_dimensions = [512; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    changed = right.clone();
    changed.dimensions = [256; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    assert_other_physics_rejected(&left, &right);
}

#[test]
fn fixed_diagnostics_serialize_tiny_spectrum_outputs() {
    for (kind, schema, label) in [
        (
            ComparisonKind::ForceResolutionDiagnostic,
            "p10-snapshot-force-resolution-diagnostic-output-v1",
            "FORCE_RESOLUTION_DIAGNOSTIC",
        ),
        (
            ComparisonKind::MethodDiagnostic,
            "p10-snapshot-method-diagnostic-output-v1",
            "METHOD_DIAGNOSTIC",
        ),
    ] {
        let root = root(label);
        let mut left = manifest(root.join("left.bin"), 4, "left");
        let mut right = manifest(root.join("right.bin"), 4, "right");
        left.comparison_kind = kind;
        right.comparison_kind = kind;
        left.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: left.identity.clone(),
        });
        right.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: right.identity.clone(),
        });
        left.admission_guard = Some(AdmissionGuard {
            advective_limit: 0.45,
            maximum_attempts: 48,
        });
        right.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        });
        for manifest in [&mut left, &mut right] {
            manifest.evolution.clock_target = 256;
            manifest.evolution.comparison_endpoint = 256;
            manifest.evolution.schedule[0].until_exclusive = 256;
            manifest.elapsed = 256;
            manifest.target = 256;
            manifest.epoch = 8;
            manifest.accepted_steps = 8;
        }
        match kind {
            ComparisonKind::ForceResolutionDiagnostic => {
                right.evolution.integration_force_dimensions = [512; 3];
            }
            ComparisonKind::MethodDiagnostic => {
                right.evolution.method = "hochbruck-ostermann".into();
            }
            _ => unreachable!(),
        }
        let left_values = fields(left.domain().unwrap().layout(), 1.0);
        let right_values = fields(right.domain().unwrap().layout(), 1.25);
        write(&mut left, &left_values);
        write(&mut right, &right_values);
        let left_state = decode::load(&left).unwrap();
        let right_state = decode::load(&right).unwrap();
        let output = compare::diagnostic_output(
            &left,
            &left_state,
            &right,
            &right_state,
            123,
            schema,
            label,
            None,
        )
        .unwrap();
        let json = serde_json::to_value(output).unwrap();
        assert_eq!(json["schema"], schema);
        assert_eq!(json["comparison_kind"], label);
        assert_eq!(json["acceptance"]["status"], "not_assessed");
        assert_eq!(json["acceptance"]["accepted_windows"], 0);
        assert!(json.get("arithmetic_control").is_none());
        assert_eq!(json["left_identity"], "left");
        assert_eq!(json["right_identity"], "right");
        assert_eq!(json["left_admission_guard"]["advective_limit"], 0.45);
        assert_eq!(json["right_admission_guard"]["advective_limit"], 3.3);
        assert_eq!(json["left_plan_sha256"], left.plan_sha256);
        assert_eq!(json["right_plan_sha256"], right.plan_sha256);
        assert_eq!(json["clock"]["left_accepted_steps"], 8);
        assert_eq!(json["clock"]["right_accepted_steps"], 8);
        assert_eq!(json["left_admission_guard"]["maximum_attempts"], 48);
        assert_eq!(json["right_admission_guard"]["maximum_attempts"], 48);
        assert_eq!(json["admitted_bytes"], 123);
    }
}

fn assert_other_physics_rejected(left: &Manifest, right: &Manifest) {
    let rejects = |candidate: &Manifest| compare::validate_manifest_pair(left, candidate).is_err();
    let mut changed = right.clone();
    changed.evolution.case_sha256 = "d".repeat(64);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.quantum_exponent += 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.clock_target += 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.comparison_endpoint -= 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.lengths[0] = f64::from_bits(1.0_f64.to_bits() + 1);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.viscosity = f64::from_bits(0.01_f64.to_bits() + 1);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.schedule[0].step_ticks = 16;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.absolute_tolerances[0] = 2e-5;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.relative_tolerances[1] = 2e-5;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.accepted_steps -= 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.profile.as_mut().unwrap().value.push_str("-changed");
    assert!(rejects(&changed));
    changed = right.clone();
    changed.arithmetic_control = left.arithmetic_control.clone().or_else(|| {
        let mut l = left.clone();
        let mut r = right.clone();
        let suffix = match left.comparison_kind {
            ComparisonKind::ForceResolutionDiagnostic => "force",
            ComparisonKind::MethodDiagnostic => "method",
            _ => unreachable!("helper is only used by fixed-schedule diagnostics"),
        };
        let temp = root(&format!("forbidden-control-{suffix}"));
        enable_time(&mut l, &mut r, &temp);
        l.arithmetic_control
    });
    assert!(rejects(&changed));
    changed = right.clone();
    changed.comparison_kind = ComparisonKind::MatchedSpatial;
    assert!(rejects(&changed));
}

fn assert_manifest_decodes(root: &std::path::Path, name: &str, manifest: &Manifest) {
    let path = root.join(name);
    fs::write(&path, serde_json::to_vec(manifest).unwrap()).unwrap();
    decode::read_manifest(&path).unwrap();
}
