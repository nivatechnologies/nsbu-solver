#![allow(clippy::too_many_lines)]
//! Admission/guard-arm coverage: ledger normalization, observer construction and
//! observation guards, record optional arms, quantity arithmetic guards, report
//! witnesses and the CLI admission surface.
use crate::fixtures::{self, clock_record_json, Fixture};
use crate::{
    command, insert, ledger, n512, observer, quantity, report, Request,
};
use nsbu_benchmarks::fields::reference::ReferenceEvaluation;
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
use std::{fs, path::PathBuf};

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

pub(crate) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-coverage")
}

pub(crate) fn fixture(name: &str) -> Fixture {
    let layout = Layout::new(N).expect("fixture layout");
    let coefficients = fixtures::synthetic_state(layout);
    fixtures::write_fixture(
        &fixture_root(),
        name,
        &coefficients,
        "MATCHED_SPATIAL",
        "cox-matthews",
        [512; 3],
        ELAPSED,
        TARGET,
        N,
        [1.0; 3],
        1.0,
    )
}

pub(crate) fn request() -> Request {
    Request {
        velocity_samples: 16,
        pressure_samples: 16,
        force_samples: 16,
        workers: 1,
        root_budget: 32,
        max_reference_evaluations: 1 << 24,
        cap: 1 << 30,
        velocity_floor: 1e-12,
        pressure_floor: 1e-12,
        backend: "owned-radix".to_owned(),
    }
}

fn ledger_request() -> Request {
    Request {
        velocity_samples: 512,
        pressure_samples: 1024,
        force_samples: 1024,
        workers: 32,
        root_budget: 32,
        max_reference_evaluations: 1 << 40,
        cap: 1 << 48,
        velocity_floor: 1.0,
        pressure_floor: 1.0,
        backend: "rustfft-6.4.1-avx-avx2-fma".to_owned(),
    }
}

pub(crate) fn test_inputs() -> ledger::AdmissionInputs {
    ledger::AdmissionInputs {
        source: Domain::new(N, [1.0; 3], 1.0).expect("fixture domain"),
        velocity_samples: Layout::new([16; 3]).expect("fixture velocity samples"),
        pressure_samples: Layout::new([16; 3]).expect("fixture pressure samples"),
        force_samples: Layout::new([16; 3]).expect("fixture force samples"),
        workers: 1,
        backend: FftBackend::OwnedRadix,
        root_budget: 32,
        identity_len: 64,
    }
}

#[test]
fn ledger_normalization_refuses_uncovered_grids_and_degenerate_counts() {
    let base = test_inputs();
    let uncovered = ledger::AdmissionInputs {
        velocity_samples: Layout::new([4; 3]).expect("coarse samples"),
        ..base
    };
    assert!(matches!(
        ledger::preflight(uncovered),
        Err(SolverError::InvalidDomain)
    ));
    assert!(matches!(
        ledger::preflight(ledger::AdmissionInputs { workers: 0, ..base }),
        Err(SolverError::InvalidPayload)
    ));
    assert!(matches!(
        ledger::preflight(ledger::AdmissionInputs {
            root_budget: 129,
            ..base
        }),
        Err(SolverError::InvalidPayload)
    ));
}

#[test]
fn observer_construction_and_observation_arms_refuse() {
    let inputs = test_inputs();
    let (ledger, _) = observer::ReferenceObserver::preflight(inputs).expect("baseline");
    let catalog = nsbu_solver::spectral::FftCatalog::new(FftBackend::OwnedRadix, ledger.catalog_bytes)
        .expect("fixture catalog");
    assert!(matches!(
        observer::ReferenceObserver::new(inputs, &catalog, &ledger, ledger.total_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let mut observer =
        observer::ReferenceObserver::new(inputs, &catalog, &ledger, 1 << 30).expect("observer");
    let clock = TickClock::restore(-20, TARGET, ELAPSED, TARGET - ELAPSED).expect("clock");
    let short = core::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); 4]);
    assert!(matches!(
        observer.observe(clock, short.each_ref().map(Vec::as_slice), 1e-12, 1e-12, 32),
        Err(SolverError::InvalidPayload)
    ));
    let half_len = Domain::new(N, [1.0; 3], 1.0).expect("domain").layout().half_len();
    let mut broken = vec![Complex64::new(0.0, 0.0); half_len];
    broken[0] = Complex64::new(0.0, 0.5);
    let broken = [broken.clone(), broken.clone(), broken];
    assert!(observer
        .observe(
            clock,
            broken.each_ref().map(Vec::as_slice),
            1e-12,
            1e-12,
            32
        )
        .is_err());
}

