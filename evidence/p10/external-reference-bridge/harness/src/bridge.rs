use crate::model::{debug, Manifest as SnapshotManifest, NormOutput};
use nsbu_benchmarks::{scalar, time::BenchmarkTime, CASE_SHA256};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Layout, TickClock},
    spectral::{transfer, FftBackend, FftCatalog, FftPlan},
    Complex64,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

mod io;
mod producer;
mod validator;

use io::{read_bounded, resolve, verify_current_executable, verify_file};
use producer::{backend, produce_reference};
use validator::{validate_binding, validate_bridge, validate_snapshot_review};

const MANIFEST_CAP: u64 = 64 * 1024;
const SOURCE_CAP: u64 = 1024 * 1024;
const OUTPUT_CAP: usize = 64 * 1024;
const SNAPSHOT_DECODER_OVERHEAD: usize = 1024 * 1024;
const ALLOCATOR_ALLOWANCE: usize = 10 * 4096;
const EVALUATOR: &str = "nsbu_benchmarks::scalar::evaluate";
const ARITHMETIC: &str = "binary64-explicit-scalar-root-and-velocity";
const GRID: &str = "unshifted-periodic-x_i=i/n";
const NORMALIZATION: &str = "forward-r2c-divide-by-complete-real-sample-count";
const CROP: &str = "normalization-preserving-strict-band-transfer-no-rescale";
const NYQUIST: &str = "sample-nyquist-not-retained;target-nyquist-explicit-zero";
const PROJECTION: &str = "none";
const CLASSIFICATION: &str = "sampled-binary64-reference-diagnostic-not-continuum-or-enclosure";
const EXECUTION_CONTEXT: &str = "contended-local-p10-campaign-bounded-1800s";
const REFERENCE_SOURCE_COMMIT: &str = "6bdea3084d737ce6585cb67ab5d48810bd03cd50";
const PROFILE_M384: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";
const PROFILE_M512: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
const SOURCE_M384: &str = "aed49b7d7874a0a720dee88b65ba180c7286fa65";
const SOURCE_M512: &str = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72";
const PLAN_M384: &str = "2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a";
const PLAN_M512: &str = "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84";
const SNAPSHOT_EXECUTION_CAP: usize = 206_158_430_208;
const SNAPSHOT_ARTIFACT_CAP: usize = 137_438_953_472;
const SOURCES: [(&str, &str); 7] = [
    (
        "scalar-evaluator",
        "2bd8e841dc5b2b1bd4a301ab8bfc392b81716ce58f5c019b8a35b4c9ff599bb8",
    ),
    (
        "scalar-root",
        "8b41613dfad4f0c09c2bdb28d9e28d499cebd9e55b2210c3f6cc400730356c3b",
    ),
    (
        "benchmark-time",
        "c5301d2ab63ea3cec9e331bec3c3e41d0c2c0a4e613807a062e516046a74fd06",
    ),
    (
        "benchmark-error",
        "9cd63ed56079fcf20e90d3139aeff3ebe45525f9b2acf81ef9bcea522a97137a",
    ),
    (
        "benchmark-module",
        "398dd1e167349fa99e79ada2815e34cedd458d7989855a19973135688986ef2b",
    ),
    (
        "tick-clock",
        "c8d56c25c03aeda44fb69e8896406ee02eb32646de0222ea5782c11fac5cc4ce",
    ),
    (
        "case-definition",
        "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e",
    ),
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceBinding {
    pub role: String,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BridgeManifest {
    pub schema: String,
    pub snapshot_manifest: PathBuf,
    pub snapshot_manifest_sha256: String,
    pub case_sha256: String,
    pub reference_source_commit: String,
    pub reference_sources: Vec<SourceBinding>,
    pub harness_source_commit: String,
    pub binary_sha256: String,
    pub reference_evaluator: String,
    pub arithmetic: String,
    pub sample_dimensions: [usize; 3],
    pub retained_dimensions: [usize; 3],
    pub physical_grid: String,
    pub fft_backend: String,
    pub fft_normalization: String,
    pub crop: String,
    pub nyquist: String,
    pub projection: String,
    pub coordinate_shift: bool,
    pub mean_alignment: bool,
    pub classification: String,
    pub execution_context: String,
    pub execution_cap_bytes: usize,
    pub clock_exponent: i32,
    pub clock_target: u128,
    pub elapsed: u128,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Storage {
    pub decoded_actual_state: usize,
    pub snapshot_decoder_overhead: usize,
    pub physical_reference_velocity: usize,
    pub fft_catalog: usize,
    pub fft_plan_and_workspace: usize,
    pub fft_output_spectrum: usize,
    pub retained_reference_velocity: usize,
    pub allocator_allowance: usize,
    pub serialization: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Work {
    pub reference_evaluations: usize,
    pub maximum_root_iterations: usize,
    pub scalar_forward_transforms: usize,
    pub transfer_coefficient_visits: usize,
    pub comparison_work_units: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Preflight {
    pub storage: Storage,
    pub work: Work,
}

#[derive(Debug, Serialize)]
pub(crate) struct PreflightOutput<'a> {
    pub schema: &'static str,
    pub status: &'static str,
    pub bridge: BridgeOutput<'a>,
    pub snapshot_identity: &'a str,
    pub snapshot_source_commit: &'a str,
    pub snapshot_plan_sha256: &'a str,
    pub snapshot_coefficient_sha256: &'a str,
    pub snapshot_file_sha256: &'a str,
    pub preflight: Preflight,
    pub reference_assignments: usize,
    pub resume_or_import_interfaces: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct DiagnosticOutput<'a> {
    pub schema: &'static str,
    pub status: &'static str,
    pub bridge: BridgeOutput<'a>,
    pub snapshot_identity: &'a str,
    pub snapshot_source_commit: &'a str,
    pub snapshot_plan_sha256: &'a str,
    pub snapshot_coefficient_sha256: &'a str,
    pub snapshot_file_sha256: &'a str,
    pub difference: NormOutput,
    pub actual: NormOutput,
    pub sampled_reference: NormOutput,
    pub signed_reference_minus_actual_mean: [f64; 3],
    pub preflight: Preflight,
    pub reference_assignments: usize,
    pub resume_or_import_interfaces: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct BridgeOutput<'a> {
    pub case_sha256: &'a str,
    pub reference_source_commit: &'a str,
    pub reference_sources: [SourceOutput<'a>; 7],
    pub harness_source_commit: &'a str,
    pub binary_sha256: &'a str,
    pub reference_evaluator: &'a str,
    pub arithmetic: &'a str,
    pub sample_dimensions: [usize; 3],
    pub retained_dimensions: [usize; 3],
    pub physical_grid: &'a str,
    pub fft_backend: &'a str,
    pub fft_normalization: &'a str,
    pub crop: &'a str,
    pub nyquist: &'a str,
    pub projection: &'a str,
    pub coordinate_shift: bool,
    pub mean_alignment: bool,
    pub classification: &'a str,
    pub execution_context: &'a str,
    pub execution_cap_bytes: usize,
    pub clock_exponent: i32,
    pub clock_target: u128,
    pub elapsed: u128,
}

#[derive(Debug, Serialize)]
pub(crate) struct SourceOutput<'a> {
    pub role: &'a str,
    pub sha256: &'a str,
}

pub(crate) struct BoundInput {
    pub bridge: BridgeManifest,
    pub snapshot: SnapshotManifest,
    pub preflight: Preflight,
}

pub(crate) fn bind(path: &Path) -> Result<BoundInput, String> {
    let bridge = read_bridge(path)?;
    validate_production_input(&bridge)?;
    let snapshot = bind_snapshot(&bridge)?;
    let preflight = preflight(&bridge, &snapshot)?;
    if preflight.storage.total != bridge.execution_cap_bytes {
        return Err("bridge execution cap does not equal exact reservation".into());
    }
    Ok(BoundInput {
        bridge,
        snapshot,
        preflight,
    })
}

fn read_bridge(path: &Path) -> Result<BridgeManifest, String> {
    let bytes = read_bounded(path, MANIFEST_CAP, "bridge manifest exceeds 64 KiB")?;
    let mut bridge: BridgeManifest = serde_json::from_slice(&bytes).map_err(debug)?;
    resolve(path, &mut bridge.snapshot_manifest);
    for source in &mut bridge.reference_sources {
        resolve(path, &mut source.path);
    }
    Ok(bridge)
}

fn validate_production_input(bridge: &BridgeManifest) -> Result<(), String> {
    validate_bridge(bridge)?;
    let profile = [
        bridge.sample_dimensions == [768; 3],
        bridge.retained_dimensions == [384; 3],
        [512, 4096].contains(&bridge.elapsed),
    ];
    if !profile.into_iter().all(std::convert::identity) {
        return Err("unsupported production bridge profile".into());
    }
    verify_file(
        &bridge.snapshot_manifest,
        MANIFEST_CAP,
        &bridge.snapshot_manifest_sha256,
        "snapshot manifest",
    )?;
    for source in &bridge.reference_sources {
        verify_file(&source.path, SOURCE_CAP, &source.sha256, &source.role)?;
    }
    Ok(())
}

fn bind_snapshot(bridge: &BridgeManifest) -> Result<SnapshotManifest, String> {
    let snapshot = crate::decode::read_external_reference_manifest(&bridge.snapshot_manifest)?;
    validate_binding(bridge, &snapshot)?;
    validate_snapshot_review(bridge, &snapshot)?;
    // The reviewed decoder validates both snapshot lengths before allocating either state.
    crate::decode::preflight(&snapshot, &snapshot)?;
    Ok(snapshot)
}

pub(crate) fn preflight_output(input: &BoundInput) -> PreflightOutput<'_> {
    PreflightOutput {
        schema: "p10-external-reference-bridge-output-v1",
        status: "preflight-only",
        bridge: bridge_output(&input.bridge),
        snapshot_identity: &input.snapshot.identity,
        snapshot_source_commit: &input.snapshot.source_commit,
        snapshot_plan_sha256: &input.snapshot.plan_sha256,
        snapshot_coefficient_sha256: &input.snapshot.coefficient_sha256,
        snapshot_file_sha256: &input.snapshot.file_sha256,
        preflight: input.preflight,
        reference_assignments: 0,
        resume_or_import_interfaces: 0,
    }
}

pub(crate) fn execute(input: &BoundInput, cap: usize) -> Result<DiagnosticOutput<'_>, String> {
    if input.preflight.storage.total > cap {
        return Err(format!(
            "bridge reservation {} exceeds cap {cap}",
            input.preflight.storage.total
        ));
    }
    verify_current_executable(&input.bridge.binary_sha256)?;
    eprintln!("{{\"event\":\"snapshot_decode_start\"}}");
    let actual = crate::decode::load(&input.snapshot)?;
    eprintln!("{{\"event\":\"snapshot_decode_complete\"}}");
    let reference = produce_reference(&input.bridge, &input.preflight)?;
    let domain = input.snapshot.domain()?;
    let comparison = ComparisonPlan::new(domain, domain)
        .map_err(debug)?
        .compare(
            actual.coefficients.each_ref().map(Vec::as_slice),
            reference.each_ref().map(Vec::as_slice),
        )
        .map_err(debug)?;
    let actual_norms =
        crate::absolute::measure(domain, actual.coefficients.each_ref().map(Vec::as_slice))?;
    let reference_norms =
        crate::absolute::measure(domain, reference.each_ref().map(Vec::as_slice))?;
    Ok(DiagnosticOutput {
        schema: "p10-external-reference-bridge-output-v1",
        status: "sampled-reference-diagnostic-complete",
        bridge: bridge_output(&input.bridge),
        snapshot_identity: &input.snapshot.identity,
        snapshot_source_commit: &input.snapshot.source_commit,
        snapshot_plan_sha256: &input.snapshot.plan_sha256,
        snapshot_coefficient_sha256: &input.snapshot.coefficient_sha256,
        snapshot_file_sha256: &input.snapshot.file_sha256,
        difference: comparison.full.into(),
        actual: actual_norms,
        sampled_reference: reference_norms,
        signed_reference_minus_actual_mean: comparison.mean_error,
        preflight: input.preflight,
        reference_assignments: 0,
        resume_or_import_interfaces: 0,
    })
}

fn bridge_output(bridge: &BridgeManifest) -> BridgeOutput<'_> {
    let source = |index: usize| SourceOutput {
        role: &bridge.reference_sources[index].role,
        sha256: &bridge.reference_sources[index].sha256,
    };
    BridgeOutput {
        case_sha256: &bridge.case_sha256,
        reference_source_commit: &bridge.reference_source_commit,
        reference_sources: [
            source(0),
            source(1),
            source(2),
            source(3),
            source(4),
            source(5),
            source(6),
        ],
        harness_source_commit: &bridge.harness_source_commit,
        binary_sha256: &bridge.binary_sha256,
        reference_evaluator: &bridge.reference_evaluator,
        arithmetic: &bridge.arithmetic,
        sample_dimensions: bridge.sample_dimensions,
        retained_dimensions: bridge.retained_dimensions,
        physical_grid: &bridge.physical_grid,
        fft_backend: &bridge.fft_backend,
        fft_normalization: &bridge.fft_normalization,
        crop: &bridge.crop,
        nyquist: &bridge.nyquist,
        projection: &bridge.projection,
        coordinate_shift: bridge.coordinate_shift,
        mean_alignment: bridge.mean_alignment,
        classification: &bridge.classification,
        execution_context: &bridge.execution_context,
        execution_cap_bytes: bridge.execution_cap_bytes,
        clock_exponent: bridge.clock_exponent,
        clock_target: bridge.clock_target,
        elapsed: bridge.elapsed,
    }
}

fn preflight(bridge: &BridgeManifest, snapshot: &SnapshotManifest) -> Result<Preflight, String> {
    let samples = Layout::new(bridge.sample_dimensions).map_err(debug)?;
    let retained = snapshot.domain()?.layout();
    let backend = backend(bridge)?;
    let decoded = crate::decode::state_bytes(snapshot)?;
    let physical = bytes(samples.real_len(), 3 * size_of::<f64>())?;
    let fft_output = bytes(samples.half_len(), size_of::<Complex64>())?;
    let retained_reference = bytes(retained.half_len(), 3 * size_of::<Complex64>())?;
    let catalog = FftCatalog::reservation(backend).map_err(debug)?;
    let fft = FftPlan::reservation_with_shared_backend(samples, backend).map_err(debug)?;
    let total = [
        decoded,
        SNAPSHOT_DECODER_OVERHEAD,
        physical,
        catalog,
        fft,
        fft_output,
        retained_reference,
        ALLOCATOR_ALLOWANCE,
        OUTPUT_CAP,
    ]
    .into_iter()
    .try_fold(0usize, checked_add)?;
    let comparison = ComparisonPlan::new(snapshot.domain()?, snapshot.domain()?).map_err(debug)?;
    Ok(Preflight {
        storage: Storage {
            decoded_actual_state: decoded,
            snapshot_decoder_overhead: SNAPSHOT_DECODER_OVERHEAD,
            physical_reference_velocity: physical,
            fft_catalog: catalog,
            fft_plan_and_workspace: fft,
            fft_output_spectrum: fft_output,
            retained_reference_velocity: retained_reference,
            allocator_allowance: ALLOCATOR_ALLOWANCE,
            serialization: OUTPUT_CAP,
            total,
        },
        work: Work {
            reference_evaluations: samples.real_len(),
            maximum_root_iterations: samples
                .real_len()
                .checked_mul(128)
                .ok_or("root work overflow")?,
            scalar_forward_transforms: 3,
            transfer_coefficient_visits: samples
                .half_len()
                .checked_mul(3)
                .ok_or("transfer work overflow")?,
            comparison_work_units: comparison.work_units(),
        },
    })
}

fn bytes(count: usize, width: usize) -> Result<usize, String> {
    count
        .checked_mul(width)
        .ok_or_else(|| "storage overflow".into())
}

fn checked_add(total: usize, value: usize) -> Result<usize, String> {
    total
        .checked_add(value)
        .ok_or_else(|| "storage overflow".into())
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn write_json(value: &impl Serialize) -> Result<String, String> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(OUTPUT_CAP).map_err(debug)?;
    serde_json::to_writer_pretty(&mut bytes, value).map_err(debug)?;
    bytes.write_all(b"\n").map_err(debug)?;
    if bytes.len() > OUTPUT_CAP {
        return Err("bridge output exceeds 64 KiB bound".into());
    }
    String::from_utf8(bytes).map_err(debug)
}

#[cfg(test)]
mod bridge_tests;
