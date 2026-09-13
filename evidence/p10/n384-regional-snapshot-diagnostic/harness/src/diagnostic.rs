use crate::{
    cache::PackedReference,
    decode,
    model::{self, ComparisonKind, Manifest, ProfileBindingKind},
};
use nsbu_benchmarks::{
    fields::reference,
    regions::{classify, CoveragePlan, CoverageStatus, NominalRegion, SpatialRegion},
    time::BenchmarkTime,
    CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError, TensorErrors},
    },
    domain::{Domain, Layout, TickClock},
    spectral::{FftBackend, FftCatalog},
    Complex64,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) const CAP_BYTES: usize = 128_771_370_072;
const SCOPED_RLIMIT_AS_BYTES: usize = 137_438_953_472;
const EXTRA_MEMORY_GATE: usize = 16 * 1024 * 1024 * 1024;
const OUTPUT_CAP: usize = 64 * 1024;
const WORKERS: usize = 32;
const STACK_BYTES: usize = 2 * 1024 * 1024;
const ROOT_BUDGET: usize = 128;
const DIMENSION: usize = 768;
const SNAPSHOT_MANIFEST_SHA256: &str =
    "cc9e328b6b0dcd4518c27cd2724e839f50a7d64539ed45e7ac902dfd68c71d21";
const SNAPSHOT_FILE_SHA256: &str =
    "951be3d85acd3179c2e152a11220e5231ead5d5b8a709217da83b988e9abc243";
