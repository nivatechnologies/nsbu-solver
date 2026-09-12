//! Imported-gauge identity, global convention and actual all-six pressure tracking.
use nsbu_benchmarks::{
    fields::reference,
    runtime_force::ForceSettings,
    time::BenchmarkTime,
    v2_experiment::{
        diagnostic::StartupProfile,
        pressure_reference::{
            GaugeError, ImportedGauge, PressureReferencePlan, PressureReferenceSample,
            PressureReferenceWorkspace,
        },
        FamilyError, FamilyPlan, FamilySettings, V2Family,
    },
};
use nsbu_solver::{
    domain::{Layout, SpectralState, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
use sha2::{Digest, Sha256};

const CAP: usize = 256 * 1024 * 1024;
const ENDPOINT_CAP: usize = 1024 * 1024 * 1024;
const CLOCK0: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock0.json");
const CLOCK64: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock64.json");
const CLOCK128: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock128.json");
const CLOCK2048: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock2048.json");
const CLOCK4096: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock4096.json");

fn gauges() -> [ImportedGauge<'static>; 3] {
    [CLOCK0, CLOCK64, CLOCK128].map(|bytes| ImportedGauge::load(bytes, bytes.len()).unwrap())
}

fn endpoint_gauges() -> [ImportedGauge<'static>; 3] {
    [CLOCK0, CLOCK2048, CLOCK4096].map(|bytes| ImportedGauge::load(bytes, bytes.len()).unwrap())
}

fn endpoint_clocks() -> [TickClock; 3] {
    [0, 2048, 4096].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}

fn endpoint_settings() -> FamilySettings {
    FamilySettings {
        grids: [12, 16, 24],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([24; 3]).unwrap(),
            workers: 12,
        },
        endpoint: 4096,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}

fn tiny_endpoint_settings() -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [128, 64, 32],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 4096,
        tolerances: Tolerances {
            absolute: [1e3; 2],
            relative: [0.0; 2],
        },
        advective_limit: 1e3,
    }
}

fn state_hash(state: &SpectralState) -> [u8; 32] {
    let mut digest = Sha256::new();
    for component in 0..3 {
        for value in state.component(component).unwrap() {
            digest.update(value.re.to_bits().to_le_bytes());
            digest.update(value.im.to_bits().to_le_bytes());
        }
    }
    digest.finalize().into()
}

fn family_hashes(family: &V2Family<'_>) -> [[u8; 32]; 6] {
    std::array::from_fn(|branch| state_hash(family.branch(branch).unwrap().state()))
}

fn assert_artifact_projection(gauge: ImportedGauge<'_>) {
    let decoded: serde_json::Value = serde_json::from_slice(gauge.artifact_bytes()).unwrap();
    let rows = decoded["estimates"].as_array().unwrap();
    assert_eq!(gauge.estimates().len(), 10);
    assert_eq!(gauge.estimates()[0].precision, 80);
    assert_eq!(gauge.estimates()[5].precision, 120);
    let finest = 4 * gauge.estimates()[0].axial_panels;
    assert_eq!(
        (
            gauge.estimates()[7].axial_panels,
            gauge.estimates()[7].radial_panels
        ),
        (finest, finest)
    );
    assert_eq!(gauge.changes().precision_differences.len(), 5);
    for ((typed, binary), raw) in gauge
        .estimates()
        .iter()
        .zip(gauge.binary64_means())
        .zip(rows)
    {
        assert_eq!(
            typed.axial_panels,
            raw["axial_panels"].as_u64().unwrap() as usize
        );
        assert_eq!(
            typed.radial_panels,
            raw["radial_squared_panels"].as_u64().unwrap() as usize
        );
        assert_eq!(typed.precision, raw["precision"].as_u64().unwrap() as usize);
        assert_eq!(typed.raw_mean, raw["mean"].as_str().unwrap());
        assert_eq!(
            binary.to_bits(),
            typed.raw_mean.parse::<f64>().unwrap().to_bits()
        );
    }
    assert_eq!(
        gauge.changes().finest_mean,
        decoded["changes"]["finest_mean"].as_str().unwrap()
    );
    assert_eq!(gauge.estimates()[7].raw_mean, gauge.changes().finest_mean);
    let changes = gauge.changes();
    for (typed, key) in [
        (changes.coarse_to_middle, "coarse_to_middle"),
        (changes.middle_to_fine, "middle_to_fine"),
        (changes.axial_only_to_fine, "axial_only_to_fine"),
        (changes.radial_only_to_fine, "radial_only_to_fine"),
    ] {
        assert_eq!(typed, decoded["changes"][key].as_str().unwrap());
    }
    for (typed, raw) in changes.precision_differences.iter().zip(
        decoded["changes"]["precision_differences"]
            .as_array()
            .unwrap(),
    ) {
        assert_eq!(*typed, raw.as_str().unwrap());
    }
}

