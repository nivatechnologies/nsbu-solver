#![allow(clippy::too_many_lines)]
use crate::fixtures::{self, Fixture};
use crate::{execute, ledger, observer, Mode, Request};
use nsbu_solver::{
    domain::{Domain, Layout},
    spectral::FftBackend,
};
use serde_json::Value;
use std::path::PathBuf;

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;
const QUANTUM: i32 = -20;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-tests")
}

fn request() -> Request {
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

fn fixture(name: &str) -> Fixture {
    fixture_with(name, "MATCHED_SPATIAL", "cox-matthews", [512; 3], ELAPSED, [1.0; 3], 1.0)
}

#[allow(clippy::too_many_arguments)]
fn fixture_with(
    name: &str,
    comparison_kind: &str,
    method: &str,
    force: [usize; 3],
    elapsed: u128,
    lengths: [f64; 3],
    viscosity: f64,
) -> Fixture {
    let layout = Layout::new(N).expect("fixture layout");
    let coefficients = fixtures::synthetic_state(layout);
    fixtures::write_fixture(
        &fixture_root(),
        name,
        &coefficients,
        comparison_kind,
        method,
        force,
        elapsed,
        TARGET,
        N,
        lengths,
        viscosity,
    )
}

fn parse(output: Result<String, String>) -> Value {
    serde_json::from_str(&output.expect("command output")).expect("json output")
}

fn run(fixture_path: &std::path::Path, request: Request) -> Result<String, String> {
    execute(Mode::Run, fixture_path, request, None)
}

#[test]
fn run_binds_identities_and_stays_unqualified() {
    let fixture = fixture("bind");
    let value = parse(run(&fixture.manifest_path, request()));
    assert_eq!(
        value["schema"],
        "p10-n512-analytical-reference-observer-v1"
    );
    assert_eq!(
        value["scope"],
        "analytical-reference-observer:unqualified-sampled-diagnostic"
    );
    assert_eq!(value["qualification"], false);
    assert_eq!(value["accepted_windows"], 0);
    assert_eq!(value["clock"]["exponent"], QUANTUM);
    assert_eq!(value["clock"]["elapsed"].as_u64(), Some(ELAPSED as u64));
    assert_eq!(value["clock"]["target"].as_u64(), Some(TARGET as u64));
    assert_eq!(
        value["clock"]["remaining"].as_u64(),
        Some((TARGET - ELAPSED) as u64)
    );
    assert_eq!(
        value["clock"]["physical_rounding_estimates"],
        serde_json::json!([0.0, 0.0])
    );
    assert_eq!(
        value["identity_binding"]["case_field"],
        nsbu_benchmarks::CASE_SHA256
    );
    assert_eq!(value["identity_binding"]["retained_field"], "8");
    assert_eq!(
        value["identity_binding"]["reviewed_case_sha256"],
        nsbu_benchmarks::CASE_SHA256
    );
    assert!(value["manifest_identity"].as_str().is_some_and(|identity| identity.contains("name=bind")));
    assert_eq!(
        value["observations"]["snapshot"]["coefficient_sha256"],
        fixture.coefficient_sha256
    );
    assert_eq!(value["observations"]["finite"], true);
    assert_eq!(
        value["observations"]["reference_precision_refinement"],
        "required-separate-comparison:not-performed-here"
    );
    let names: Vec<&str> = value["observations"]["quantities"]
        .as_array()
        .expect("quantities")
        .iter()
        .map(|quantity| quantity["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        [
            "velocity",
            "gradient",
            "hessian",
            "vorticity",
            "pressure",
            "pressure_gradient"
        ]
    );
}

#[test]
fn quantities_carry_global_regions_peaks_and_gauge() {
    let fixture = fixture("coverage");
    let value = parse(run(&fixture.manifest_path, request()));
    let quantities = value["observations"]["quantities"]
        .as_array()
        .expect("quantities");
    for quantity in quantities {
        let name = quantity["name"].as_str().expect("name");
        assert_eq!(quantity["global"]["samples"].as_u64(), Some(16 * 16 * 16), "{name}");
        assert_eq!(quantity["regions"].as_array().expect("regions").len(), 5, "{name}");
        assert_eq!(quantity["grid_complete"], true, "{name}");
        for key in ["error", "relative_error", "reference"] {
            let peak = &quantity["peaks"][key];
            assert!(peak["value"].as_f64().is_some_and(f64::is_finite), "{name}/{key}");
            assert_eq!(peak["index"].as_array().expect("index").len(), 3, "{name}/{key}");
        }
    }
    let gauge = &value["observations"]["pressure_gauge"];
    assert!(gauge["measured_analytical_lattice_mean"].as_f64().is_some_and(f64::is_finite));
    assert!(gauge["actual_pressure_lattice_mean_witness"].as_f64().is_some_and(f64::is_finite));
    assert_eq!(
        gauge["imported_high_precision_gauge_artifact"],
        "not-imported;separate-required-comparison"
    );
    assert_eq!(
        gauge["kind"],
        "measured-analytical-sample-lattice-mean-subtracted"
    );
    assert_eq!(value["requested"]["root_budget"], 32);
    assert!(value["work"]["reference_evaluations"].as_u64().expect("evals") > 0);
}

#[test]
fn preflight_reports_fits_without_executing() {
    let fixture = fixture("preflight");
    let value = parse(execute(
        Mode::Preflight,
        &fixture.manifest_path,
        request(),
        None,
    ));
    assert_eq!(value["fits"], true);
    assert!(value["observations"].is_null());
    assert!(value["ledger"]["total_bytes"].as_u64().expect("bytes") > 0);
    assert_eq!(
        value["ledger"]["basis"],
        "conservative-admission-upper-bound-not-measured-allocator-peak"
    );
}

#[test]
fn results_publish_create_only() {
    let fixture = fixture("publish");
    let output = fixture_root().join("publish").join("result.json");
    let _ = std::fs::remove_file(&output);
    let message = execute(
        Mode::Preflight,
        &fixture.manifest_path,
        request(),
        Some(&output),
    )
    .expect("first publish");
    assert!(message.starts_with("published-create-only"));
    let error = execute(
        Mode::Preflight,
        &fixture.manifest_path,
        request(),
        Some(&output),
    )
    .expect_err("second publish must refuse");
    assert!(error.contains("create-only"), "{error}");
}

#[test]
fn tampered_provenance_and_case_are_refused() {
    let fixture = fixture("tamper");
    let file_tampered =
        fixtures::tamper(&fixture.manifest_path, "file-hash", &["file_sha256"]);
    let error = run(&file_tampered, request()).expect_err("tampered file hash must refuse");
    assert!(error.contains("hash") || error.contains("SHA-256"), "{error}");
    let case_tampered =
        fixtures::tamper(&fixture.manifest_path, "case-hash", &["evolution", "case_sha256"]);
    let error = run(&case_tampered, request()).expect_err("tampered case must refuse");
    assert!(error.contains("case"), "{error}");
}

#[test]
fn unsupported_profiles_are_refused() {
    let method = fixture_with(
        "ho-method",
        "MATCHED_SPATIAL",
        "hochbruck-ostermann",
        [512; 3],
        ELAPSED,
        [1.0; 3],
        1.0,
    );
    let error = run(&method.manifest_path, request()).expect_err("HO must refuse");
    assert!(
        error.contains("cox-matthews") || error.contains("evolution"),
        "{error}"
    );
    let diagnostic = fixture_with(
        "method-diagnostic",
        "METHOD_DIAGNOSTIC",
        "cox-matthews",
        [512; 3],
        ELAPSED,
        [1.0; 3],
        1.0,
    );
    let error = run(&diagnostic.manifest_path, request()).expect_err("method profile must refuse");
    assert!(error.contains("method-diagnostic"), "{error}");
    let shifted = fixture_with(
        "shifted-domain",
        "MATCHED_SPATIAL",
        "cox-matthews",
        [512; 3],
        ELAPSED,
        [2.0; 3],
        1.0,
    );
    let error = run(&shifted.manifest_path, request()).expect_err("non-unit domain must refuse");
    assert!(error.contains("unit-cube"), "{error}");
}

fn fixture_target(name: &str, target: u128) -> Fixture {
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
        target,
        N,
        [1.0; 3],
        1.0,
    )
}

