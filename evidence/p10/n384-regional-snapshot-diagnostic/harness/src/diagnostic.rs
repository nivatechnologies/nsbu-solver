use crate::{
    cache::PackedReference,
    decode,
    model::{self, Manifest},
};
use nsbu_solver::{
    diagnostics::derivatives::DerivativeWorkspace,
    domain::Layout,
    spectral::{FftBackend, FftCatalog},
};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

mod measurement;
mod policy;

use measurement::{coverage, measure, sample_reference};
use policy::{
    coefficient_hash, current_executable_hash, launch_gate, launch_policy, no_acceptance,
    snapshot_binding, storage, validate_snapshot, verify_manifest_hash, work,
};

pub(crate) const CAP_BYTES: usize = 128_771_370_072;
const SCOPED_RLIMIT_AS_BYTES: usize = 137_438_953_472;
const EXTRA_MEMORY_GATE: usize = 16 * 1024 * 1024 * 1024;
const OUTPUT_CAP: usize = 64 * 1024;
const WORKERS: usize = 32;
const STACK_BYTES: usize = 2 * 1024 * 1024;
const ROOT_BUDGET: usize = 128;
const DIMENSION: usize = 768;
const M384_SNAPSHOT_MANIFEST_SHA256: &str =
    "cc9e328b6b0dcd4518c27cd2724e839f50a7d64539ed45e7ac902dfd68c71d21";
const M384_SNAPSHOT_FILE_SHA256: &str =
    "951be3d85acd3179c2e152a11220e5231ead5d5b8a709217da83b988e9abc243";
const M384_COEFFICIENT_SHA256: &str =
    "5f559ad2e80747f102c1bf426211ca2313a89ac63b35cfecf4db723aaf57b44a";
const M384_SOURCE_COMMIT: &str = "aed49b7d7874a0a720dee88b65ba180c7286fa65";
const M384_PLAN_SHA256: &str = "2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a";
const M384_PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";
const M512_SNAPSHOT_MANIFEST_SHA256: &str =
    "92bf674356740ae230fe10eab617c9d9c1bdcb4edf84c2f446983a12d148f4d1";
const M512_SNAPSHOT_FILE_SHA256: &str =
    "7a1d8d21e17c85c7f37ea474f5f5e694a91889ebabcec12424427308d020def9";
const M512_COEFFICIENT_SHA256: &str =
    "4fbfa9890470ab61dca7ddbb026fd1f93c2f0959d15bf87e713ee0a9111c02af";
const M512_SOURCE_COMMIT: &str = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72";
const M512_PLAN_SHA256: &str = "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84";
const M512_PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";

