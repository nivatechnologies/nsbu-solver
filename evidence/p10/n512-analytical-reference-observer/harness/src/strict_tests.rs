#![allow(dead_code, clippy::too_many_lines)]
//! Independently checked fixtures for the repaired blockers: live-array ledger
//! derivation, strict provenance parsing, atomic publication, and the numeric
//! derivative/gauge/peak witness contracts, with hand-computed expectations.
use crate::fixtures;
use crate::quantity::{axis_distance, entry, maximum};
use crate::{execute, publication, Mode, Request};
use nsbu_solver::diagnostics::physical::PhysicalQuantity;
use nsbu_solver::domain::Layout;
use serde_json::Value;
use std::path::PathBuf;

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-strict")
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

fn fixture(name: &str) -> fixtures::Fixture {
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

fn parse(output: Result<String, String>) -> Value {
    serde_json::from_str(&output.expect("command output")).expect("json output")
}

fn run(fixture_path: &std::path::Path, request: Request) -> Result<String, String> {
    execute(Mode::Run, fixture_path, request, None)
}

#[test]
fn duplicate_json_keys_are_refused_everywhere() {
    for (name, text) in [
        ("top", r#"{"a":1,"a":2}"#),
        ("nested", r#"{"a":{"b":1,"b":2}}"#),
        ("trailing", r#"{"a":1} {"b":2}"#),
        ("escaped-key", r#"{"a\u0041":1}"#),
    ] {
        let error = crate::provenance::reject_duplicate_keys(text).expect_err(name);
        assert!(
            error.contains("duplicate") || error.contains("trailing") || error.contains("escaped"),
            "{name}: {error}"
        );
    }
    for (name, text) in [
        ("plain", r#"{"a":1,"b":{"c":2,"d":3},"e":[1,2,3]}"#),
        ("brace-in-string", r#"{"a":"x{y},z\"}","b":1}"#),
    ] {
        crate::provenance::reject_duplicate_keys(text).expect(name);
    }
}

#[test]
fn identity_fields_are_unique_and_delimited() {
    let fields =
        crate::provenance::identity_fields_strict("case=abc;retained=8;provider=p").expect("ok");
    assert_eq!(fields.len(), 3);
    for (name, identity) in [
        ("duplicate", "case=a;case=b"),
        ("no-equals", "case=a;bogus"),
        ("empty-field", "case=a;;provider=p"),
    ] {
        let error = crate::provenance::identity_fields_strict(identity).expect_err(name);
        assert!(error.contains("identity"), "{name}: {error}");
    }
}

#[test]
fn entry_schedule_matches_the_reviewed_hand_computed_values() {
    assert_eq!(entry(PhysicalQuantity::Scalar, 0), (0, [0, 0, 0]));
    assert_eq!(entry(PhysicalQuantity::ScalarGradient, 1), (0, [0, 1, 0]));
    assert_eq!(entry(PhysicalQuantity::Vector, 2), (2, [0, 0, 0]));
    assert_eq!(entry(PhysicalQuantity::Gradient, 4), (1, [0, 1, 0]));
    assert_eq!(entry(PhysicalQuantity::Hessian, 0), (0, [2, 0, 0]));
    assert_eq!(entry(PhysicalQuantity::Hessian, 4), (0, [0, 2, 0]));
    assert_eq!(entry(PhysicalQuantity::Vorticity, 0), (2, [0, 1, 0]));
    assert_eq!(entry(PhysicalQuantity::Vorticity, 2), (1, [1, 0, 0]));
}

#[test]
fn periodic_axis_distance_is_hand_checked_minimum_image() {
    assert_eq!(axis_distance(8, 1, 6), 3);
    assert_eq!(axis_distance(8, 0, 4), 4);
    assert_eq!(axis_distance(8, 7, 0), 1);
    assert_eq!(axis_distance(4, 0, 0), 0);
    assert_eq!(axis_distance(2, 1, 0), 1);
    assert_eq!(axis_distance(0, 3, 5), 0);
}

#[test]
fn peak_maximum_first_tie_and_rejections_are_hand_checked() {
    let layout = Layout::new([4; 3]).expect("layout");
    let mut values = vec![0.0; 64];
    values[0] = 1.0;
    values[1] = 3.0;
    values[2] = 2.0;
    values[63] = 3.0;
    let peak = maximum(&values, 64, layout).expect("maximum");
    assert_eq!((peak.linear, peak.index, peak.value), (1, [0, 0, 1], 3.0));
    values[5] = -1.0;
    assert!(maximum(&values, 64, layout).is_err());
    let mismatch = Layout::new([2; 3]).expect("mismatch layout");
    assert!(maximum(&values, 64, mismatch).is_err());
}

#[test]
fn pressure_lattice_mean_is_hand_checked() {
    let rows = [[2.0, 9.0, 0.0, 0.0], [4.0, 0.0, 0.0, 0.0], [6.0, 0.0, 0.0, 0.0], [8.0, 0.0, 0.0, 0.0]];
    assert_eq!(crate::cache::lattice_mean(&rows, 0).expect("mean"), 5.0);
    assert_eq!(crate::cache::lattice_mean(&rows, 1).expect("mean"), 2.25);
    assert!(crate::cache::lattice_mean(&[], 0).is_err());
    let nan_rows = [[f64::NAN, 0.0, 0.0, 0.0]];
    assert!(crate::cache::lattice_mean(&nan_rows, 0).is_err());
}

fn n512_ledger_output() -> Value {
    std::fs::create_dir_all(fixture_root()).expect("fixture root");
    let record_path = fixture_root().join("strict-record.json");
    std::fs::write(
        &record_path,
        serde_json::to_vec_pretty(&fixtures::clock_record_json("strict")).unwrap(),
    )
    .expect("record");
    parse(crate::n512::n512_ledger(
        &record_path,
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
        },
        None,
    ))
}

#[test]
fn live_array_ledger_matches_the_actual_n512_allocations() {
    let value = n512_ledger_output();
    let velocity = 512_usize.pow(3);
    let pressure = 1024_usize.pow(3);
    let largest = velocity.max(pressure);
    let expected = (2 * largest + 2 * velocity + 2 * pressure) * std::mem::size_of::<f64>();
    let previous = (3 * velocity + 2 * pressure) * std::mem::size_of::<f64>();
    let comparison = value["ledger"]["live_array_bytes"]["comparison_arrays"]
        .as_u64()
        .expect("comparison arrays");
    assert_eq!(comparison, expected as u64);
    assert_eq!(
        expected - previous,
        16_106_127_360,
        "the corrected live-array term must recover the prior undercount exactly"
    );
    let total = value["budget"]["total_bytes"].as_u64().expect("total");
    assert!(total >= comparison + 16_106_127_360);
    assert_eq!(value["fits"], true);
    let headroom = value["budget"]["headroom_vs_nominal_estimate_bytes"].as_u64();
    assert_eq!(
        value["budget"]["fits_baccus_512_gib_nominal_estimate"],
        headroom.is_some()
    );
    if let Some(headroom) = headroom {
        assert_eq!(headroom, 549_755_813_888 - total);
    }
}

#[test]
fn n512_ledger_refuses_duplicate_or_unbound_records() {
    std::fs::create_dir_all(fixture_root()).expect("fixture root");
    let duplicate = fixture_root().join("strict-duplicate.json");
    std::fs::write(
        &duplicate,
        "{\"identity\":\"case=a\",\"identity\":\"case=b\",\"resumable\":false,\"qualification\":false}",
    )
    .expect("record");
    let ledger_request = Request {
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
    };
    let error = crate::n512::n512_ledger(&duplicate, ledger_request.clone(), None)
        .expect_err("duplicate keys must refuse");
    assert!(error.contains("duplicate"), "{error}");
    let missing_profile = fixture_root().join("strict-no-profile.json");
    let mut record = fixtures::clock_record_json("strict");
    record["identity"] = serde_json::Value::String(format!(
        "source={};case={};retained=512;schema=p10-analytical-reference-fixture-record-v1;\
         provider=parallel-reduced-v2-force-w3;method=cox-matthews;endpoint=4096",
        fixtures::fixture_source_commit(),
        nsbu_benchmarks::CASE_SHA256,
    ));
    std::fs::write(&missing_profile, serde_json::to_vec_pretty(&record).unwrap())
        .expect("record");
    let error = crate::n512::n512_ledger(&missing_profile, ledger_request, None)
        .expect_err("profile binding is required");
    assert!(error.contains("profile"), "{error}");
}

#[test]
fn strict_manifest_provenance_tampering_is_refused() {
    let fixture = fixture("strict-provenance");
    let text = std::fs::read_to_string(&fixture.manifest_path).expect("manifest");
    let duplicated = text.replacen(
        "{",
        "{\n  \"schema\": \"p10-snapshot-comparison-input-v1\",",
        1,
    );
    let duplicate_path = fixture_root().join("strict-provenance/duplicate.json");
    std::fs::write(&duplicate_path, &duplicated).expect("write");
    let error = run(&duplicate_path, request()).expect_err("duplicate key must refuse");
    assert!(error.contains("duplicate"), "{error}");
    let foreign = text.replace(
        "p10-snapshot-comparison-input-v1",
        "p10-snapshot-comparison-input-v2",
    );
    let foreign_path = fixture_root().join("strict-provenance/foreign.json");
    std::fs::write(&foreign_path, &foreign).expect("write");
    let error = run(&foreign_path, request()).expect_err("foreign schema must refuse");
    assert!(error.contains("schema"), "{error}");
    std::fs::write(
        fixture_root().join("strict-provenance/plan.json"),
        b"tampered plan bytes",
    )
    .expect("plan tamper");
    let error = run(&fixture.manifest_path, request()).expect_err("plan digest must bind");
    assert!(error.contains("plan"), "{error}");
    std::fs::write(fixture_root().join("strict-provenance/plan.json"), {
        let restored = fixtures::write_fixture(
            &fixture_root().join("restored"),
            "strict-provenance",
            &fixtures::synthetic_state(Layout::new(N).expect("fixture layout")),
            "MATCHED_SPATIAL",
            "cox-matthews",
            [512; 3],
            ELAPSED,
            TARGET,
            N,
            [1.0; 3],
            1.0,
        );
        std::fs::read(restored.manifest_path.parent().unwrap().join("plan.json"))
            .expect("restored plan")
    })
    .expect("plan restore");
    let identity_tampered =
        fixtures::tamper(&fixture.manifest_path, "identity-field", &["identity"]);
    let error = run(&identity_tampered, request()).expect_err("identity must bind snapshot");
    assert!(error.contains("identity"), "{error}");
}

#[test]
fn atomic_publication_refusals_and_write_failure_leave_nothing() {
    let directory = fixture_root().join("publish");
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("result.json");
    let _ = std::fs::remove_file(&path);
    publication::publish(&path, b"first".as_slice()).expect("publish");
    assert_eq!(std::fs::read(&path).expect("read"), b"first");
    let error = publication::publish(&path, b"second".as_slice()).expect_err("overwrite");
    assert!(error.contains("create-only"), "{error}");
    let link = directory.join("link.json");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&path, &link).expect("symlink");
    let error = publication::publish(&link, b"x".as_slice()).expect_err("symlink");
    assert!(error.contains("create-only"), "{error}");
    let missing = directory.join("missing-parent").join("result.json");
    let error = publication::publish(&missing, b"x".as_slice()).expect_err("missing parent");
    assert!(error.contains("temporary-create"), "{error}");
    let injected = directory.join("injected.json");
    let _ = std::fs::remove_file(&injected);
    let error = publication::publish_with_fault(
        &injected,
        b"x".as_slice(),
        Some(publication::PublicationFault::Write),
    )
    .expect_err("injected write failure");
    assert!(error.contains("injected"), "{error}");
    assert!(!injected.exists());
    let leftovers = std::fs::read_dir(&directory)
        .expect("listing")
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .filter(|name| name.starts_with('.'))
        .collect::<Vec<_>>();
    assert!(leftovers.is_empty(), "temporary leak: {leftovers:?}");
}

#[test]
fn run_reports_actual_peaks_height_errors_and_gauge_witnesses() {
    let fixture = fixture("strict-witness");
    let value = parse(run(&fixture.manifest_path, request()));
    for quantity in value["observations"]["quantities"]
        .as_array()
        .expect("quantities")
    {
        let name = quantity["name"].as_str().expect("name");
        let peaks = &quantity["peaks"];
        for key in ["error", "relative_error", "reference", "actual"] {
            let peak = &peaks[key];
            assert!(
                peak["value"].as_f64().is_some_and(|v| v.is_finite() && v >= 0.0),
                "{name}/{key}"
            );
            assert!(!peak["source"].as_str().expect("source").is_empty());
            assert!(peak["field_identity"]
                .as_str()
                .expect("identity")
                .starts_with(name));
        }
        let height = peaks["actual_vs_reference_peak_height_error"]
            .as_f64()
            .expect("height error");
        let expected = peaks["actual"]["value"].as_f64().expect("actual")
            - peaks["reference"]["value"].as_f64().expect("reference");
        assert_eq!(height, expected, "{name}");
        let distance = peaks["peak_location_periodic_distance_cells"]
            .as_array()
            .expect("distance");
        assert_eq!(distance.len(), 3, "{name}");
        for cell in distance {
            assert!(cell.as_u64().expect("cell") <= 8, "{name}");
        }
    }
    let gauge = &value["observations"]["pressure_gauge"];
    assert!(gauge["actual_pressure_lattice_mean_witness"]
        .as_f64()
        .expect("actual mean")
        .abs()
        <= 1e-9);
    assert!(gauge["measured_analytical_lattice_mean"]
        .as_f64()
        .is_some_and(f64::is_finite));
}

#[test]
fn parent_sync_runs_after_attach_even_when_cleanup_denies() {
    let directory = fixture_root().join("sync-after-cleanup");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("result.json");
    let error = publication::publish_with_fault(
        &path,
        b"payload".as_slice(),
        Some(publication::PublicationFault::CleanupDeniedAfterAttach),
    )
    .expect_err("a denied temporary unlink must fail publication");
    // The real unlink answers EACCES and the residue is only claimed because
    // the existence check confirmed it; the parent sync still ran, so the
    // report carries cleanup uncertainty and no directory-sync failure.
    assert!(error.contains("could not be removed"), "{error}");
    assert!(error.contains("existence confirmed"), "{error}");
    assert!(error.contains("PermissionDenied"), "real kernel error: {error}");
    assert!(!error.contains("directory-sync"), "sync must have succeeded: {error}");
    assert_eq!(std::fs::read(&path).expect("attachment intact"), b"payload");
    std::fs::remove_file(&path).expect("attached cleanup");
    for entry in std::fs::read_dir(&directory).expect("listing") {
        let entry = entry.expect("entry");
        if entry.file_name().to_string_lossy().starts_with('.') {
            std::fs::remove_file(entry.path()).expect("leftover temporary");
        }
    }
}

#[test]
fn rollback_remove_and_sync_faults_report_the_genuine_outcome() {
    let directory = fixture_root().join("rollback-faults");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("directory");

    // Remove fault: the rollback's unlink meets the real EACCES; the message
    // claims an incomplete attachment only because the residue is confirmed.
    let path = directory.join("remove-denied.json");
    let error = publication::publish_with_fault(
        &path,
        b"payload".as_slice(),
        Some(publication::PublicationFault::RollbackRemoveDenied),
    )
    .expect_err("rollback removal must fail at the real boundary");
    assert!(error.contains("completeness"), "{error}");
    assert!(error.contains("could not be removed"), "{error}");
    assert!(error.contains("existence confirmed"), "{error}");
    assert!(error.contains("PermissionDenied"), "real kernel error: {error}");
    assert!(path.exists(), "the confirmed incomplete attachment must still be there");
    std::fs::remove_file(&path).expect("rollback cleanup");

    // Sync fault: the removal succeeds but the rollback's own parent sync meets
    // the genuine kernel refusal, so the report keeps its durability caveat.
    let path = directory.join("sync-denied.json");
    let error = publication::publish_with_fault(
        &path,
        b"payload".as_slice(),
        Some(publication::PublicationFault::RollbackSyncDenied),
    )
    .expect_err("rollback parent sync must fail at the real boundary");
    assert!(error.contains("was removed"), "{error}");
    assert!(error.contains("durability sync of the removal failed"), "{error}");
    assert!(error.contains("may not be durable"), "{error}");
    assert!(error.contains("PermissionDenied"), "real kernel error: {error}");
    assert!(!path.exists(), "the removal itself succeeded");
}

#[test]
fn create_only_publication_refuses_a_preexisting_stale_temporary() {
    let directory = fixture_root().join("stale-temp");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("capture.json");
    let stale = directory.join(format!(".capture.json.{:x}.tmp", std::process::id()));
    std::fs::write(&stale, b"stale").expect("stale write");
    let error = publication::publish(&path, b"payload".as_slice())
        .expect_err("a stale temporary must refuse publication");
    assert!(error.contains("stale temporary"), "{error}");
    assert!(!path.exists(), "nothing may attach after the refusal");
    std::fs::remove_dir_all(&directory).expect("cleanup");
}

#[test]
fn residue_status_states_uncertainty_when_existence_cannot_be_confirmed() {
    use std::os::unix::fs::PermissionsExt;
    let directory = fixture_root().join("residue-blind");
    let _ = std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).ok();
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("artifact.json");
    assert!(publication::residue_status(&path).contains("confirmed absent"));
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o000)).expect("revoke");
    let status = publication::residue_status(&path);
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).expect("restore");
    assert!(status.contains("could not be confirmed"), "{status}");
    assert!(!status.contains("confirmed absent"), "{status}");
    std::fs::remove_dir_all(&directory).expect("cleanup");
}
