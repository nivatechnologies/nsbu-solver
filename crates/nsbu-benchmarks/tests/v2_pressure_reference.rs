//! Imported-gauge identity, global convention and actual all-six pressure tracking.
use nsbu_benchmarks::{
    fields::reference,
    time::BenchmarkTime,
    v2_experiment::{
        diagnostic::StartupProfile,
        pressure_reference::{
            GaugeError, ImportedGauge, PressureReferencePlan, PressureReferenceWorkspace,
        },
        FamilyError, V2Family,
    },
};
use nsbu_solver::domain::{Layout, TickClock};

const CAP: usize = 256 * 1024 * 1024;
const CLOCK0: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock0.json");
const CLOCK64: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock64.json");
const CLOCK128: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock128.json");

fn gauges() -> [ImportedGauge<'static>; 3] {
    [CLOCK0, CLOCK64, CLOCK128].map(|bytes| ImportedGauge::load(bytes, bytes.len()).unwrap())
}

#[test]
fn exact_artifact_bytes_retain_all_raw_estimates_and_separate_changes() {
    let imported = gauges();
    assert_eq!(imported.map(|gauge| gauge.clock().elapsed()), [0, 64, 128]);
    for gauge in imported {
        let decoded: serde_json::Value = serde_json::from_slice(gauge.artifact_bytes()).unwrap();
        let rows = decoded["estimates"].as_array().unwrap();
        assert_eq!(gauge.estimates().len(), 10);
        assert_eq!(gauge.estimates()[0].precision, 80);
        assert_eq!(gauge.estimates()[5].precision, 120);
        assert_eq!(
            (
                gauge.estimates()[7].axial_panels,
                gauge.estimates()[7].radial_panels
            ),
            (32, 32)
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
    assert_eq!(imported[0].mean().to_bits(), 0.0f64.to_bits());
    assert!(imported[1].mean() < 0.0);
    assert!(imported[2].mean() < imported[1].mean());

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
        assert_eq!(sample.clock().elapsed(), expected);
        assert_eq!(sample.family_identity(), family_plan.identity());
        assert_eq!(sample.sample_layout().dimensions(), [24; 3]);
        assert_eq!(sample.force_layout().dimensions(), [24; 3]);
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
    assert_eq!(workspace.remaining(), 0);
    assert!(matches!(
        workspace.measure(&family),
        Err(FamilyError::Numerical(_))
    ));
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