#[test]
fn endpoint_and_foreign_clocks_are_refused() {
    let endpoint = fixture_with(
        "endpoint",
        "MATCHED_SPATIAL",
        "cox-matthews",
        [512; 3],
        TARGET,
        [1.0; 3],
        1.0,
    );
    let error = run(&endpoint.manifest_path, request()).expect_err("endpoint must refuse");
    assert!(error.contains("remaining"), "{error}");
    let foreign = fixture_target("foreign-target", TARGET * 2);
    let error = run(&foreign.manifest_path, request()).expect_err("foreign clock must refuse");
    assert!(error.contains("time identity"), "{error}");
    let foreign_exponent = fixture_target("foreign-exponent", TARGET / 2);
    let error = run(&foreign_exponent.manifest_path, request())
        .expect_err("foreign exponent clock must refuse");
    assert!(error.contains("time identity"), "{error}");
}

#[test]
fn work_and_memory_caps_refuse_before_allocation() {
    let fixture = fixture("caps");
    let starved = Request {
        max_reference_evaluations: 1,
        ..request()
    };
    let error = run(&fixture.manifest_path, starved).expect_err("work cap must refuse");
    assert!(error.contains("work preflight refusal"), "{error}");
    let (ledger, _) = observer::ReferenceObserver::preflight(test_inputs(32)).expect("baseline");
    let starved_memory = Request {
        cap: ledger.total_bytes - 1,
        ..request()
    };
    let error = run(&fixture.manifest_path, starved_memory).expect_err("cap-1 must refuse");
    assert!(error.contains("exceeds cap"), "{error}");
}