const COEFFICIENT_SHA256: &str = "5f559ad2e80747f102c1bf426211ca2313a89ac63b35cfecf4db723aaf57b44a";
const SOURCE_COMMIT: &str = "aed49b7d7874a0a720dee88b65ba180c7286fa65";
const PLAN_SHA256: &str = "2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a";
const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";

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
    storage: Storage,
    work: Work,
    binary_sha256: String,
}

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
    validate_snapshot(&manifest)?;
    let binary_sha256 = current_executable_hash()?;
    if require_state {
        launch_gate(&binary_sha256)?;
        decode::preflight(&manifest, &manifest)?;
    }
    let storage = storage()?;
    let work = work(&manifest)?;
    Ok(BoundInput {
        manifest,
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
    if post_hash != COEFFICIENT_SHA256 || post_hash != snapshot.coefficient_sha256 {
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

fn sample_reference(
    cache: &mut [PackedReference],
    labels: &mut [u8],
    header: model::ClockHeader,
) -> Result<[RegionCount; 5], String> {
    if cache.len() != DIMENSION.pow(3) || labels.len() != cache.len() {
        return Err("reference cache shape mismatch".into());
    }
    let clock = TickClock::restore(-20, 8192, header.elapsed, 8192 - header.elapsed)
        .map_err(model::debug)?;
    let time = BenchmarkTime::new(clock).map_err(model::debug)?;
    let plane = DIMENSION * DIMENSION;
    let chunk = 24 * plane;
    let counts = std::thread::scope(|scope| -> Result<[usize; 5], String> {
        let mut handles = Vec::new();
        handles
            .try_reserve_exact(WORKERS)
            .map_err(|_| "worker handle allocation failed")?;
        for (worker, (cache_chunk, label_chunk)) in cache
            .chunks_mut(chunk)
            .zip(labels.chunks_mut(chunk))
            .enumerate()
        {
            let start_plane = worker * 24;
            let handle = std::thread::Builder::new()
                .name(format!("regional-reference-{worker:02}"))
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, move || {
                    sample_reference_chunk(cache_chunk, label_chunk, start_plane, clock, time)
                })
                .map_err(model::debug)?;
            handles.push(handle);
        }
        let mut total = [0_usize; 5];
        for handle in handles {
            let local = handle.join().map_err(|_| "reference worker panicked")??;
            for (sum, value) in total.iter_mut().zip(local) {
                *sum = sum.checked_add(value).ok_or("region count overflow")?;
            }
        }
        Ok(total)
    })?;
    if counts.iter().sum::<usize>() != cache.len() {
        return Err("regional classification count mismatch".into());
    }
    Ok(region_counts(counts))
}

fn sample_reference_chunk(
    cache: &mut [PackedReference],
    labels: &mut [u8],
    start_plane: usize,
    clock: TickClock,
    time: BenchmarkTime,
) -> Result<[usize; 5], String> {
    let plane = DIMENSION * DIMENSION;
    if cache.len() != labels.len() || !cache.len().is_multiple_of(plane) {
        return Err("reference worker chunk shape mismatch".into());
    }
    let mut counts = [0_usize; 5];
    for (local, (packed, label)) in cache.iter_mut().zip(labels).enumerate() {
        let i = start_plane + local / plane;
        let within = local % plane;
        let j = within / DIMENSION;
        let k = within % DIMENSION;
        let point = [
            i as f64 / DIMENSION as f64,
            j as f64 / DIMENSION as f64,
            k as f64 / DIMENSION as f64,
        ];
        let independent = reference::evaluate(point, time).map_err(model::debug)?;
        *packed = PackedReference::pack_checked(&independent)?;
        let index = region_index(
            classify(point, clock, ROOT_BUDGET)
                .map_err(model::debug)?
                .spatial,
        );
        *label = u8::try_from(index).map_err(model::debug)?;
        counts[index] += 1;
    }
    Ok(counts)
}

#[allow(clippy::too_many_arguments)]
fn measure<const COMPONENTS: usize>(
    quantity: Quantity,
    coefficients: [&[Complex64]; 3],
    cache: &[PackedReference],
    labels: &[u8],
    workspace: &mut DerivativeWorkspace,
    errors: &mut [f64],
    references: &mut [f64],
) -> Result<QuantityReport, String> {
    if quantity.components() != COMPONENTS
        || cache.len() != errors.len()
        || cache.len() != references.len()
        || cache.len() != labels.len()
    {
        return Err("quantity workspace shape mismatch".into());
    }
    errors.fill(0.0);
    references.fill(0.0);
    for entry in 0..COMPONENTS {
        let (component, orders) = quantity.entry(entry);
        let actual = workspace
            .sample(
                coefficients[component],
                Derivative::new(orders).map_err(model::debug)?,
            )
            .map_err(model::debug)?;
        for (index, ((error, reference_magnitude), packed)) in errors
            .iter_mut()
            .zip(references.iter_mut())
            .zip(cache)
            .enumerate()
        {
            let expected = quantity.reference(*packed, entry);
            let difference = actual.values[index] - expected;
            *error = error.hypot(difference);
            *reference_magnitude = reference_magnitude.hypot(expected);
            if !error.is_finite() || !reference_magnitude.is_finite() {
                return Err("non-finite pointwise tensor magnitude".into());
            }
        }
    }
    let reference_peak = references.iter().copied().fold(0.0, f64::max);
    let relative_floor = quantity.absolute_floor().max(1e-3 * reference_peak);
    let (global, regions) = reduce::<COMPONENTS>(errors, references, labels, relative_floor)?;
    Ok(QuantityReport {
        quantity,
        scalar_inverse_transforms: quantity.transforms(),
        relative_floor,
        global,
        regions,
    })
}

fn reduce<const COMPONENTS: usize>(
    errors: &[f64],
    references: &[f64],
    labels: &[u8],
    floor: f64,
) -> Result<(SampleOutput, [RegionOutput; 5]), String> {
    let mut global = TensorErrors::<COMPONENTS>::new(errors.len(), floor).map_err(model::debug)?;
    let empty = TensorErrors::<COMPONENTS>::new(errors.len(), floor).map_err(model::debug)?;
    let mut regional = [empty; 5];
    for ((&error, &reference), &label) in errors.iter().zip(references).zip(labels) {
        let index = usize::from(label);
        let region = regional
            .get_mut(index)
            .ok_or("invalid cached region label")?;
        global
            .push_magnitudes(error, reference)
            .map_err(model::debug)?;
        region
            .push_magnitudes(error, reference)
            .map_err(model::debug)?;
    }
    let mut outputs = region_names().map(|region| RegionOutput {
        region,
        sampled: SampleOutput::NoSamples,
    });
    for (output, accumulator) in outputs.iter_mut().zip(regional) {
        output.sampled = sample_output(accumulator.finish().map_err(model::debug)?);
    }
    Ok((
        sample_output(global.finish().map_err(model::debug)?),
        outputs,
    ))
}

fn sample_output(value: SampledError) -> SampleOutput {
    match value {
        SampledError::NoSamples => SampleOutput::NoSamples,
        SampledError::Measured(LocalError {
            components,
            samples,
            rms_error,
            peak_error,
            peak_relative_error,
            reference_peak,
            relative_floor,
        }) => SampleOutput::Measured {
            components,
            samples,
            rms_error,
            peak_error,
            peak_relative_error,
            reference_peak,
            relative_floor,
        },
    }
}

fn coverage(clock: model::ClockHeader) -> Result<[CoverageOutput; 6], String> {
    let clock =
        TickClock::restore(-20, 8192, clock.elapsed, 8192 - clock.elapsed).map_err(model::debug)?;
    let mut output = [CoverageOutput {
        region: "",
        requested_panels: 0,
        fine_panels: 0,
        status: "",
        fraction: 0.0,
        refinement_change: 0.0,
        evaluations: 0,
    }; 6];
    let mut next = 0;
    for (name, region) in [
        ("core", NominalRegion::CORE),
        ("annulus", NominalRegion::ANNULUS),
    ] {
        for panels in [256, 512, 1024] {
            let measured = CoveragePlan::new(panels, 3 * panels + 2)
                .map_err(model::debug)?
                .evaluate(clock, region)
                .map_err(model::debug)?;
            output[next] = CoverageOutput {
                region: name,
                requested_panels: panels,
                fine_panels: measured.panels,
                status: match measured.status {
                    CoverageStatus::Nonempty => "nonempty",
                    CoverageStatus::RegionEmpty => "region_empty",
                },
                fraction: measured.fraction,
                refinement_change: measured.refinement_change,
                evaluations: measured.evaluations,
            };
            next += 1;
        }
    }
    Ok(output)
}

fn validate_snapshot(manifest: &Manifest) -> Result<(), String> {
    let profile = manifest.profile.as_ref();
    let exact = [
        manifest.comparison_kind == ComparisonKind::MatchedSpatial,
        manifest.source_commit == SOURCE_COMMIT,
        manifest.plan_sha256 == PLAN_SHA256,
        manifest.coefficient_sha256 == COEFFICIENT_SHA256,
        manifest.file_sha256 == SNAPSHOT_FILE_SHA256,
        manifest.dimensions == [384; 3],
        manifest.evolution.case_sha256 == CASE_SHA256,
        manifest.evolution.quantum_exponent == -20,
        manifest.evolution.clock_target == 8192,
        manifest.evolution.comparison_endpoint == 512,
        manifest.evolution.lengths == [1.0; 3],
        manifest.evolution.viscosity == 1.0,
        manifest.evolution.method == "cox-matthews",
        manifest.evolution.integration_force_dimensions == [384; 3],
        manifest.elapsed == 512,
        manifest.target == 8192,
        manifest.epoch == 8,
        manifest.accepted_steps == 8,
        profile.is_some_and(|value| {
            value.kind == ProfileBindingKind::IdentityProfileField && value.value == PROFILE
        }),
        manifest.evolution.schedule.len() == 1,
        manifest.evolution.schedule.first().is_some_and(|segment| {
            segment.from_inclusive == 0
                && segment.until_exclusive == 512
                && segment.step_ticks == 64
        }),
        identity_value(&manifest.identity, "source") == Some(SOURCE_COMMIT),
        identity_value(&manifest.identity, "case") == Some(CASE_SHA256),
        identity_value(&manifest.identity, "profile") == Some(PROFILE),
        identity_value(&manifest.identity, "retained") == Some("384"),
        identity_value(&manifest.identity, "force_samples") == Some("384"),
        identity_value(&manifest.identity, "observer_conservative") == Some("768"),
        manifest.backend == "rustfft-6.4.1-avx-avx2-fma",
    ];
    exact
        .into_iter()
        .all(std::convert::identity)
        .then_some(())
        .ok_or_else(|| "snapshot is not the reviewed N384/M384 clock-512 baseline".into())
}

fn verify_manifest_hash(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(model::debug)?;
    if bytes.len() > OUTPUT_CAP {
        return Err("snapshot manifest exceeds 64 KiB".into());
    }
    if format!("{:x}", Sha256::digest(bytes)) != SNAPSHOT_MANIFEST_SHA256 {
        return Err("snapshot manifest SHA-256 mismatch".into());
    }
    Ok(())
}

fn identity_value<'a>(identity: &'a str, key: &str) -> Option<&'a str> {
    let mut values = identity
        .split(';')
        .filter_map(|field| field.split_once('='))
        .filter(|(name, _)| *name == key)
        .map(|(_, value)| value);
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn storage() -> Result<Storage, String> {
    let source = Domain::new([384; 3], [1.0; 3], 1.0).map_err(model::debug)?;
    let samples = Layout::new([DIMENSION; 3]).map_err(model::debug)?;
    let derivative_required = DerivativeWorkspace::reservation_with_shared_backend(
        source,
        samples,
        FftBackend::RustFft6_4_1AvxFma,
    )
    .map_err(model::debug)?;
    let derivative_allowance = DERIVATIVE_WORKSPACE_BUDGET
        .checked_sub(derivative_required)
        .ok_or_else(|| {
            format!(
                "derivative workspace {derivative_required} exceeds reviewed budget {DERIVATIVE_WORKSPACE_BUDGET}"
            )
        })?;
    let total = [
        DECODED_BYTES,
        DECODER_OVERHEAD,
        FFT_CATALOG_BYTES,
        DERIVATIVE_WORKSPACE_BUDGET,
        PACKED_CACHE_BYTES,
        MAGNITUDE_BYTES,
        LABEL_BYTES,
        WORKER_STACK_BYTES,
        ALLOCATOR_ALLOWANCE,
        OUTPUT_CAP,
    ]
    .into_iter()
    .try_fold(0_usize, |sum, value| {
        sum.checked_add(value).ok_or("storage reservation overflow")
    })?;
    if total != CAP_BYTES
        || size_of::<PackedReference>() != 30 * size_of::<f64>()
        || DIMENSION.pow(3) * size_of::<PackedReference>() != PACKED_CACHE_BYTES
    {
        return Err("compiled storage reservation differs from reviewed exact cap".into());
    }
    Ok(Storage {
        decoded_actual_coefficients: DECODED_BYTES,
        snapshot_decoder_overhead: DECODER_OVERHEAD,
        avx_fft_catalog: FFT_CATALOG_BYTES,
        single_derivative_workspace_required: derivative_required,
        single_derivative_workspace_budget: DERIVATIVE_WORKSPACE_BUDGET,
        derivative_workspace_reservation_allowance: derivative_allowance,
        packed_reference_cache: PACKED_CACHE_BYTES,
        two_magnitude_arrays: MAGNITUDE_BYTES,
        cached_region_labels: LABEL_BYTES,
        worker_stacks: WORKER_STACK_BYTES,
        allocator_allowance: ALLOCATOR_ALLOWANCE,
        serialization: OUTPUT_CAP,
        total,
    })
}

fn work(manifest: &Manifest) -> Result<Work, String> {
    let points = DIMENSION.pow(3);
    let visits = DerivativeWorkspace::coefficient_visits(manifest.domain()?.layout())
        .map_err(model::debug)?;
    Ok(Work {
        sample_points: points,
        reference_evaluations: points,
        maximum_reference_root_iterations: points * ROOT_BUDGET,
        ordered_hessian_mapping_comparisons: points * 27,
        classifications: points,
        maximum_classification_root_iterations: points * ROOT_BUDGET,
        scalar_inverse_transforms: 39,
        coefficient_visits_per_transform: visits,
        derivative_coefficient_visits: 39 * visits,
        tensor_component_point_visits: 39 * points,
        regional_magnitude_pushes: 3 * points,
        coverage_geometry_evaluations: 10_764,
    })
}

fn launch_gate(binary_sha256: &str) -> Result<(), String> {
    if std::env::var("P10_ROOT_FULL_RUN_REVIEW").as_deref() != Ok("approved") {
        return Err("full run requires P10_ROOT_FULL_RUN_REVIEW=approved".into());
    }
    if std::env::var("P10_RESIDUAL_LOCALIZATION_IDLE").as_deref() != Ok("1") {
        return Err("full run requires P10_RESIDUAL_LOCALIZATION_IDLE=1".into());
    }
    if std::env::var("P10_CPU_WORKERS").as_deref() != Ok("32") {
        return Err("full run requires the reviewed P10_CPU_WORKERS=32 fit".into());
    }
    let limits = address_space_limits()?;
    if limits != [SCOPED_RLIMIT_AS_BYTES; 2] {
        return Err(format!(
            "full run requires actual RLIMIT_AS soft/hard limits of {SCOPED_RLIMIT_AS_BYTES}, observed {limits:?}"
        ));
    }
    if std::env::var("P10_DIAGNOSTIC_BINARY_SHA256").as_deref() != Ok(binary_sha256) {
        return Err(
            "full run requires P10_DIAGNOSTIC_BINARY_SHA256 to match the running executable".into(),
        );
    }
    let available_cpus = std::thread::available_parallelism()
        .map_err(model::debug)?
        .get();
    if available_cpus < WORKERS {
        return Err(format!(
            "CPU fit exposes {available_cpus} workers, fewer than required {WORKERS}"
        ));
    }
    let available_memory = mem_available()?;
    let required = CAP_BYTES
        .checked_add(EXTRA_MEMORY_GATE)
        .ok_or("memory gate overflow")?;
    if available_memory < required {
        return Err(format!(
            "MemAvailable {available_memory} is below required {required}"
        ));
    }
    Ok(())
}

fn mem_available() -> Result<usize, String> {
    let text = fs::read_to_string("/proc/meminfo").map_err(model::debug)?;
    let line = text
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))
        .ok_or("MemAvailable is missing")?;
    let kib = line
        .split_ascii_whitespace()
        .nth(1)
        .ok_or("MemAvailable value is missing")?
        .parse::<usize>()
        .map_err(model::debug)?;
    kib.checked_mul(1024)
        .ok_or_else(|| "MemAvailable overflow".into())
}

