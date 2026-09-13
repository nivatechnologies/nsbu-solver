use super::{
    model, Acceptance, LaunchPolicy, RegionCount, ReviewedSnapshot, SnapshotBinding, Storage, Work,
    ALLOCATOR_ALLOWANCE, CAP_BYTES, DECODED_BYTES, DECODER_OVERHEAD, DERIVATIVE_WORKSPACE_BUDGET,
    DIMENSION, EXTRA_MEMORY_GATE, FFT_CATALOG_BYTES, LABEL_BYTES, MAGNITUDE_BYTES, OUTPUT_CAP,
    PACKED_CACHE_BYTES, REVIEWED_SNAPSHOTS, ROOT_BUDGET, SCOPED_RLIMIT_AS_BYTES, STACK_BYTES,
    WORKERS, WORKER_STACK_BYTES,
};
use crate::{
    cache::PackedReference,
    model::{ComparisonKind, Manifest, ProfileBindingKind},
};
use nsbu_benchmarks::{regions::SpatialRegion, CASE_SHA256};
use nsbu_solver::{
    diagnostics::derivatives::DerivativeWorkspace,
    domain::{Domain, Layout},
    spectral::FftBackend,
    Complex64,
};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

pub(super) fn validate_snapshot(manifest: &Manifest) -> Result<&'static ReviewedSnapshot, String> {
    REVIEWED_SNAPSHOTS
        .iter()
        .find(|binding| snapshot_matches(manifest, binding))
        .ok_or_else(|| "snapshot is not a reviewed N384 clock-512 regional baseline".into())
}

fn snapshot_matches(manifest: &Manifest, binding: &ReviewedSnapshot) -> bool {
    let profile = manifest.profile.as_ref();
    let exact = [
        manifest.comparison_kind == ComparisonKind::MatchedSpatial,
        manifest.source_commit == binding.source_commit,
        manifest.plan_sha256 == binding.plan_sha256,
        manifest.coefficient_sha256 == binding.coefficient_sha256,
        manifest.file_sha256 == binding.file_sha256,
        manifest.dimensions == [384; 3],
        manifest.evolution.case_sha256 == CASE_SHA256,
        manifest.evolution.quantum_exponent == -20,
        manifest.evolution.clock_target == 8192,
        manifest.evolution.comparison_endpoint == 512,
        manifest.evolution.lengths == [1.0; 3],
        manifest.evolution.viscosity == 1.0,
        manifest.evolution.method == "cox-matthews",
        manifest.evolution.integration_force_dimensions == [binding.integration_force_dimension; 3],
        manifest.elapsed == 512,
        manifest.target == 8192,
        manifest.epoch == 8,
        manifest.accepted_steps == 8,
        profile.is_some_and(|value| {
            value.kind == ProfileBindingKind::IdentityProfileField && value.value == binding.profile
        }),
        manifest.evolution.schedule.len() == 1,
        manifest.evolution.schedule.first().is_some_and(|segment| {
            segment.from_inclusive == 0
                && segment.until_exclusive == 512
                && segment.step_ticks == 64
        }),
        identity_value(&manifest.identity, "source") == Some(binding.source_commit),
        identity_value(&manifest.identity, "case") == Some(CASE_SHA256),
        identity_value(&manifest.identity, "profile") == Some(binding.profile),
        identity_value(&manifest.identity, "retained") == Some("384"),
        identity_value(&manifest.identity, "force_samples")
            == Some(if binding.integration_force_dimension == 384 {
                "384"
            } else {
                "512"
            }),
        identity_value(&manifest.identity, "observer_conservative") == Some("768"),
        manifest.backend == "rustfft-6.4.1-avx-avx2-fma",
    ];
    exact.into_iter().all(std::convert::identity)
}

pub(super) fn verify_manifest_hash(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(model::debug)?;
    if bytes.len() > OUTPUT_CAP {
        return Err("snapshot manifest exceeds 64 KiB".into());
    }
    let observed = format!("{:x}", Sha256::digest(bytes));
    if !REVIEWED_SNAPSHOTS
        .iter()
        .any(|binding| observed == binding.manifest_sha256)
    {
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

pub(super) fn storage() -> Result<Storage, String> {
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

pub(super) fn work(manifest: &Manifest) -> Result<Work, String> {
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

pub(super) fn launch_gate(binary_sha256: &str) -> Result<(), String> {
    launch_gate_with(
        binary_sha256,
        SCOPED_RLIMIT_AS_BYTES,
        CAP_BYTES + EXTRA_MEMORY_GATE,
    )
}

pub(super) fn launch_gate_with(
    binary_sha256: &str,
    expected_address_space_limit: usize,
    minimum_mem_available: usize,
) -> Result<(), String> {
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
    if limits != [expected_address_space_limit; 2] {
        return Err(format!(
            "full run requires actual RLIMIT_AS soft/hard limits of {expected_address_space_limit}, observed {limits:?}"
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
    if available_memory < minimum_mem_available {
        return Err(format!(
            "MemAvailable {available_memory} is below required {minimum_mem_available}"
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

pub(super) fn parse_address_space_limits(text: &str) -> Result<[usize; 2], String> {
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

pub(super) fn current_executable_hash() -> Result<String, String> {
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

pub(super) fn launch_policy() -> LaunchPolicy {
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

pub(super) fn snapshot_binding(manifest: &Manifest) -> SnapshotBinding<'_> {
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

pub(super) fn no_acceptance() -> Acceptance {
    Acceptance {
        status: "not_assessed",
        accepted_windows: 0,
    }
}

pub(super) fn region_index(region: SpatialRegion) -> usize {
    match region {
        SpatialRegion::Core => 0,
        SpatialRegion::Annulus => 1,
        SpatialRegion::InteriorOutsideNominal => 2,
        SpatialRegion::Collar => 3,
        SpatialRegion::Exterior => 4,
    }
}

pub(super) fn region_names() -> [&'static str; 5] {
    [
        "core",
        "annulus",
        "interior_outside_nominal",
        "collar",
        "exterior",
    ]
}

pub(super) fn region_counts(counts: [usize; 5]) -> [RegionCount; 5] {
    std::array::from_fn(|index| RegionCount {
        region: region_names()[index],
        samples: counts[index],
    })
}

pub(super) fn coefficient_hash(coefficients: [&[Complex64]; 3]) -> String {
    let mut hash = Sha256::new();
    for component in coefficients {
        for coefficient in component {
            hash.update(coefficient.re.to_bits().to_le_bytes());
            hash.update(coefficient.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}
