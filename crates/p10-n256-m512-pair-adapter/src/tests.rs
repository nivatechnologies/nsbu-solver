use super::{contract, decode, lineage, pair};
use crate::model::{
    AdmissionGuard, ClockHeader, ComparisonKind, Evolution, LineageBinding, LineageIntake,
    Manifest, ProfileBinding, ProfileBindingKind, RestRecord, ScheduleSegment, StateRecord,
    INPUT_SCHEMA,
};
use nsbu_solver::{domain::Domain, Complex64};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

pub(crate) const R6_FROZEN_PLAN_PATH: &str =
    "/mnt/niva-array/work/opencode-offline-preparation-20260914/captured-metadata/r6-frozen-plan.json";
pub(crate) const R6_ENDPOINT_STATE_PATH: &str =
    "/mnt/niva-array/nsbu-solver/work/p10-m512-endpoint-r6-artifacts-20260913/output/step-048-clock-4096/state.bin";

pub(crate) fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "p10-n256-m512-pair-adapter-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

pub(crate) fn coarse_identity(source: &str) -> String {
    format!(
        "source={source};case={};profile={};backend=fixture-backend;w3_source={};rhs_w3={};force_w3={};retained=256;force_samples=512;rhs_dealias=384;method=cox-matthews;schedule=h64-clocks0-through2048-then-h128-through4096;endpoint=4096;advective_limit=3.3;schema={};attempt_schema={};resume=unsupported;host=baccus;numa={};external_stop={}",
        contract::CASE_SHA256,
        contract::COARSE_PROFILE,
        contract::COARSE_W3_SOURCE,
        contract::COARSE_RHS_W3,
        contract::COARSE_FORCE_W3,
        contract::COARSE_STATE_SCHEMA,
        contract::COARSE_ATTEMPT_SCHEMA,
        contract::COARSE_NUMA,
        contract::COARSE_EXTERNAL_STOP,
    )
}

pub(crate) fn closed_evolution() -> Evolution {
    Evolution {
        case_sha256: contract::CASE_SHA256.into(),
        quantum_exponent: contract::QUANTUM_EXPONENT,
        clock_target: contract::CLOCK_TARGET,
        comparison_endpoint: contract::COMPARISON_ENDPOINT,
        lengths: [1.0; 3],
        viscosity: 1.0,
        method: "cox-matthews".into(),
        integration_force_dimensions: contract::FORCE_DIMENSIONS,
        schedule: vec![
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
        ],
        absolute_tolerances: [1e-5, 1e-4],
        relative_tolerances: [1e-5, 1e-5],
    }
}

pub(crate) fn coarse_manifest(dir: &std::path::Path, source: &str) -> Manifest {
    Manifest {
        schema: INPUT_SCHEMA.into(),
        comparison_kind: ComparisonKind::N256M512PairEndpointDiagnostic,
        snapshot: dir.join("coarse-endpoint-state.bin"),
        plan: dir.join("coarse-frozen-plan.json"),
        identity: coarse_identity(source),
        source_commit: source.into(),
        plan_sha256: "0".repeat(64),
        coefficient_sha256: "1".repeat(64),
        file_sha256: "2".repeat(64),
        backend: "fixture-backend".into(),
        execution: "fixture-coarse-execution".into(),
        dimensions: contract::COARSE_DIMENSIONS,
        evolution: closed_evolution(),
        elapsed: contract::COMPARISON_ENDPOINT,
        target: contract::CLOCK_TARGET,
        epoch: contract::ENDPOINT_EPOCH,
        accepted_steps: contract::ENDPOINT_ACCEPTED_STEPS,
        profile: ProfileBinding {
            kind: ProfileBindingKind::IdentityProfileField,
            value: contract::COARSE_PROFILE.into(),
        },
        admission_guard: AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: contract::MAXIMUM_ATTEMPTS,
        },
        lineage: None,
    }
}

pub(crate) fn fine_manifest(_dir: &std::path::Path) -> Manifest {
    Manifest {
        schema: INPUT_SCHEMA.into(),
        comparison_kind: ComparisonKind::N256M512PairEndpointDiagnostic,
        snapshot: PathBuf::from(R6_ENDPOINT_STATE_PATH),
        plan: PathBuf::from(R6_FROZEN_PLAN_PATH),
        identity: crate::fine_identity::FINE_IDENTITY.into(),
        source_commit: contract::R6_SOURCE.into(),
        plan_sha256: contract::R6_PLAN.into(),
        coefficient_sha256: contract::R6_ENDPOINT_COEFFICIENT_SHA256.into(),
        file_sha256: contract::R6_ENDPOINT_FILE_SHA256.into(),
        backend: crate::fine_identity::FINE_BACKEND.into(),
        execution: crate::fine_identity::FINE_EXECUTION.into(),
        dimensions: contract::FINE_DIMENSIONS,
        evolution: closed_evolution(),
        elapsed: contract::COMPARISON_ENDPOINT,
        target: contract::CLOCK_TARGET,
        epoch: contract::ENDPOINT_EPOCH,
        accepted_steps: contract::ENDPOINT_ACCEPTED_STEPS,
        profile: ProfileBinding {
            kind: ProfileBindingKind::IdentityProfileField,
            value: contract::R6_PROFILE.into(),
        },
        admission_guard: AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: contract::MAXIMUM_ATTEMPTS,
        },
        lineage: None,
    }
}