#[test]
fn exact_artifact_bytes_retain_all_raw_estimates_and_separate_changes() {
    let imported = gauges();
    assert_eq!(imported.map(|gauge| gauge.clock().elapsed()), [0, 64, 128]);
    for gauge in imported {
        assert_artifact_projection(gauge);
    }
    assert_eq!(imported[0].mean().to_bits(), 0.0f64.to_bits());
    assert!(imported[1].mean() < 0.0);
    assert!(imported[2].mean() < imported[1].mean());
    let endpoint = endpoint_gauges();
    assert_eq!(
        endpoint.map(|gauge| gauge.clock().elapsed()),
        [0, 2048, 4096]
    );
    for gauge in endpoint {
        assert_artifact_projection(gauge);
        if gauge.clock().elapsed() > 0 {
            assert_eq!(gauge.estimates()[0].axial_panels, 32);
            assert_eq!(gauge.estimates()[7].radial_panels, 128);
        }
    }

    assert_eq!(
        ImportedGauge::load(CLOCK64, CLOCK64.len() - 1).unwrap_err(),
        GaugeError::CapacityExceeded
    );
    let mut changed = CLOCK64.to_vec();
    let last = changed.len() - 2;
    changed[last] ^= 1;
    assert_eq!(
        ImportedGauge::load(&changed, changed.len()).unwrap_err(),
        GaugeError::InvalidArtifact
    );
    let mut changed_endpoint = CLOCK2048.to_vec();
    let middle = changed_endpoint.len() / 2;
    changed_endpoint[middle] ^= 1;
    assert_eq!(
        ImportedGauge::load(&changed_endpoint, changed_endpoint.len()).unwrap_err(),
        GaugeError::InvalidArtifact
    );
}

#[test]
fn gauge_is_one_global_constant_and_gradient_is_invariant() {
    let clock = TickClock::restore(-20, 8192, 128, 8064).unwrap();
    let time = BenchmarkTime::new(clock).unwrap();
    let a = reference::evaluate([0.5, 0.5, 0.5], time).unwrap();
    let b = reference::evaluate([0.49, 0.49, 0.49], time).unwrap();
    let mean = gauges()[2].mean();
    assert_eq!(a.pressure_raw.to_bits(), 0.0f64.to_bits());
    assert_eq!((a.pressure_raw - mean).to_bits(), (-mean).to_bits());
    assert_eq!(
        (a.pressure_raw - mean) - (b.pressure_raw - mean),
        a.pressure_raw - b.pressure_raw
    );
    assert_eq!(
        a.pressure_gradient,
        reference::evaluate([0.5, 0.5, 0.5], time)
            .unwrap()
            .pressure_gradient
    );
    assert_ne!(mean.to_bits(), 0.0f64.to_bits());
}

fn assert_actual_sample(
    sample: PressureReferenceSample<'_>,
    expected: u128,
    family_identity: [u8; 32],
    sample_grid: usize,
    force_grid: usize,
) {
    assert_eq!(sample.clock().elapsed(), expected);
    assert_eq!(sample.family_identity(), family_identity);
    assert_eq!(sample.sample_layout().dimensions(), [sample_grid; 3]);
    assert_eq!(sample.force_layout().dimensions(), [force_grid; 3]);
    assert_eq!(sample.gauge().clock(), sample.clock());
    eprintln!(
        "clock={expected} branch2_pressure_rms={} branch2_gradient_rms={}",
        sample.branches()[2].pressure.rms_error,
        sample.branches()[2].pressure_gradient.rms_error
    );
    for (index, branch) in sample.branches().iter().enumerate() {
        assert_eq!(branch.branch, index);
        for error in [branch.pressure, branch.pressure_gradient] {
            assert!(error.rms_error.is_finite());
            assert!(error.peak_error.is_finite());
            assert!(error.peak_relative_error.is_finite());
        }
    }
    if expected >= 64 {
        assert!(sample.branches()[2].pressure.rms_error > 1e-12);
        assert!(sample.branches()[2].pressure_gradient.rms_error > 1e-10);
    }
}