fn address_space_limits() -> Result<[usize; 2], String> {
    let text = fs::read_to_string("/proc/self/limits").map_err(model::debug)?;
    parse_address_space_limits(&text)
}

fn parse_address_space_limits(text: &str) -> Result<[usize; 2], String> {
    let fields = text
        .lines()
        .find(|line| line.starts_with("Max address space"))
        .ok_or("Max address space is missing from /proc/self/limits")?
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if fields.len() != 6 || fields[5] != "bytes" {
        return Err("invalid Max address space record".into());
    }
    let soft = fields[3].parse::<usize>().map_err(model::debug)?;
    let hard = fields[4].parse::<usize>().map_err(model::debug)?;
    Ok([soft, hard])
}

fn current_executable_hash() -> Result<String, String> {
    let path = std::env::current_exe().map_err(model::debug)?;
    let mut file = fs::File::open(path).map_err(model::debug)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(model::debug)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn launch_policy() -> LaunchPolicy {
    LaunchPolicy {
        exact_cap_bytes: CAP_BYTES,
        external_address_space_limit_bytes: SCOPED_RLIMIT_AS_BYTES,
        minimum_mem_available_bytes: CAP_BYTES + EXTRA_MEMORY_GATE,
        workers: WORKERS,
        stack_bytes_per_worker: STACK_BYTES,
        hard_timeout_seconds: 2400,
        root_review_required: true,
        residual_localization_must_be_idle: true,
    }
}

fn snapshot_binding(manifest: &Manifest) -> SnapshotBinding<'_> {
    SnapshotBinding {
        identity: &manifest.identity,
        source_commit: &manifest.source_commit,
        plan_sha256: &manifest.plan_sha256,
        coefficient_sha256: &manifest.coefficient_sha256,
        file_sha256: &manifest.file_sha256,
        profile: manifest
            .profile
            .as_ref()
            .map_or("missing", |profile| profile.value.as_str()),
        elapsed: manifest.elapsed,
    }
}