/// The exact bytes the `n384-prep` writer's `publish_rest` emits for an identity.
pub(crate) fn rest_artifact_bytes(identity: &str) -> Vec<u8> {
    serde_json::to_vec(&crate::model::RestArtifact {
        schema: crate::model::REST_SCHEMA.into(),
        identity: identity.into(),
        clock: 0,
        state_payload: false,
        observation_status: crate::model::REST_OBSERVATION_STATUS.into(),
        balance: crate::model::REST_BALANCE.into(),
    })
    .unwrap()
}

/// Write the real metadata-only rest record and return its whole-file SHA-256.
pub(crate) fn write_rest_artifact(path: &std::path::Path, identity: &str) -> String {
    let bytes = rest_artifact_bytes(identity);
    fs::write(path, &bytes).unwrap();
    format!("{:x}", Sha256::digest(&bytes))
}

/// Write the reviewed closed-schedule intake with synthetic state paths and the
/// exact metadata-only rest record the capture writer actually publishes.
pub(crate) fn intake_fixture(
    dir: &std::path::Path,
    source: &str,
    plan_sha256: &str,
) -> (PathBuf, LineageIntake) {
    let states = contract::required_clocks()
        .into_iter()
        .enumerate()
        .map(|(index, clock)| StateRecord {
            clock,
            path: dir.join(format!("step-{:03}-clock-{clock:04}", index + 1)),
            coefficient_sha256: format!("{:064x}", index + 1),
            file_sha256: format!("{:064x}", index + 1000),
        })
        .collect();
    let rest_hash = write_rest_artifact(&dir.join("rest"), &coarse_identity(source));
    let intake = LineageIntake {
        schema: crate::model::LINEAGE_SCHEMA.into(),
        host: contract::COARSE_HOST.into(),
        profile: contract::COARSE_PROFILE.into(),
        source_commit: source.into(),
        plan_sha256: plan_sha256.into(),
        binary_sha256: "3".repeat(64),
        from_rest: true,
        rest: RestRecord {
            path: dir.join("rest"),
            file_sha256: rest_hash,
        },
        states,
    };
    let path = dir.join("lineage-intake.json");
    fs::write(&path, serde_json::to_vec_pretty(&intake).unwrap()).unwrap();
    (path, intake)
}

/// A synthetic reviewed closed-lineage receipt bound to the intake's declared
/// step/endpoint hashes. Tests use it to exercise the completed-lineage
/// authentication path; it never represents real vault N256 lineage.
pub(crate) fn trusted_anchor(intake: &LineageIntake) -> lineage::TrustedLineage {
    lineage::TrustedLineage {
        identity: coarse_identity(&intake.source_commit),
        source_commit: intake.source_commit.clone(),
        plan_sha256: intake.plan_sha256.clone(),
        binary_sha256: intake.binary_sha256.clone(),
        rest_file_sha256: intake.rest.file_sha256.clone(),
        steps: intake
            .states
            .iter()
            .map(|record| lineage::TrustedStep {
                clock: record.clock,
                coefficient_sha256: record.coefficient_sha256.clone(),
                file_sha256: record.file_sha256.clone(),
            })
            .collect(),
    }
}

/// Writer-schema header with a sparse body of the exact committed length.
pub(crate) fn write_sparse_state(path: &std::path::Path, identity: &str, header: ClockHeader) {
    let mut file = File::create(path).unwrap();
    file.write_all(b"P10AVXSNAP1\0").unwrap();
    file.write_all(&(identity.len() as u64).to_le_bytes())
        .unwrap();
    file.write_all(identity.as_bytes()).unwrap();
    for word in [
        header.elapsed,
        header.target,
        header.epoch,
        header.accepted_steps,
    ] {
        file.write_all(&word.to_le_bytes()).unwrap();
    }
    let total = 12 + 8 + identity.len() + 4 * 16 + contract::COARSE_STATE_BYTES + 32;
    file.set_len(u64::try_from(total).unwrap()).unwrap();
}

pub(crate) fn bind_intake(manifest: &mut Manifest, path: &PathBuf) {
    let bytes = fs::read(path).unwrap();
    manifest.lineage = Some(LineageBinding {
        intake: path.clone(),
        intake_sha256: format!("{:x}", Sha256::digest(bytes)),
    });
}