#[test]
fn actual_accepted_states_publish_all_six_only_after_complete_measurement() {
    let startup = StartupProfile::new().unwrap();
    let diagnostic = startup.plan(CAP).unwrap();
    let family_plan = diagnostic.family_plan();
    let plan = PressureReferencePlan::new(
        family_plan,
        gauges(),
        Layout::new([24; 3]).unwrap(),
        [1e-8, 1e-7],
        CAP,
    )
    .unwrap();
    assert_eq!(plan.force_layout().dimensions(), [24; 3]);
    assert_eq!(
        plan.gauges().map(|gauge| gauge.clock().elapsed()),
        [0, 64, 128]
    );
    let mut family = V2Family::new(family_plan).unwrap();
    let mut workspace = PressureReferenceWorkspace::new(plan).unwrap();
    for expected in [0, 64, 128] {
        family.advance().unwrap();
        let sample = workspace.measure(&family).unwrap();
        assert_actual_sample(sample, expected, family_plan.identity(), 24, 24);
    }
    assert_eq!(workspace.remaining(), 0);
    assert!(matches!(
        workspace.measure(&family),
        Err(FamilyError::Numerical(_))
    ));
}

#[test]
fn reviewed_endpoint_schedule_admits_and_measures_only_the_rest_state() {
    let clocks = endpoint_clocks();
    let family = FamilyPlan::new(
        endpoint_settings(),
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        ENDPOINT_CAP,
    )
    .unwrap();
    let plan = PressureReferencePlan::new(
        family,
        endpoint_gauges(),
        Layout::new([48; 3]).unwrap(),
        [1e-8, 1e-7],
        ENDPOINT_CAP,
    )
    .unwrap();
    assert_eq!(
        plan.gauges().map(|gauge| gauge.clock().elapsed()),
        [0, 2048, 4096]
    );
    assert_eq!(plan.sample_layout().dimensions(), [48; 3]);
    assert_eq!(plan.force_layout().dimensions(), [48; 3]);

    let mut states = V2Family::new(family).unwrap();
    let mut reference = PressureReferenceWorkspace::new(plan).unwrap();
    states.advance().unwrap();
    let sample = reference.measure(&states).unwrap();
    assert_actual_sample(sample, 0, family.identity(), 48, 48);

    let mut reordered = endpoint_gauges();
    reordered.swap(1, 2);
    assert!(PressureReferencePlan::new(
        family,
        reordered,
        Layout::new([48; 3]).unwrap(),
        [1e-8, 1e-7],
        ENDPOINT_CAP,
    )
    .is_err());
    assert!(PressureReferencePlan::new(
        family,
        gauges(),
        Layout::new([48; 3]).unwrap(),
        [1e-8, 1e-7],
        ENDPOINT_CAP,
    )
    .is_err());
}

#[test]
fn midpoint_gauge_measures_tiny_nonzero_states_without_mutation() {
    let clocks = endpoint_clocks();
    let family = FamilyPlan::new(
        tiny_endpoint_settings(),
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        ENDPOINT_CAP,
    )
    .unwrap();
    let plan = PressureReferencePlan::new(
        family,
        endpoint_gauges(),
        Layout::new([24; 3]).unwrap(),
        [1e-8, 1e-7],
        ENDPOINT_CAP,
    )
    .unwrap();
    let mut states = V2Family::new(family).unwrap();
    let mut reference = PressureReferenceWorkspace::new(plan).unwrap();
    for expected in [0, 2048] {
        states.advance().unwrap();
        let before = family_hashes(&states);
        let sample = reference.measure(&states).unwrap();
        assert_actual_sample(sample, expected, family.identity(), 24, 24);
        assert_eq!(sample.gauge().clock().elapsed(), expected);
        assert_eq!(before, family_hashes(&states));
    }
}

#[test]
fn reordered_gauges_and_short_joint_cap_are_refused() {
    let startup = StartupProfile::new().unwrap();
    let diagnostic = startup.plan(CAP).unwrap();
    let family = diagnostic.family_plan();
    let samples = Layout::new([24; 3]).unwrap();
    let mut reordered = gauges();
    reordered.swap(1, 2);
    assert!(PressureReferencePlan::new(family, reordered, samples, [1e-8, 1e-7], CAP).is_err());
    let admitted =
        PressureReferencePlan::new(family, gauges(), samples, [1e-8, 1e-7], CAP).unwrap();
    assert!(PressureReferencePlan::new(
        family,
        gauges(),
        samples,
        [1e-8, 1e-7],
        admitted.bounds().joint_storage_bytes - 1
    )
    .is_err());
}