fn no_acceptance() -> Acceptance {
    Acceptance {
        status: "not_assessed",
        accepted_windows: 0,
    }
}

fn region_index(region: SpatialRegion) -> usize {
    match region {
        SpatialRegion::Core => 0,
        SpatialRegion::Annulus => 1,
        SpatialRegion::InteriorOutsideNominal => 2,
        SpatialRegion::Collar => 3,
        SpatialRegion::Exterior => 4,
    }
}

fn region_names() -> [&'static str; 5] {
    [
        "core",
        "annulus",
        "interior_outside_nominal",
        "collar",
        "exterior",
    ]
}

fn region_counts(counts: [usize; 5]) -> [RegionCount; 5] {
    std::array::from_fn(|index| RegionCount {
        region: region_names()[index],
        samples: counts[index],
    })
}

fn coefficient_hash(coefficients: [&[Complex64]; 3]) -> String {
    let mut hash = Sha256::new();
    for component in coefficients {
        for coefficient in component {
            hash.update(coefficient.re.to_bits().to_le_bytes());
            hash.update(coefficient.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
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
mod tests {
    use super::*;

    #[test]
    fn exact_cap_and_work_are_closed() {
        let storage = storage().unwrap();
        assert_eq!(storage.total, CAP_BYTES);
        assert_eq!(storage.packed_reference_cache, DIMENSION.pow(3) * 30 * 8);
        assert_eq!(storage.two_magnitude_arrays, DIMENSION.pow(3) * 2 * 8);
        assert_eq!(storage.cached_region_labels, DIMENSION.pow(3));
    }

    #[test]
    fn ordered_component_schedule_is_complete() {
        let velocity = (0..3)
            .map(|index| Quantity::Velocity.entry(index))
            .collect::<Vec<_>>();
        let gradient = (0..9)
            .map(|index| Quantity::Gradient.entry(index))
            .collect::<Vec<_>>();
        let hessian = (0..27)
            .map(|index| Quantity::OrderedHessian.entry(index))
            .collect::<Vec<_>>();
        assert_eq!(velocity.len(), 3);
        assert_eq!(gradient.len(), 9);
        assert_eq!(hessian.len(), 27);
        let mut canonical_hessian = hessian.clone();
        canonical_hessian.sort_unstable();
        canonical_hessian.dedup();
        assert_eq!(canonical_hessian.len(), 18);
        for component in 0..3 {
            assert_eq!(
                hessian
                    .iter()
                    .filter(|(observed, _)| *observed == component)
                    .count(),
                9
            );
        }
    }

    #[test]
    fn execution_gate_requires_review_and_idle_signal() {
        if std::env::var_os("P10_ROOT_FULL_RUN_REVIEW").is_none()
            && std::env::var_os("P10_RESIDUAL_LOCALIZATION_IDLE").is_none()
        {
            assert!(launch_gate(&"0".repeat(64)).is_err());
        }
    }

    #[test]
    fn address_space_limit_parser_requires_numeric_soft_and_hard_bytes() {
        let limits = parse_address_space_limits(
            "Limit Soft Limit Hard Limit Units\nMax address space 137438953472 137438953472 bytes\n",
        )
        .unwrap();
        assert_eq!(limits, [137_438_953_472; 2]);
        assert!(parse_address_space_limits(
            "Limit Soft Limit Hard Limit Units\nMax address space unlimited unlimited bytes\n",
        )
        .is_err());
    }

    #[test]
    fn cached_labels_partition_every_observation_once() {
        let errors = [1.0, 2.0, 3.0, 4.0, 5.0];
        let references = [5.0, 4.0, 3.0, 2.0, 1.0];
        let labels = [0, 1, 2, 3, 4];
        let (global, regions) = reduce::<3>(&errors, &references, &labels, 1e-8).unwrap();
        assert!(matches!(
            global,
            SampleOutput::Measured { samples: 5, .. }
        ));
        for region in regions {
            assert!(matches!(
                region.sampled,
                SampleOutput::Measured { samples: 1, .. }
            ));
        }
        assert_eq!(regions[3].region, "collar");
    }
}