#[test]
fn ledger_record_optional_arms_bind_or_refuse() {
    fs::create_dir_all(fixture_root()).expect("fixture root");
    for (name, mutate, fragment) in [
        (
            "non-hex-record-commit",
            Box::new(|record: &mut Value| record["source_commit"] = json!("xyz"))
                as Box<dyn Fn(&mut Value)>,
            "record source_commit differs",
        ),
        ("missing-clock", Box::new(|record: &mut Value| {
            let object = record.as_object_mut().expect("object");
            object.remove("clock");
        }), "clock is missing"),
        ("missing-epoch", Box::new(|record: &mut Value| {
            let object = record.as_object_mut().expect("object");
            object.remove("epoch");
        }), "epoch"),
        ("missing-state-hash", Box::new(|record: &mut Value| {
            let object = record.as_object_mut().expect("object");
            object.remove("state_sha256");
        }), "state_sha256"),
    ] {
        let path = fixture_root().join(format!("coverage-{name}.json"));
        let mut record = clock_record_json(name);
        mutate(&mut record);
        fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
        let error = n512::n512_ledger(&path, ledger_request(), None).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }

    // The canonical source_commit string is required: omission is refused.
    let path = fixture_root().join("coverage-missing-commit.json");
    let mut record = clock_record_json("missing-commit");
    record.as_object_mut().expect("object").remove("source_commit");
    fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
    let error = n512::n512_ledger(&path, ledger_request(), None)
        .expect_err("missing canonical source_commit must refuse");
    assert!(error.contains("carries no canonical source_commit string"), "{error}");

    // A numeric source_commit is a wrong-typed identity, never skipped.
    let path = fixture_root().join("coverage-numeric-commit.json");
    let mut record = clock_record_json("numeric-commit");
    record["source_commit"] = json!(1_234_567_890_u64);
    fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
    let error = n512::n512_ledger(&path, ledger_request(), None)
        .expect_err("numeric source_commit must refuse");
    assert!(
        error.contains("source_commit is present but is not a string"),
        "{error}"
    );

    let path = fixture_root().join("coverage-no-identity.json");
    fs::write(&path, r#"{"resumable": false, "qualification": false}"#).expect("record write");
    let error = n512::n512_ledger(&path, ledger_request(), None).expect_err("identity required");
    assert!(error.contains("identity string"), "{error}");
}

#[test]
fn quantity_arithmetic_guards_refuse_nonfinite_measurements() {
    let layout = Layout::new([8; 3]).expect("layout");
    assert!(matches!(
        quantity::maximum(&[-1.0_f64; 512], 512, layout),
        Err(SolverError::InvalidPayload)
    ));
    assert!(matches!(
        quantity::maximum(&[1.0_f64; 100], 100, layout),
        Err(SolverError::InvalidPayload)
    ));
    assert_eq!(quantity::axis_distance(0, 1, 2), 0);

    let source = Domain::new([4; 3], [1.0; 3], 1.0).expect("domain");
    let samples = Layout::new([8; 3]).expect("samples");
    let mut workspace = DerivativeWorkspace::new(source, samples, 1 << 26).expect("sampler");
    let zero = Complex64::new(0.0, 0.0);
    let spectra: Vec<Vec<Complex64>> =
        (0..3).map(|_| vec![zero; source.layout().half_len()]).collect();
    let slices: Vec<&[Complex64]> = spectra.iter().map(Vec::as_slice).collect();
    let points = samples.real_len();
    let mut actual = vec![0.0; points];
    let mut scratch = vec![0.0; points];
    let mut errors = vec![0.0; points];
    let mut references = vec![0.0; points];

    // A velocity cache against a scalar quantity falls into the NaN arm and the
    // hypot guard must refuse the arithmetic.
    let cache: Vec<ReferenceEvaluation> = (0..points)
        .map(|_| ReferenceEvaluation {
            velocity: [0.0; 3],
            gradient: [[0.0; 3]; 3],
            hessian: [[[0.0; 3]; 3]; 3],
            vorticity: [0.0; 3],
            pressure_raw: 0.0,
            pressure_gradient: [0.0; 3],
            root: None,
        })
        .collect();
    let side = quantity::ReferenceSide::Velocity(&cache);
    assert!(matches!(
        quantity::measure::<3>(
            "mismatch",
            PhysicalQuantity::Scalar,
            &mut workspace,
            &slices,
            &side,
            samples,
            TickClock::restore(-20, TARGET, ELAPSED, TARGET - ELAPSED).expect("clock"),
            1e-12,
            32,
            &mut actual,
            &mut scratch,
            &mut errors,
            &mut references,
        ),
        Err(SolverError::ArithmeticResolutionLimited)
    ));

    // Zero references with a zero floor make the relative witness non-finite.
    let rows: Vec<[f64; 4]> = vec![[0.0; 4]; points];
    let side = quantity::ReferenceSide::Pressure { rows: &rows, mean: 0.0 };
    assert!(matches!(
        quantity::measure::<1>(
            "zero-floor",
            PhysicalQuantity::Scalar,
            &mut workspace,
            &slices[..1],
            &side,
            samples,
            TickClock::restore(-20, TARGET, ELAPSED, TARGET - ELAPSED).expect("clock"),
            0.0,
            32,
            &mut actual,
            &mut scratch,
            &mut errors,
            &mut references,
        ),
        Err(SolverError::InvalidPayload)
    ));
}

#[test]
fn report_witnesses_serialize_no_samples_and_stay_finite() {
    let layout = Layout::new([8; 3]).expect("layout");
    let maximum = quantity::SampleMaximum {
        linear: 0,
        index: [0, 0, 0],
        value: 0.0,
    };
    let peak = |name: &'static str, source| {
        crate::quantity::FieldPeak::new(name, source, maximum)
    };
    use crate::quantity::PeakSource;
    let observation = observer::Observations {
        quantities: vec![observer::QuantityMeasurement {
            name: "witness",
            quantity: "Scalar",
            components: 1,
            transforms: 1,
            layout,
            global: nsbu_solver::diagnostics::local::LocalError {
                components: 1,
                samples: 2,
                rms_error: 0.0,
                peak_error: 0.0,
                peak_relative_error: 0.0,
                reference_peak: 1.0,
                relative_floor: 1e-12,
            },
            regions: vec![
                observer::RegionMeasurement {
                    region: "no-samples-region",
                    error: SampledError::NoSamples,
                },
                observer::RegionMeasurement {
                    region: "measured-region",
                    error: SampledError::Measured(
                        nsbu_solver::diagnostics::local::LocalError {
                            components: 1,
                            samples: 2,
                            rms_error: 0.0,
                            peak_error: 0.0,
                            peak_relative_error: 0.0,
                            reference_peak: 1.0,
                            relative_floor: 1e-12,
                        },
                    ),
                },
            ],
            grid_complete: true,
            root_work_charged: 2,
            error_peak: peak("witness", PeakSource::Error),
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
    assert!(report::all_finite(&observation));
    let value = report::observation_value(&observation, "hash", &json!({"elapsed": 0_u64}));
    assert_eq!(value["quantities"][0]["regions"][0]["error"]["state"], "no-samples");
    assert_eq!(value["quantities"][0]["regions"][1]["error"]["state"], "measured");
    assert_eq!(value["quantities"][0]["peaks"]["error"]["source"], "error");
    assert_eq!(value["quantities"][0]["peaks"]["actual"]["source"], "actual");
    assert_eq!(
        value["quantities"][0]["peaks"]["relative_error"]["field_identity"],
        "witness:relative-error"
    );
}

#[test]
fn request_and_cli_admission_arms_refuse() {
    fs::create_dir_all(fixture_root()).expect("fixture root");
    for (name, request, fragment) in [
        ("zero-workers", Request { workers: 0, ..request() }, "worker count or root budget"),
        ("zero-budget", Request { root_budget: 0, ..request() }, "worker count or root budget"),
        ("big-budget", Request { root_budget: 129, ..request() }, "worker count or root budget"),
        ("zero-evaluations", Request { max_reference_evaluations: 0, ..request() }, "missing work"),
        ("zero-cap", Request { cap: 0, ..request() }, "missing work"),
        ("huge-cap", Request { cap: 1 << 61, ..request() }, "exceeds bounded admission"),
        ("huge-evaluations", Request { max_reference_evaluations: 1 << 51, ..request() }, "exceeds bounded admission"),
        ("nan-floor", Request { velocity_floor: f64::NAN, ..request() }, "relative floors"),
        ("negative-floor", Request { pressure_floor: -1.0, ..request() }, "relative floors"),
    ] {
        let error = request.validate().expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }

    let fixture = fixture("cli-arms");
    let manifest = fixture.manifest_path.to_string_lossy().into_owned();
    let error = command(vec![
        "preflight".into(), manifest.clone(), "16".into(), "16".into(), "16".into(),
        "1".into(), "32".into(), "16777216".into(), "1073741824".into(),
        "1e-12".into(), "1e-12".into(), "mkl".into(),
    ])
    .expect_err("unsupported backend must refuse");
    assert!(error.contains("unsupported FFT backend"), "{error}");
    let error = command(vec![
        "preflight".into(), manifest.clone(), "16".into(), "16".into(), "16".into(),
        "300".into(), "32".into(), "16777216".into(), "1073741824".into(),
        "1e-12".into(), "1e-12".into(), "owned-radix".into(),
    ])
    .expect_err("worker admission must refuse");
    assert!(error.contains("bounded admission 1..=256"), "{error}");
    let error = command(vec![
        "preflight".into(), manifest.clone(), "12".into(), "16".into(), "16".into(),
        "1".into(), "32".into(), "16777216".into(), "1073741824".into(),
        "1e-12".into(), "1e-12".into(), "owned-radix".into(),
    ])
    .expect_err("non-power-of-two dimension must refuse");
    assert!(error.contains("power of two"), "{error}");
    let error = command(vec![
        "preflight".into(), manifest.clone(), "16".into(), "16".into(), "16".into(),
        "1".into(), "32".into(), "16777216".into(), "1073741824".into(),
        "floors".into(), "1e-12".into(), "owned-radix".into(),
    ])
    .expect_err("unparsable floor must refuse");
    assert!(error.contains("velocity relative floor"), "{error}");
    let error = command(vec![
        "preflight".into(), manifest, "16".into(), "16".into(), "16".into(),
        "1".into(), "32".into(), "16777216".into(), "1073741824".into(),
        "1e-12".into(), "floors".into(), "owned-radix".into(),
    ])
    .expect_err("unparsable pressure floor must refuse");
    assert!(error.contains("pressure relative floor"), "{error}");

    let output_path = fixture_root().join("cli-preflight-output.json");
    let _ = fs::remove_file(&output_path);
    let message = command(vec![
        "preflight".into(),
        fixture.manifest_path.to_string_lossy().into_owned(),
        "16".into(), "16".into(), "16".into(),
        "1".into(), "32".into(), "16777216".into(), "1073741824".into(),
        "1e-12".into(), "1e-12".into(), "owned-radix".into(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("preflight publishes");
    assert!(message.starts_with("published-create-only"), "{message}");
    let published: Value = serde_json::from_slice(&fs::read(&output_path).expect("published"))
        .expect("json");
    assert_eq!(published["fits"], true);

    let record_path = fixtures::write_clock_record(&fixture_root(), "cli-ledger");
    let output_path = fixture_root().join("cli-ledger-output.json");
    let _ = fs::remove_file(&output_path);
    let message = command(vec![
        "n512-ledger".into(),
        record_path.to_string_lossy().into_owned(),
        "512".into(), "1024".into(), "1024".into(),
        "32".into(), "1099511627776".into(), "281474976710656".into(),
        output_path.to_string_lossy().into_owned(),
    ])
    .expect("ledger publishes");
    assert!(message.starts_with("published-create-only"), "{message}");

    assert_eq!(insert(Value::from(7), "key", json!(1)), Value::from(7));
    assert!(insert(json!({}), "key", json!(1))["key"].is_number());
}
