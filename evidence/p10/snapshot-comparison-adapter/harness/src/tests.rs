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

type ReviewMutation = fn(&mut ArithmeticReview);

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

fn enable_matched_m512_spatial(left: &mut Manifest, right: &mut Manifest) {
    left.comparison_kind = ComparisonKind::MatchedM512SpatialDiagnostic;
    right.comparison_kind = ComparisonKind::MatchedM512SpatialDiagnostic;
    left.dimensions = [384; 3];
    right.dimensions = [512; 3];
    for manifest in [&mut *left, &mut *right] {
        manifest.evolution.case_sha256 =
            "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e".into();
        manifest.evolution.clock_target = 8192;
        manifest.evolution.comparison_endpoint = 4096;
        manifest.evolution.lengths = [1.0; 3];
        manifest.evolution.viscosity = 1.0;
        manifest.evolution.integration_force_dimensions = [512; 3];
        manifest.evolution.schedule = vec![
            ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 2048,
                step_ticks: 64,
            },
            ScheduleSegment {
                from_inclusive: 2048,
                until_exclusive: 4096,
                step_ticks: 128,
            },
        ];
        manifest.elapsed = 4096;
        manifest.target = 8192;
        manifest.epoch = 48;
        manifest.accepted_steps = 48;
        manifest.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        });
    }
    left.source_commit = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72".into();
    right.source_commit = "e25f3816f83c6a7c07202cac2878f58ace460511".into();
    left.plan_sha256 = "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84".into();
    right.plan_sha256 = "4c14ee169cfbbb2a182d972633aba1292ed041878a9c4322e64a527f87d85da8".into();
    left.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c".into(),
    });
    right.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "n512-m512-h64to2048-h128to4096-cadv33-w3-9eba11f".into(),
    });
    left.identity = format!(
        "source={};profile={}",
        left.source_commit,
        left.profile.as_ref().unwrap().value
    );
    right.identity = format!(
        "source={};profile={};production_source=0843b8b18e6a096a0208e3d896e391c7b1b2f5e0;test_source={};external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline;schema=p10-avx-n512-observer-state-v1",
        right.source_commit,
        right.profile.as_ref().unwrap().value,
        "9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645",
    );
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

#[path = "tests/decoder_regressions.rs"]
mod decoder_regressions;

#[path = "tests/m512_regressions.rs"]
mod m512_regressions;

mod m512_spatial_evolution;

mod m512_temporal;

mod command;

mod coverage_refusals;
mod m512_temporal_admission;

mod fixed_diagnostics;

mod hessian_mixed;

mod temporal_e2e;

mod time_diagnostics;