/// A contract-valid manifest pair (files not required by the pure contract).
pub(crate) fn valid_pair(dir: &std::path::Path) -> (Manifest, Manifest) {
    let mut left = coarse_manifest(dir, &"a".repeat(40));
    left.plan_sha256 = "5".repeat(64);
    let (intake_path, _) = intake_fixture(dir, &left.source_commit, &left.plan_sha256);
    bind_intake(&mut left, &intake_path);
    let right = fine_manifest(dir);
    (left, right)
}

pub(crate) fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 64.0 * f64::EPSILON * (1.0 + expected.abs()),
        "{actual:e} vs {expected:e}"
    );
}

pub(crate) fn zero_fields(layout: nsbu_solver::domain::Layout) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()])
}

/// Set a Hermitian mode pair (mode with nonnegative last coordinate) exactly.
pub(crate) fn hermitian_mode(
    layout: nsbu_solver::domain::Layout,
    fields: &mut [Vec<Complex64>; 3],
    mode: [isize; 3],
    values: [Complex64; 3],
) {
    for (mode, values) in [
        (mode, values),
        (
            mode.map(|coordinate| -coordinate),
            values.map(|value| value.conj()),
        ),
    ] {
        let (index, conjugate) = layout.locate(mode).unwrap();
        for (component, value) in fields.iter_mut().zip(values) {
            component[index] = if conjugate { value.conj() } else { value };
        }
    }
}

/// Copy every stored coarse coefficient into its located fine slot.
pub(crate) fn copy_coarse_to_fine(
    coarse: Domain,
    fine: Domain,
    source: &[Vec<Complex64>; 3],
    target: &mut [Vec<Complex64>; 3],
) {
    let coarse_layout = coarse.layout();
    let fine_layout = fine.layout();
    for (index, _) in source[0].iter().enumerate() {
        let position = coarse_layout.position(index).unwrap();
        let mode = coarse_layout.mode(position).unwrap();
        let (fine_index, conjugate) = fine_layout.locate(mode).unwrap();
        for axis in 0..3 {
            let value = source[axis][index];
            target[axis][fine_index] = if conjugate { value.conj() } else { value };
        }
    }
}

pub(crate) fn domain(dimensions: [usize; 3]) -> Domain {
    Domain::new(dimensions, [1.0; 3], 1.0).unwrap()
}

/// Snapshot byte encoder bound to the exact Rust writer schema: little-endian
/// identity length, little-endian u128 clock words, and little-endian f64
/// bit patterns (real then imaginary) for each strict half-spectrum slot.
pub(crate) fn encode(manifest: &Manifest, fields: &[Vec<Complex64>; 3]) -> Vec<u8> {
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

pub(crate) fn write_fixture(manifest: &mut Manifest, fields: &[Vec<Complex64>; 3]) -> Vec<u8> {
    let bytes = encode(manifest, fields);
    let payload = 12 + 8 + manifest.identity.len() + 4 * 16;
    manifest.coefficient_sha256 =
        format!("{:x}", Sha256::digest(&bytes[payload..bytes.len() - 32]));
    manifest.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&manifest.snapshot, &bytes).unwrap();
    bytes
}

pub(crate) fn small_manifest(dir: &std::path::Path, n: usize, name: &str) -> Manifest {
    let identity = format!("fixture-{name};profile=fixture-{name}");
    Manifest {
        schema: INPUT_SCHEMA.into(),
        comparison_kind: ComparisonKind::N256M512PairEndpointDiagnostic,
        snapshot: dir.join(format!("{name}.bin")),
        plan: dir.join(format!("{name}-plan.json")),
        identity,
        source_commit: "a".repeat(40),
        plan_sha256: String::new(),
        coefficient_sha256: String::new(),
        file_sha256: String::new(),
        backend: "fixture-backend".into(),
        execution: format!("fixture-{name}"),
        dimensions: [n; 3],
        evolution: Evolution {
            case_sha256: "c".repeat(64),
            quantum_exponent: -20,
            clock_target: 64,
            comparison_endpoint: 64,
            lengths: [1.0; 3],
            viscosity: 1.0,
            method: "cox-matthews".into(),
            integration_force_dimensions: [512; 3],
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
        epoch: 2,
        accepted_steps: 2,
        profile: ProfileBinding {
            kind: ProfileBindingKind::IdentityProfileField,
            value: format!("fixture-{name}"),
        },
        admission_guard: AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        },
        lineage: None,
    }
}

pub(crate) fn finish_small_manifest(manifest: &mut Manifest) {
    fs::write(&manifest.plan, b"{\"fixture\":true}\n").unwrap();
    manifest.plan_sha256 = format!("{:x}", Sha256::digest(b"{\"fixture\":true}\n"));
}

#[path = "tests/contract_refusals.rs"]
mod contract_refusals;

#[path = "tests/lineage_gate.rs"]
mod lineage_gate;

#[path = "tests/band_math.rs"]
mod band_math;

#[path = "tests/decoder_regressions.rs"]
mod decoder_regressions;

#[path = "tests/admission_pipeline.rs"]
mod admission_pipeline;