const DECODED_BYTES: usize = 1_366_032_384;
const DECODER_OVERHEAD: usize = 1_048_576;
const FFT_CATALOG_BYTES: usize = 29_362_480;
const DERIVATIVE_WORKSPACE_BUDGET: usize = 10_890_584_880;
const PACKED_CACHE_BYTES: usize = 108_716_359_680;
const MAGNITUDE_BYTES: usize = 7_247_757_312;
const LABEL_BYTES: usize = 452_984_832;
const WORKER_STACK_BYTES: usize = 67_108_864;
const ALLOCATOR_ALLOWANCE: usize = 65_528;

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Storage {
    decoded_actual_coefficients: usize,
    snapshot_decoder_overhead: usize,
    avx_fft_catalog: usize,
    single_derivative_workspace_required: usize,
    single_derivative_workspace_budget: usize,
    derivative_workspace_reservation_allowance: usize,
    packed_reference_cache: usize,
    two_magnitude_arrays: usize,
    cached_region_labels: usize,
    worker_stacks: usize,
    allocator_allowance: usize,
    serialization: usize,
    total: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Work {
    sample_points: usize,
    reference_evaluations: usize,
    maximum_reference_root_iterations: usize,
    ordered_hessian_mapping_comparisons: usize,
    classifications: usize,
    maximum_classification_root_iterations: usize,
    scalar_inverse_transforms: usize,
    coefficient_visits_per_transform: usize,
    derivative_coefficient_visits: usize,
    tensor_component_point_visits: usize,
    regional_magnitude_pushes: usize,
    coverage_geometry_evaluations: usize,
}

pub(crate) struct BoundInput {
    manifest: Manifest,
    binding: &'static ReviewedSnapshot,
    storage: Storage,
    work: Work,
    binary_sha256: String,
}

#[derive(Clone, Copy, Debug)]
struct ReviewedSnapshot {
    manifest_sha256: &'static str,
    file_sha256: &'static str,
    coefficient_sha256: &'static str,
    source_commit: &'static str,
    plan_sha256: &'static str,
    profile: &'static str,
    integration_force_dimension: usize,
}

const REVIEWED_SNAPSHOTS: [ReviewedSnapshot; 2] = [
    ReviewedSnapshot {
        manifest_sha256: M384_SNAPSHOT_MANIFEST_SHA256,
        file_sha256: M384_SNAPSHOT_FILE_SHA256,
        coefficient_sha256: M384_COEFFICIENT_SHA256,
        source_commit: M384_SOURCE_COMMIT,
        plan_sha256: M384_PLAN_SHA256,
        profile: M384_PROFILE,
        integration_force_dimension: 384,
    },
    ReviewedSnapshot {
        manifest_sha256: M512_SNAPSHOT_MANIFEST_SHA256,
        file_sha256: M512_SNAPSHOT_FILE_SHA256,
        coefficient_sha256: M512_COEFFICIENT_SHA256,
        source_commit: M512_SOURCE_COMMIT,
        plan_sha256: M512_PLAN_SHA256,
        profile: M512_PROFILE,
        integration_force_dimension: 512,
    },
];

#[derive(Debug, Serialize)]
pub(crate) struct PreflightOutput<'a> {
    schema: &'static str,
    status: &'static str,
    snapshot: SnapshotBinding<'a>,
    sample_dimensions: [usize; 3],
    quantities: [&'static str; 3],
    diagnostic_binary_sha256: &'a str,
    storage: Storage,
    work: Work,
    launch: LaunchPolicy,
    collar_volume_coverage: &'static str,
    peak_qualification: &'static str,
    acceptance: Acceptance,
    state_import_or_resume_interfaces: usize,
}

#[derive(Debug, Serialize)]
struct SnapshotBinding<'a> {
    identity: &'a str,
    source_commit: &'a str,
    plan_sha256: &'a str,
    coefficient_sha256: &'a str,
    file_sha256: &'a str,
    profile: &'a str,
    elapsed: u128,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct LaunchPolicy {
    exact_cap_bytes: usize,
    external_address_space_limit_bytes: usize,
    minimum_mem_available_bytes: usize,
    workers: usize,
    stack_bytes_per_worker: usize,
    hard_timeout_seconds: usize,
    root_review_required: bool,
    residual_localization_must_be_idle: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Acceptance {
    status: &'static str,
    accepted_windows: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct DiagnosticOutput<'a> {
    schema: &'static str,
    status: &'static str,
    snapshot: SnapshotBinding<'a>,
    sample_dimensions: [usize; 3],
    region_counts: [RegionCount; 5],
    quantities: [QuantityReport; 3],
    diagnostic_binary_sha256: &'a str,
    coverage: [CoverageOutput; 6],
    storage: Storage,
    work: Work,
    post_measurement_coefficient_sha256: String,
    immutable_coefficient_hash_equal: bool,
    collar_volume_coverage: &'static str,
    peak_qualification: &'static str,
    acceptance: Acceptance,
    state_import_or_resume_interfaces: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct RegionCount {
    region: &'static str,
    samples: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct QuantityReport {
    quantity: Quantity,
    scalar_inverse_transforms: usize,
    relative_floor: f64,
    global: SampleOutput,
    regions: [RegionOutput; 5],
}

#[derive(Clone, Copy, Debug, Serialize)]
struct RegionOutput {
    region: &'static str,
    sampled: SampleOutput,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SampleOutput {
    NoSamples,
    Measured {
        components: usize,
        samples: usize,
        rms_error: f64,
        peak_error: f64,
        peak_relative_error: f64,
        reference_peak: f64,
        relative_floor: f64,
    },
}

#[derive(Clone, Copy, Debug, Serialize)]
struct CoverageOutput {
    region: &'static str,
    requested_panels: usize,
    fine_panels: usize,
    status: &'static str,
    fraction: f64,
    refinement_change: f64,
    evaluations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Quantity {
    Velocity,
    Gradient,
    OrderedHessian,
}

impl Quantity {
    fn components(self) -> usize {
        match self {
            Self::Velocity => 3,
            Self::Gradient => 9,
            Self::OrderedHessian => 27,
        }
    }

    fn transforms(self) -> usize {
        self.components()
    }

    fn absolute_floor(self) -> f64 {
        match self {
            Self::Velocity => 1e-8,
            Self::Gradient => 1e-7,
            Self::OrderedHessian => 1e-6,
        }
    }

    fn entry(self, index: usize) -> (usize, [u8; 3]) {
        let mut orders = [0; 3];
        match self {
            Self::Velocity => (index, orders),
            Self::Gradient => {
                orders[index % 3] = 1;
                (index / 3, orders)
            }
            Self::OrderedHessian => {
                orders[(index / 3) % 3] += 1;
                orders[index % 3] += 1;
                (index / 9, orders)
            }
        }
    }

    fn reference(self, packed: PackedReference, index: usize) -> f64 {
        match self {
            Self::Velocity => packed.velocity(index),
            Self::Gradient => packed.gradient(index / 3, index % 3),
            Self::OrderedHessian => packed.hessian(index / 9, (index / 3) % 3, index % 3),
        }
    }
}

pub(crate) fn bind(path: &Path, cap: usize, require_state: bool) -> Result<BoundInput, String> {
    if cap != CAP_BYTES {
        return Err(format!(
            "CAP_BYTES {cap} does not equal exact reservation {CAP_BYTES}"
        ));
    }
    verify_manifest_hash(path)?;
    let manifest = decode::read_external_reference_manifest(path)?;
    let binding = validate_snapshot(&manifest)?;
    let binary_sha256 = current_executable_hash()?;
    if require_state {
        launch_gate(&binary_sha256)?;
        decode::preflight(&manifest, &manifest)?;
    }
    let storage = storage()?;
    let work = work(&manifest)?;
    Ok(BoundInput {
        manifest,
        binding,
        storage,
        work,
        binary_sha256,
    })
}

pub(crate) fn preflight_output(input: &BoundInput) -> PreflightOutput<'_> {
    PreflightOutput {
        schema: "p10-n384-regional-snapshot-diagnostic-v1",
        status: "bound-preflight-only-state-file-not-read",
        snapshot: snapshot_binding(&input.manifest),
        sample_dimensions: [DIMENSION; 3],
        quantities: ["velocity", "gradient", "ordered_hessian"],
        diagnostic_binary_sha256: &input.binary_sha256,
        storage: input.storage,
        work: input.work,
        launch: launch_policy(),
        collar_volume_coverage: "not_assessed_sampled_collar_errors_and_counts_only",
        peak_qualification: "not_assessed",
        acceptance: no_acceptance(),
        state_import_or_resume_interfaces: 0,
    }
}

pub(crate) fn execute(input: &BoundInput) -> Result<DiagnosticOutput<'_>, String> {
    let snapshot = decode::load(&input.manifest)?;
    let domain = input.manifest.domain()?;
    let samples = Layout::new([DIMENSION; 3]).map_err(model::debug)?;
    let mut cache = try_zeros(samples.real_len())?;
    let mut labels = try_zeros(samples.real_len())?;
    let region_counts = sample_reference(&mut cache, &mut labels, snapshot.clock)?;
    let catalog =
        FftCatalog::new(FftBackend::RustFft6_4_1AvxFma, FFT_CATALOG_BYTES).map_err(model::debug)?;
    let reservation = DerivativeWorkspace::reservation_from_catalog(domain, samples, &catalog)
        .map_err(model::debug)?;
    if reservation > DERIVATIVE_WORKSPACE_BUDGET {
        return Err(format!(
            "derivative workspace reservation {reservation} exceeds admitted budget {DERIVATIVE_WORKSPACE_BUDGET}"
        ));
    }
    let mut workspace = DerivativeWorkspace::new_from_catalog(
        domain,
        samples,
        &catalog,
        DERIVATIVE_WORKSPACE_BUDGET,
    )
    .map_err(model::debug)?;
    let mut errors = try_zeros(samples.real_len())?;
    let mut references = try_zeros(samples.real_len())?;
    let coefficients = snapshot.coefficients.each_ref().map(Vec::as_slice);
    let quantities = [
        measure::<3>(
            Quantity::Velocity,
            coefficients,
            &cache,
            &labels,
            &mut workspace,
            &mut errors,
            &mut references,
        )?,
        measure::<9>(
            Quantity::Gradient,
            coefficients,
            &cache,
            &labels,
            &mut workspace,
            &mut errors,
            &mut references,
        )?,
        measure::<27>(
            Quantity::OrderedHessian,
            coefficients,
            &cache,
            &labels,
            &mut workspace,
            &mut errors,
            &mut references,
        )?,
    ];
    let coverage = coverage(snapshot.clock)?;
    let post_hash = coefficient_hash(coefficients);
    if post_hash != input.binding.coefficient_sha256 || post_hash != snapshot.coefficient_sha256 {
        return Err("post-measurement immutable coefficient hash mismatch".into());
    }
    Ok(DiagnosticOutput {
        schema: "p10-n384-regional-snapshot-diagnostic-v1",
        status: "sampled_read_only_diagnostic_complete",
        snapshot: snapshot_binding(&input.manifest),
        sample_dimensions: [DIMENSION; 3],
        region_counts,
        quantities,
        diagnostic_binary_sha256: &input.binary_sha256,
        coverage,
        storage: input.storage,
        work: input.work,
        post_measurement_coefficient_sha256: post_hash,
        immutable_coefficient_hash_equal: true,
        collar_volume_coverage: "not_assessed_sampled_collar_errors_and_counts_only",
        peak_qualification: "not_assessed",
        acceptance: no_acceptance(),
        state_import_or_resume_interfaces: 0,
    })
}

fn try_zeros<T: Clone + Default>(length: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "bounded diagnostic allocation failed")?;
    values.resize(length, T::default());
    Ok(values)
}

pub(crate) fn json(value: &impl Serialize) -> Result<String, String> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(OUTPUT_CAP)
        .map_err(|_| "output allocation failed")?;
    serde_json::to_writer_pretty(&mut bytes, value).map_err(model::debug)?;
    bytes.write_all(b"\n").map_err(model::debug)?;
    if bytes.len() > OUTPUT_CAP {
        return Err("diagnostic output exceeds 64 KiB".into());
    }
    String::from_utf8(bytes).map_err(model::debug)
}

pub(crate) fn write_transactional(path: &PathBuf, value: &impl Serialize) -> Result<(), String> {
    let body = json(value)?;
    let candidate = path.with_extension("candidate");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&candidate)
        .map_err(model::debug)?;
    file.write_all(body.as_bytes()).map_err(model::debug)?;
    file.sync_all().map_err(model::debug)?;
    fs::rename(candidate, path).map_err(model::debug)
}

#[cfg(test)]
mod tests;
