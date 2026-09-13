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
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

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
const REFERENCE_SOURCE_COMMIT: &str = "6bdea3084d737ce6585cb67ab5d48810bd03cd50";
const REFERENCE_SOURCE_SHA256: &str =
    "2bd8e841dc5b2b1bd4a301ab8bfc392b81716ce58f5c019b8a35b4c9ff599bb8";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BridgeManifest {
    pub schema: String,
    pub snapshot_manifest: PathBuf,
    pub snapshot_manifest_sha256: String,
    pub case_sha256: String,
    pub reference_source_commit: String,
    pub reference_source: PathBuf,
    pub reference_source_sha256: String,
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
    pub bridge: &'a BridgeManifest,
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
    pub bridge: &'a BridgeManifest,
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
    resolve(path, &mut bridge.reference_source);
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
    verify_file(
        &bridge.reference_source,
        SOURCE_CAP,
        &bridge.reference_source_sha256,
        "reference source",
    )?;
    Ok(())
}

fn bind_snapshot(bridge: &BridgeManifest) -> Result<SnapshotManifest, String> {
    let snapshot = crate::decode::read_manifest(&bridge.snapshot_manifest)?;
    validate_binding(bridge, &snapshot)?;
    // The reviewed decoder validates both snapshot lengths before allocating either state.
    crate::decode::preflight(&snapshot, &snapshot)?;
    Ok(snapshot)
}

pub(crate) fn preflight_output(input: &BoundInput) -> PreflightOutput<'_> {
    PreflightOutput {
        schema: "p10-external-reference-bridge-output-v1",
        status: "preflight-only",
        bridge: &input.bridge,
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
    let actual = crate::decode::load(&input.snapshot)?;
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
        bridge: &input.bridge,
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

fn validate_bridge(bridge: &BridgeManifest) -> Result<(), String> {
    let identity = [
        bridge.schema == "p10-external-reference-bridge-input-v1",
        hex(&bridge.snapshot_manifest_sha256, 64),
        hex(&bridge.case_sha256, 64),
        hex(&bridge.reference_source_commit, 40),
        hex(&bridge.reference_source_sha256, 64),
        bridge.reference_source_commit == REFERENCE_SOURCE_COMMIT,
        bridge.reference_source_sha256 == REFERENCE_SOURCE_SHA256,
        bridge.case_sha256 == CASE_SHA256,
    ];
    let arithmetic = [
        bridge.reference_evaluator == EVALUATOR,
        bridge.arithmetic == ARITHMETIC,
        bridge.physical_grid == GRID,
        bridge.fft_normalization == NORMALIZATION,
        bridge.crop == CROP,
        bridge.nyquist == NYQUIST,
        bridge.projection == PROJECTION,
        !bridge.coordinate_shift,
        !bridge.mean_alignment,
        bridge.classification == CLASSIFICATION,
    ];
    let clock = bridge.elapsed > 0 && bridge.elapsed < bridge.clock_target;
    if !identity.into_iter().all(std::convert::identity)
        || !arithmetic.into_iter().all(std::convert::identity)
        || !clock
    {
        return Err("invalid external reference bridge binding".into());
    }
    let (Ok(samples), Ok(retained)) = (
        Layout::new(bridge.sample_dimensions),
        Layout::new(bridge.retained_dimensions),
    ) else {
        return Err("invalid external reference bridge dimensions".into());
    };
    if retained
        .dimensions()
        .into_iter()
        .zip(samples.dimensions())
        .any(|(n, m)| n > m)
    {
        return Err("reference sample grid is smaller than retained grid".into());
    }
    backend(bridge)?;
    Ok(())
}

fn validate_binding(bridge: &BridgeManifest, snapshot: &SnapshotManifest) -> Result<(), String> {
    let matched = [
        snapshot.dimensions == bridge.retained_dimensions,
        snapshot.evolution.case_sha256 == bridge.case_sha256,
        snapshot.evolution.quantum_exponent == bridge.clock_exponent,
        snapshot.target == bridge.clock_target,
        snapshot.elapsed == bridge.elapsed,
        snapshot.evolution.comparison_endpoint == bridge.elapsed,
        snapshot.evolution.lengths == [1.0; 3],
        snapshot.evolution.viscosity == 1.0,
    ];
    if !matched.into_iter().all(std::convert::identity) {
        return Err("snapshot/reference binding mismatch".into());
    }
    TickClock::restore(
        bridge.clock_exponent,
        bridge.clock_target,
        bridge.elapsed,
        bridge.clock_target - bridge.elapsed,
    )
    .map_err(debug)?;
    Ok(())
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

fn produce_reference(
    bridge: &BridgeManifest,
    preflight: &Preflight,
) -> Result<[Vec<Complex64>; 3], String> {
    let samples = Layout::new(bridge.sample_dimensions).map_err(debug)?;
    let retained = Layout::new(bridge.retained_dimensions).map_err(debug)?;
    let backend = backend(bridge)?;
    let catalog = FftCatalog::new(backend, preflight.storage.fft_catalog).map_err(debug)?;
    let (plan, mut workspace) =
        FftPlan::new_from_catalog(samples, &catalog, preflight.storage.fft_plan_and_workspace)
            .map_err(debug)?;
    let mut physical = three_zeros(samples.real_len())?;
    sample_velocity(bridge, &mut physical)?;
    let mut spectrum = try_zeros(samples.half_len())?;
    let mut result = three_zeros(retained.half_len())?;
    for component in 0..3 {
        plan.forward(&physical[component], &mut spectrum, &mut workspace)
            .map_err(debug)?;
        transfer(samples, retained, &spectrum, &mut result[component]).map_err(debug)?;
    }
    Ok(result)
}

fn sample_velocity(bridge: &BridgeManifest, output: &mut [Vec<f64>; 3]) -> Result<(), String> {
    let clock = TickClock::restore(
        bridge.clock_exponent,
        bridge.clock_target,
        bridge.elapsed,
        bridge.clock_target - bridge.elapsed,
    )
    .map_err(debug)?;
    let time = BenchmarkTime::new(clock).map_err(debug)?;
    let [nx, ny, nz] = bridge.sample_dimensions;
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let index = (i * ny + j) * nz + k;
                let point = [
                    i as f64 / nx as f64,
                    j as f64 / ny as f64,
                    k as f64 / nz as f64,
                ];
                let velocity = scalar::evaluate(point, time).map_err(debug)?.velocity;
                for component in 0..3 {
                    output[component][index] = velocity[component];
                }
            }
        }
    }
    Ok(())
}

