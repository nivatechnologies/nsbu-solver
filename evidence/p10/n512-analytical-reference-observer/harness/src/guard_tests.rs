#![allow(clippy::too_many_lines)]
//! Report finiteness guards, measurement buffer/clock refusals and snapshot
//! profile binding admission arms.
use crate::fixtures::Fixture;
use crate::{observer, quantity, report, Request};
use nsbu_solver::{
    Complex64, SolverError,
    diagnostics::{
        derivatives::DerivativeWorkspace,
        local::SampledError,
        physical::PhysicalQuantity,
    },
    domain::{Domain, Layout, TickClock},
    spectral::FftBackend,
};
use serde_json::{json, Value};
use std::fs;

use crate::arm_tests::{test_inputs};

const TARGET: u128 = 8192;
const ELAPSED: u128 = 1024;

fn fixture(name: &str) -> Fixture {
    crate::arm_tests::fixture(name)
}

fn request() -> Request {
    crate::arm_tests::request()
}

#[test]
fn report_finiteness_guards_refuse_nonfinite_witnesses() {
    let layout = Layout::new([8; 3]).expect("layout");
    let maximum = quantity::SampleMaximum { linear: 0, index: [0, 0, 0], value: 0.0 };
    let peak = |name: &'static str, source| quantity::FieldPeak::new(name, source, maximum);
    use crate::quantity::PeakSource;
    let local = nsbu_solver::diagnostics::local::LocalError {
        components: 1,
        samples: 2,
        rms_error: 0.0,
        peak_error: 0.0,
        peak_relative_error: 0.0,
        reference_peak: 1.0,
        relative_floor: 1e-12,
    };
    let build = |peak_value: f64, region: SampledError| observer::Observations {
        quantities: vec![observer::QuantityMeasurement {
            name: "witness",
            quantity: "Scalar",
            components: 1,
            transforms: 1,
            layout,
            global: local,
            regions: vec![observer::RegionMeasurement { region: "r", error: region }],
            grid_complete: true,
            root_work_charged: 2,
            error_peak: quantity::FieldPeak::new(
                "witness",
                PeakSource::Error,
                quantity::SampleMaximum { linear: 0, index: [0, 0, 0], value: peak_value },
            ),
            relative_peak: peak("witness", PeakSource::RelativeError),
            reference_peak: peak("witness", PeakSource::Reference),
            actual_peak: peak("witness", PeakSource::Actual),
            peak_height_error: 0.0,
            peak_distance: [0, 0, 0],
        }],
        pressure_gauge_mean: 0.0,
        actual_pressure_lattice_mean: 0.0,
        provider_root_iterations: 0,
        executed_scalar_transforms: 1,
    };
    assert!(!report::all_finite(&build(f64::INFINITY, SampledError::NoSamples)));
    let nan_region = SampledError::Measured(nsbu_solver::diagnostics::local::LocalError {
        rms_error: f64::NAN,
        ..local
    });
    assert!(!report::all_finite(&build(0.0, nan_region)));
    let bad_global = nsbu_solver::diagnostics::local::LocalError {
        peak_error: f64::INFINITY,
        ..local
    };
    let mut observations = build(0.0, SampledError::NoSamples);
    observations.quantities[0].global = bad_global;
    assert!(!report::all_finite(&observations));
}

#[test]
fn measurement_buffers_and_foreign_clocks_are_refused() {
    let source = Domain::new([4; 3], [1.0; 3], 1.0).expect("domain");
    let samples = Layout::new([8; 3]).expect("samples");
    let mut workspace = DerivativeWorkspace::new(source, samples, 1 << 26).expect("sampler");
    let zero = Complex64::new(0.0, 0.0);
    let spectra: Vec<Vec<Complex64>> =
        (0..3).map(|_| vec![zero; source.layout().half_len()]).collect();
    let slices: Vec<&[Complex64]> = spectra.iter().map(Vec::as_slice).collect();
    let rows: Vec<[f64; 4]> = vec![[0.0; 4]; samples.real_len()];
    let side = quantity::ReferenceSide::Pressure { rows: &rows, mean: 0.0 };
    let clock = TickClock::restore(-20, TARGET, ELAPSED, TARGET - ELAPSED).expect("clock");
    assert!(matches!(
        quantity::measure::<1>(
            "short",
            PhysicalQuantity::Scalar,
            &mut workspace,
            &slices[..1],
            &side,
            samples,
            clock,
            1e-12,
            32,
            &mut [0.0; 4],
            &mut [0.0; 4],
            &mut [0.0; 4],
            &mut [0.0; 4],
        ),
        Err(SolverError::InvalidPayload)
    ));

    let inputs = test_inputs();
    let (ledger, _) = observer::ReferenceObserver::preflight(inputs).expect("baseline");
    let catalog = nsbu_solver::spectral::FftCatalog::new(FftBackend::OwnedRadix, ledger.catalog_bytes)
        .expect("catalog");
    let mut observer =
        observer::ReferenceObserver::new(inputs, &catalog, &ledger, 1 << 30).expect("observer");
    let half_len = inputs.source.layout().half_len();
    let velocity = core::array::from_fn(|_| vec![zero; half_len]);
    assert!(matches!(
        observer.observe(
            TickClock::restore(-19, TARGET, ELAPSED, TARGET - ELAPSED).expect("foreign clock"),
            velocity.each_ref().map(Vec::as_slice),
            1e-12,
            1e-12,
            32,
        ),
        Err(SolverError::InvalidPayload)
    ));
}

#[test]
fn snapshot_profile_bindings_must_match_the_identity() {

    let fixture = fixture("profile-arms");
    let directory = fixture.manifest_path.parent().expect("directory");
    for (name, value, expect_ok) in [
        ("matching", "owned-radix-fixture", true),
        ("foreign", "some-other-profile", false),
    ] {
        let mut manifest: Value = serde_json::from_slice(&fs::read(&fixture.manifest_path).expect("manifest"))
            .expect("manifest");
        manifest["profile"] = json!({
            "kind": "identity-profile-field",
            "value": value,
        });
        let path = directory.join(format!("profile-{name}.manifest.json"));
        fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).expect("manifest");
        let result = crate::execute(
            crate::Mode::Preflight,
            &path,
            Request { root_budget: 32, ..request() },
            None,
        );
        if expect_ok {
            result.expect("matching profile binds");
        } else {
            let error = result.expect_err("foreign profile must refuse");
            assert!(error.contains("profile"), "{error}");
        }
    }
}
