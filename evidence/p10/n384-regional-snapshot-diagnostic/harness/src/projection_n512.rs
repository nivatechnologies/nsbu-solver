use crate::model::debug;
use nsbu_benchmarks::{scalar, time::BenchmarkTime, CASE_SHA256};
use nsbu_solver::{
    domain::{Layout, TickClock},
    spectral::{transfer, FftBackend, FftCatalog, FftPlan},
    Complex64,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const ORIGIN: &str = "sampled_analytic_projection";
const REFERENCE_SOURCE_COMMIT: &str = "6bdea3084d737ce6585cb67ab5d48810bd03cd50";
const SAMPLE_DIMENSION: usize = 1024;
const RETAINED_DIMENSION: usize = 512;
const PRODUCER_PEAK_BYTES: usize = 46_246_603_464;
const MEASUREMENT_PEAK_BYTES: usize = 305_085_516_888;
const CONSERVATIVE_CAP_BYTES: usize = MEASUREMENT_PEAK_BYTES;
const FFT_CATALOG_BYTES: usize = 29_362_480;
const FFT_BUDGET_BYTES: usize = 8_606_810_520;
const DERIVATIVE_WORKSPACE_BYTES: usize = 25_803_457_328;
const PROJECTED_COEFFICIENT_BYTES: usize = 3_233_808_384;
const PACKED_REFERENCE_CACHE_BYTES: usize = 257_698_037_760;
const MAGNITUDE_ARRAY_BYTES: usize = 17_179_869_184;
const REGION_LABEL_BYTES: usize = 1_073_741_824;
const WORKER_STACK_BYTES: usize = 67_108_864;
const MEASUREMENT_ALLOCATOR_ALLOWANCE: usize = 65_528;
const ADDRESS_SPACE_LIMIT_BYTES: usize = 343_597_383_680;
const MINIMUM_MEM_AVAILABLE_BYTES: usize = 322_265_386_072;
const WORKERS: usize = 32;
const STACK_BYTES: usize = 2 * 1024 * 1024;
const PRODUCER_ALLOWANCE: usize = 40_960;
const SERIALIZATION_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectionClock {
    Early512,
    Endpoint4096,
}

impl ProjectionClock {
    fn elapsed(self) -> u128 {
        match self {
            Self::Early512 => 512,
            Self::Endpoint4096 => 4096,
        }
    }

    fn exact(self) -> Result<TickClock, String> {
        let elapsed = self.elapsed();
        let remaining = 8192_u128
            .checked_sub(elapsed)
            .ok_or("projection clock exceeds exact target")?;
        TickClock::restore(-20, 8192, elapsed, remaining).map_err(debug)
    }
}

#[derive(Clone, Copy, Serialize)]
struct SourceIdentity {
    role: &'static str,
    sha256: &'static str,
}

#[derive(Clone, Copy, Serialize)]
struct ResourceLedger {
    projected_coefficients: usize,
    fft_catalog: usize,
    derivative_workspace_required: usize,
    derivative_workspace_budget: usize,
    derivative_workspace_allowance: usize,
    packed_reference_cache: usize,
    two_magnitude_arrays: usize,
    cached_region_labels: usize,
    worker_stacks: usize,
    allocator_allowance: usize,
    serialization: usize,
    measurement_total: usize,
}

const SOURCES: [SourceIdentity; 4] = [
    SourceIdentity {
        role: "scalar-evaluator",
        sha256: "2bd8e841dc5b2b1bd4a301ab8bfc392b81716ce58f5c019b8a35b4c9ff599bb8",
    },
    SourceIdentity {
        role: "scalar-root",
        sha256: "8b41613dfad4f0c09c2bdb28d9e28d499cebd9e55b2210c3f6cc400730356c3b",
    },
    SourceIdentity {
        role: "benchmark-time",
        sha256: "c5301d2ab63ea3cec9e331bec3c3e41d0c2c0a4e613807a062e516046a74fd06",
    },
    SourceIdentity {
        role: "case-definition",
        sha256: CASE_SHA256,
    },
];
#[derive(Serialize)]
pub(crate) struct ProjectionRecord {
    schema: &'static str,
    status: &'static str,
    origin: &'static str,
    reference_source_commit: &'static str,
    reference_sources: [SourceIdentity; 4],
    case_sha256: &'static str,
    clock: [i128; 3],
    sample_dimensions: [usize; 3],
    retained_dimensions: [usize; 3],
    physical_grid: &'static str,
    fft_normalization: &'static str,
    crop: &'static str,
    producer_peak_bytes: usize,
    producer_allocator_allowance_bytes: usize,
    serialization_bytes: usize,
    measurement_peak_bytes: usize,
    conservative_cap_bytes: usize,
    external_address_space_limit_bytes: usize,
    minimum_mem_available_bytes: usize,
    hard_timeout_seconds: usize,
    measurement_storage: ResourceLedger,
    producer_scratch_dropped_before_measurement: bool,
    diagnostic_binary_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    projected_coefficient_sha256_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    projected_coefficient_hash_equal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measurements: Option<crate::diagnostic::Measurements>,
    state_inputs: usize,
    trajectory_from_rest_claims: usize,
    state_import_or_resume_interfaces: usize,
    acceptance: ProjectionAcceptance,
}

#[derive(Serialize)]
struct ProjectionAcceptance {
    status: &'static str,
    accepted_windows: usize,
}

pub(crate) fn preflight(clock: ProjectionClock) -> Result<ProjectionRecord, String> {
    validate_sources()?;
    validate_resources()?;
    clock.exact()?;
    record(clock, "bound_preflight_only_no_state_input", None, None)
}

pub(crate) fn execute(clock: ProjectionClock, cap: usize, output: &PathBuf) -> Result<(), String> {
    if cap != CONSERVATIVE_CAP_BYTES {
        return Err("projection CAP_BYTES differs from reviewed conservative cap".into());
    }
    validate_sources()?;
    validate_resources()?;
    let binary = crate::diagnostic::executable_hash()?;
    crate::diagnostic::authorize_projection_run(
        &binary,
        ADDRESS_SPACE_LIMIT_BYTES,
        MINIMUM_MEM_AVAILABLE_BYTES,
    )?;
    let coefficients = produce_projection(clock)?;
    let borrowed = coefficients.each_ref().map(Vec::as_slice);
    let exact = clock.exact()?;
    let clock_header = crate::model::ClockHeader {
        elapsed: exact.elapsed(),
        target: exact.target(),
        epoch: 0,
        accepted_steps: 0,
    };
    let before = crate::diagnostic::hash_coefficients(borrowed);
    let measurements = crate::diagnostic::measure_coefficients_at(
        borrowed,
        clock_header,
        RETAINED_DIMENSION,
        SAMPLE_DIMENSION,
        DERIVATIVE_WORKSPACE_BYTES,
    )?;
    if measurements.coefficient_sha256 != before {
        return Err("projected coefficients changed during measurement".into());
    }
    crate::diagnostic::write_transactional(
        output,
        &record(
            clock,
            "sampled_analytic_projection_diagnostic_complete",
            Some(measurements),
            Some(before),
        )?,
    )
}

fn record(
    clock: ProjectionClock,
    status: &'static str,
    measurements: Option<crate::diagnostic::Measurements>,
    before: Option<String>,
) -> Result<ProjectionRecord, String> {
    let equal = before
        .as_ref()
        .zip(measurements.as_ref())
        .map(|(before, after)| before == &after.coefficient_sha256);
    Ok(ProjectionRecord {
        schema: "p10-n512-m1024-analytic-projection-regional-diagnostic-v1",
        status,
        origin: ORIGIN,
        reference_source_commit: REFERENCE_SOURCE_COMMIT,
        reference_sources: SOURCES,
        case_sha256: CASE_SHA256,
        clock: [-20, 8192, clock.elapsed() as i128],
        sample_dimensions: [SAMPLE_DIMENSION; 3],
        retained_dimensions: [RETAINED_DIMENSION; 3],
        physical_grid: "unshifted-periodic-x_i=i/n",
        fft_normalization: "forward-r2c-divide-by-complete-real-sample-count",
        crop: "normalization-preserving-strict-band-transfer-no-rescale",
        producer_peak_bytes: PRODUCER_PEAK_BYTES,
        producer_allocator_allowance_bytes: PRODUCER_ALLOWANCE,
        serialization_bytes: SERIALIZATION_BYTES,
        measurement_peak_bytes: MEASUREMENT_PEAK_BYTES,
        conservative_cap_bytes: CONSERVATIVE_CAP_BYTES,
        external_address_space_limit_bytes: ADDRESS_SPACE_LIMIT_BYTES,
        minimum_mem_available_bytes: MINIMUM_MEM_AVAILABLE_BYTES,
        hard_timeout_seconds: 6000,
        measurement_storage: resource_ledger()?,
        producer_scratch_dropped_before_measurement: true,
        diagnostic_binary_sha256: crate::diagnostic::executable_hash()?,
        projected_coefficient_sha256_before: before,
        projected_coefficient_hash_equal: equal,
        measurements,
        state_inputs: 0,
        trajectory_from_rest_claims: 0,
        state_import_or_resume_interfaces: 0,
        acceptance: ProjectionAcceptance {
            status: "not_assessed",
            accepted_windows: 0,
        },
    })
}

fn produce_projection(clock: ProjectionClock) -> Result<[Vec<Complex64>; 3], String> {
    let samples = Layout::new([SAMPLE_DIMENSION; 3]).map_err(debug)?;
    let retained = Layout::new([RETAINED_DIMENSION; 3]).map_err(debug)?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog = FftCatalog::new(backend, FFT_CATALOG_BYTES).map_err(debug)?;
    project_velocity(
        samples,
        retained,
        &catalog,
        FFT_BUDGET_BYTES,
        sample_velocity(samples, clock)?,
    )
}

fn sample_velocity(layout: Layout, clock: ProjectionClock) -> Result<[Vec<f64>; 3], String> {
    let mut physical = [
        zeros(layout.real_len())?,
        zeros(layout.real_len())?,
        zeros(layout.real_len())?,
    ];
    let time = BenchmarkTime::new(clock.exact()?).map_err(debug)?;
    let plane = SAMPLE_DIMENSION * SAMPLE_DIMENSION;
    let [x_all, y_all, z_all] = &mut physical;
    std::thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::new();
        for (worker, ((x, y), z)) in x_all
            .chunks_mut(32 * plane)
            .zip(y_all.chunks_mut(32 * plane))
            .zip(z_all.chunks_mut(32 * plane))
            .enumerate()
        {
            handles.push(
                std::thread::Builder::new()
                    .stack_size(STACK_BYTES)
                    .spawn_scoped(scope, move || {
                        for local in 0..x.len() {
                            let index = worker * 32 * plane + local;
                            let point = [
                                (index / plane) as f64 / SAMPLE_DIMENSION as f64,
                                ((index / SAMPLE_DIMENSION) % SAMPLE_DIMENSION) as f64
                                    / SAMPLE_DIMENSION as f64,
                                (index % SAMPLE_DIMENSION) as f64 / SAMPLE_DIMENSION as f64,
                            ];
                            let value = scalar::evaluate(point, time).map_err(debug)?.velocity;
                            x[local] = value[0];
                            y[local] = value[1];
                            z[local] = value[2];
                        }
                        Ok::<_, String>(())
                    })
                    .map_err(debug)?,
            );
        }
        if handles.len() != WORKERS {
            return Err("projection worker partition mismatch".into());
        }
        for handle in handles {
            handle.join().map_err(|_| "projection worker panicked")??;
        }
        Ok(())
    })?;
    Ok(physical)
}