fn backend(bridge: &BridgeManifest) -> Result<FftBackend, String> {
    match bridge.fft_backend.as_str() {
        "rustfft-6.4.1-avx-avx2-fma" => Ok(FftBackend::RustFft6_4_1AvxFma),
        "project-owned-mixed-radix-binary64" => Ok(FftBackend::OwnedRadix),
        _ => Err("unsupported FFT backend binding".into()),
    }
}

fn resolve(manifest: &Path, value: &mut PathBuf) {
    if value.is_relative() {
        *value = manifest
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&*value);
    }
}

fn verify_file(path: &Path, cap: u64, expected: &str, label: &str) -> Result<(), String> {
    let bytes = read_bounded(path, cap, &format!("{label} exceeds byte bound"))?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err(format!("{label} SHA-256 mismatch"));
    }
    Ok(())
}

fn read_bounded(path: &Path, cap: u64, message: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(debug)?;
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes).map_err(debug)?;
    if bytes.len() as u64 > cap {
        return Err(message.into());
    }
    Ok(bytes)
}

fn try_zeros<T: Clone + Default>(length: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "bounded allocation failed")?;
    values.resize(length, T::default());
    Ok(values)
}

fn three_zeros<T: Clone + Default>(length: usize) -> Result<[Vec<T>; 3], String> {
    Ok([try_zeros(length)?, try_zeros(length)?, try_zeros(length)?])
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