#[test]
fn forged_ledgers_are_refused_before_allocation() {
    let inputs = test_inputs(32);
    let (baseline, _) = observer::ReferenceObserver::preflight(inputs).expect("baseline");
    let mut ledger = baseline;
    ledger.total_bytes -= 1;
    let catalog = nsbu_solver::spectral::FftCatalog::new(FftBackend::OwnedRadix, baseline.catalog_bytes)
        .expect("fixture catalog");
    match observer::ReferenceObserver::new(inputs, &catalog, &ledger, 1 << 30) {
        Err(nsbu_solver::SolverError::InvalidPayload) => {}
        Err(error) => panic!("unexpected refusal: {error:?}"),
        Ok(_) => panic!("forged ledger must refuse"),
    }
}

#[test]
fn coefficients_are_observed_read_only() {
    let fixture = fixture("readonly");
    let value = parse(run(&fixture.manifest_path, request()));
    assert_eq!(
        value["observations"]["snapshot"]["coefficient_sha256"],
        fixture.coefficient_sha256
    );
    assert_eq!(
        value["observations"]["snapshot"]["coefficient_sha256"],
        crate::cache::coefficient_sha256(fixture.coefficients.each_ref().map(Vec::as_slice))
    );
}

#[test]
fn n512_ledger_is_arithmetic_only_and_bounded() {
    let record_path = fixtures::write_clock_record(&fixture_root(), "n512");
    let value = parse(crate::n512::n512_ledger(&record_path, ledger_request(), None));
    assert_eq!(value["mode"], "n512-ledger");
    assert_eq!(value["qualification"], false);
    assert_eq!(value["requested"]["velocity_sample_dimension"], 512);
    assert_eq!(
        value["basis"],
        "arithmetic-preflight-only:no-availability-check:no-allocation-of-state-observer-catalog-force-table-cache-snapshot"
    );
    let total = value["budget"]["total_bytes"].as_u64().expect("total");
    assert!(total > 0);
    assert_eq!(
        value["budget"]["fits_baccus_512_gib_nominal_estimate"],
        value["budget"]["headroom_vs_nominal_estimate_bytes"].is_number()
    );
    let reference_cache = value["ledger"]["exact_reservation_bytes"]["reference_cache"]
        .as_u64()
        .expect("cache");
    assert!(reference_cache > 512_u64.pow(3));
}

#[test]
fn n512_ledger_refuses_records_that_are_resumable_or_qualified() {
    let record_path = fixture_root().join("n512-resumable.json");
    std::fs::create_dir_all(fixture_root()).expect("fixture root");
    std::fs::write(
        &record_path,
        serde_json::json!({"identity": "case=x", "resumable": true, "qualification": false}).to_string(),
    )
    .expect("record");
    let error = crate::n512::n512_ledger(&record_path, ledger_request(), None)
    .expect_err("resumable record must refuse");
    assert!(error.contains("non-resumable"), "{error}");
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

#[test]
fn n512_ledger_refuses_foreign_identities() {
    for (name, identity) in [
        (
            "foreign-case",
            format!("case={}0;retained=512;provider=p", nsbu_benchmarks::CASE_SHA256),
        ),
        ("foreign-retained", "case=deadbeef;retained=256;provider=p".to_owned()),
        ("missing-provider", "case=deadbeef;retained=512".to_owned()),
    ] {
        let record_path = fixture_root().join(format!("n512-{name}.json"));
        std::fs::create_dir_all(fixture_root()).expect("fixture root");
        std::fs::write(
            &record_path,
            serde_json::json!({"identity": identity, "resumable": false, "qualification": false})
                .to_string(),
        )
        .expect("record");
        let error = crate::n512::n512_ledger(&record_path, ledger_request(), None)
            .expect_err("foreign identity must refuse");
        assert!(
            error.contains("case") || error.contains("retained") || error.contains("provider"),
            "{name}: {error}"
        );
    }
}

#[test]
fn cli_surface_refusals_are_bounded() {
    let error = crate::command(vec!["bogus".into()]).expect_err("unknown mode must refuse");
    assert!(error.contains("unknown mode"), "{error}");
    let error = crate::command(vec!["n512-ledger".into(), "one.json".into()])
        .expect_err("truncated ledger arguments must refuse");
    assert!(error.contains("seven arguments"), "{error}");
    let error = crate::command(vec![
        "preflight".into(),
        "manifest.json".into(),
        "not-a-number".into(),
    ])
    .expect_err("truncated run arguments must refuse");
    assert!(error.contains("invalid integer") || error.contains("eleven or twelve"), "{error}");
    let help = crate::command(vec!["--help".into()]).expect("help");
    assert!(help.contains("schema=p10-n512-analytical-reference-observer-v1"), "{help}");
    assert!(help.contains("unqualified-sampled-diagnostic"), "{help}");
}

fn test_inputs(root_budget: usize) -> ledger::AdmissionInputs {
    ledger::AdmissionInputs {
        source: Domain::new(N, [1.0; 3], 1.0).expect("fixture domain"),
        velocity_samples: Layout::new([16; 3]).expect("fixture velocity samples"),
        pressure_samples: Layout::new([16; 3]).expect("fixture pressure samples"),
        force_samples: Layout::new([16; 3]).expect("fixture force samples"),
        workers: 1,
        backend: FftBackend::OwnedRadix,
        root_budget,
        identity_len: 64,
    }
}