fn validate_sources() -> Result<(), String> {
    let bytes: [&[u8]; 4] = [
        include_bytes!("../../../../../crates/nsbu-benchmarks/src/scalar.rs"),
        include_bytes!("../../../../../crates/nsbu-benchmarks/src/root.rs"),
        include_bytes!("../../../../../crates/nsbu-benchmarks/src/time.rs"),
        include_bytes!("../../../../../crates/nsbu-benchmarks/data/similarity-mms-v2.json"),
    ];
    bytes
        .into_iter()
        .zip(SOURCES)
        .all(|(bytes, source)| format!("{:x}", Sha256::digest(bytes)) == source.sha256)
        .then_some(())
        .ok_or_else(|| "analytical projection source hash mismatch".into())
}

fn resource_ledger() -> Result<ResourceLedger, String> {
    let samples = Layout::new([SAMPLE_DIMENSION; 3]).map_err(debug)?;
    let retained = Layout::new([RETAINED_DIMENSION; 3]).map_err(debug)?;
    let domain =
        nsbu_solver::domain::Domain::new([RETAINED_DIMENSION; 3], [1.0; 3], 1.0).map_err(debug)?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let derivative_required =
        nsbu_solver::diagnostics::derivatives::DerivativeWorkspace::reservation_with_shared_backend(
            domain, samples, backend,
        )
        .map_err(debug)?;
    let producer = samples.real_len() * 3 * 8
        + FFT_CATALOG_BYTES
        + FFT_BUDGET_BYTES
        + samples.half_len() * 16
        + retained.half_len() * 3 * 16
        + PRODUCER_ALLOWANCE
        + SERIALIZATION_BYTES;
    let measurement = retained.half_len() * 3 * 16
        + FFT_CATALOG_BYTES
        + DERIVATIVE_WORKSPACE_BYTES
        + PACKED_REFERENCE_CACHE_BYTES
        + MAGNITUDE_ARRAY_BYTES
        + REGION_LABEL_BYTES
        + WORKER_STACK_BYTES
        + MEASUREMENT_ALLOCATOR_ALLOWANCE
        + SERIALIZATION_BYTES;
    let derivative_allowance = DERIVATIVE_WORKSPACE_BYTES
        .checked_sub(derivative_required)
        .ok_or("N512/M1024 derivative reservation exceeds reviewed budget")?;
    if retained.half_len() * 3 * 16 != PROJECTED_COEFFICIENT_BYTES
        || producer != PRODUCER_PEAK_BYTES
        || measurement != MEASUREMENT_PEAK_BYTES
    {
        return Err("N512/M1024 resource reservation differs from reviewed ledger".into());
    }
    Ok(ResourceLedger {
        projected_coefficients: PROJECTED_COEFFICIENT_BYTES,
        fft_catalog: FFT_CATALOG_BYTES,
        derivative_workspace_required: derivative_required,
        derivative_workspace_budget: DERIVATIVE_WORKSPACE_BYTES,
        derivative_workspace_allowance: derivative_allowance,
        packed_reference_cache: PACKED_REFERENCE_CACHE_BYTES,
        two_magnitude_arrays: MAGNITUDE_ARRAY_BYTES,
        cached_region_labels: REGION_LABEL_BYTES,
        worker_stacks: WORKER_STACK_BYTES,
        allocator_allowance: MEASUREMENT_ALLOCATOR_ALLOWANCE,
        serialization: SERIALIZATION_BYTES,
        measurement_total: measurement,
    })
}

fn validate_resources() -> Result<(), String> {
    resource_ledger().map(|_| ())
}

pub(crate) fn project_velocity(
    samples: Layout,
    retained: Layout,
    catalog: &FftCatalog,
    fft_budget: usize,
    physical: [Vec<f64>; 3],
) -> Result<[Vec<Complex64>; 3], String> {
    if physical
        .iter()
        .any(|values| values.len() != samples.real_len())
    {
        return Err("analytical projection physical shape mismatch".into());
    }
    let (plan, mut workspace) =
        FftPlan::new_from_catalog(samples, catalog, fft_budget).map_err(debug)?;
    let mut spectrum = zeros(samples.half_len())?;
    let mut projected = [
        zeros(retained.half_len())?,
        zeros(retained.half_len())?,
        zeros(retained.half_len())?,
    ];
    for component in 0..3 {
        plan.forward(&physical[component], &mut spectrum, &mut workspace)
            .map_err(debug)?;
        transfer(samples, retained, &spectrum, &mut projected[component]).map_err(debug)?;
    }
    Ok(projected)
}

fn zeros<T: Clone + Default>(len: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| "analytical projection allocation failed")?;
    values.resize(len, T::default());
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_solver::spectral::FftBackend;

    #[test]
    fn two_phase_storage_is_closed_and_has_no_state_input() {
        let samples = Layout::new([SAMPLE_DIMENSION; 3]).unwrap();
        let retained = Layout::new([RETAINED_DIMENSION; 3]).unwrap();
        let projection_subphase = samples.real_len() * 3 * 8
            + FFT_CATALOG_BYTES
            + FFT_BUDGET_BYTES
            + samples.half_len() * 16
            + retained.half_len() * 3 * 16
            + PRODUCER_ALLOWANCE
            + SERIALIZATION_BYTES;
        assert_eq!(projection_subphase, PRODUCER_PEAK_BYTES);
        assert_eq!(MEASUREMENT_PEAK_BYTES, CONSERVATIVE_CAP_BYTES);
        validate_resources().unwrap();
        let record = preflight(ProjectionClock::Early512).unwrap();
        assert_eq!(
            (record.state_inputs, record.trajectory_from_rest_claims),
            (0, 0)
        );
    }

    #[test]
    fn projection_clocks_are_closed_and_restore_exact_remainders() {
        for (clock, elapsed, remaining) in [
            (ProjectionClock::Early512, 512, 7680),
            (ProjectionClock::Endpoint4096, 4096, 4096),
        ] {
            let exact = clock.exact().unwrap();
            assert_eq!((exact.elapsed(), exact.remaining()), (elapsed, remaining));
            let record = preflight(clock).unwrap();
            assert_eq!(record.clock, [-20, 8192, elapsed as i128]);
            assert_eq!(record.state_inputs, 0);
            assert_eq!(record.trajectory_from_rest_claims, 0);
        }
    }

    #[test]
    fn retains_x_plus_2y_and_removes_x_plus_4y() {
        let samples = Layout::new([12; 3]).unwrap();
        let retained = Layout::new([6; 3]).unwrap();
        let tau = 2.0 * std::f64::consts::PI;
        let physical = std::array::from_fn(|component| {
            (0..samples.real_len())
                .map(|index| {
                    let i = index / (12 * 12);
                    let j = (index / 12) % 12;
                    let [x, y] = [i as f64 / 12.0, j as f64 / 12.0];
                    (component == 0) as u8 as f64
                        * ((tau * (x + 2.0 * y)).cos() + 0.25 * (tau * (x + 4.0 * y)).cos())
                })
                .collect()
        });
        let backend = FftBackend::OwnedRadix;
        let catalog = FftCatalog::new(backend, FftCatalog::reservation(backend).unwrap()).unwrap();
        let budget = FftPlan::reservation_from_catalog(samples, &catalog).unwrap();
        let projected = project_velocity(samples, retained, &catalog, budget, physical).unwrap();
        let expected = [
            retained.locate([1, 2, 0]).unwrap().0,
            retained.locate([-1, -2, 0]).unwrap().0,
        ];
        for (index, value) in projected[0].iter().enumerate() {
            let target = if expected.contains(&index) { 0.5 } else { 0.0 };
            assert!((value.re - target).abs() < 1e-12 && value.im.abs() < 1e-12);
        }
        assert!(projected[1..]
            .iter()
            .flatten()
            .all(|value| *value == Complex64::default()));
    }
}
