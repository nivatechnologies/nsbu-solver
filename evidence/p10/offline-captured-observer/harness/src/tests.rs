use crate::fixtures::{self, Fixture};
use crate::observer::OfflineObserver;
use crate::{execute, Mode};
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_solver::{
    diagnostics::balances::BalanceSample,
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace, indicator::Tolerances, method::Method, rhs::SpectralRhs,
        transaction::CandidateState,
    },
    spectral::FftBackend,
    Complex64, SolverError,
};
use serde_json::Value;
use std::path::PathBuf;

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/offline-observer-tests")
}

fn matched_fixture(name: &str) -> Fixture {
    let layout = Layout::new(N).expect("fixture layout");
    let coefficients = fixtures::synthetic_state(layout);
    fixtures::write_fixture(
        &fixture_root(),
        name,
        &coefficients,
        "MATCHED_SPATIAL",
        "cox-matthews",
        [384; 3],
        ELAPSED,
        TARGET,
        N,
    )
    .expect("fixture write")
}

fn parse(output: Result<String, String>) -> Value {
    serde_json::from_str(&output.expect("command output")).expect("json output")
}

#[test]
fn run_binds_output_and_stays_diagnostic_only() {
    let fixture = matched_fixture("bind");
    let value = parse(execute(
        Mode::Run,
        &fixture.manifest_path,
        16,
        1,
        1 << 40,
        "owned-radix",
    ));
    assert_eq!(value["schema"], "p10-offline-captured-observer-v1");
    assert_eq!(value["scope"], "balance-diagnostic-only");
    assert_eq!(value["qualification"], false);
    assert_eq!(value["accepted_windows"], 0);
    assert_eq!(value["clock"]["exponent"], -20);
    assert_eq!(value["clock"]["elapsed"].as_u64(), Some(ELAPSED as u64));
    assert_eq!(value["clock"]["target"].as_u64(), Some(TARGET as u64));
    assert_eq!(
        value["clock"]["remaining"].as_u64(),
        Some((TARGET - ELAPSED) as u64)
    );
    assert_eq!(value["plan_sha256"], fixture_plan_sha(&fixture));
    assert_eq!(
        value["snapshot"]["coefficient_sha256"],
        fixture.coefficient_sha256
    );
    assert_eq!(value["snapshot"]["file_sha256"], fixture.file_sha256);
    assert_eq!(value["requested"]["workers"], 1);
    assert_eq!(value["requested"]["force_sample_dimensions"][0], 16);
    assert_eq!(value["finite"], true);
    let balance = &value["balance"];
    for key in [
        "l2",
        "h1",
        "vorticity_l2",
        "divergence_l2",
        "energy",
        "enstrophy",
        "energy_dissipation",
        "forcing_work",
        "stretching",
        "enstrophy_dissipation",
        "vorticity_forcing",
    ] {
        assert!(balance[key].as_f64().is_some_and(f64::is_finite), "{key}");
    }
}


#[test]
fn tampered_provenance_is_refused() {
    let fixture = matched_fixture("tamper");
    let file_tampered = fixtures::tamper(&fixture.manifest_path, "file-tampered", "file_sha256")
        .expect("tamper file hash");
    let error = execute(Mode::Run, &file_tampered, 16, 1, 1 << 40, "owned-radix")
        .expect_err("tampered file hash must refuse");
    assert!(error.contains("hash"), "{error}");
    let coefficient_tampered = fixtures::tamper(
        &fixture.manifest_path,
        "coefficient-tampered",
        "coefficient_sha256",
    )
    .expect("tamper coefficient hash");
    let error = execute(
        Mode::Run,
        &coefficient_tampered,
        16,
        1,
        1 << 40,
        "owned-radix",
    )
    .expect_err("tampered coefficient hash must refuse");
    assert!(
        error.contains("hash") || error.contains("SHA-256"),
        "{error}"
    );
}

#[test]
fn unsupported_profiles_and_endpoint_clocks_are_refused() {
    let layout = Layout::new(N).expect("fixture layout");
    let coefficients = fixtures::synthetic_state(layout);
    let method = fixtures::write_fixture(
        &fixture_root(),
        "method-diagnostic",
        &coefficients,
        "METHOD_DIAGNOSTIC",
        "hochbruck-ostermann",
        [384; 3],
        ELAPSED,
        TARGET,
        N,
    )
    .expect("fixture write");
    let error = execute(
        Mode::Run,
        &method.manifest_path,
        16,
        1,
        1 << 40,
        "owned-radix",
    )
    .expect_err("method-diagnostic profiles are unsupported");
    assert!(error.contains("unsupported"), "{error}");

    let endpoint = fixtures::write_fixture(
        &fixture_root(),
        "endpoint-clock",
        &coefficients,
        "MATCHED_SPATIAL",
        "cox-matthews",
        [384; 3],
        TARGET,
        TARGET,
        N,
    )
    .expect("fixture write");
    let error = execute(
        Mode::Run,
        &endpoint.manifest_path,
        16,
        1,
        1 << 40,
        "owned-radix",
    )
    .expect_err("zero-remaining endpoint clocks are unsupported");
    assert!(error.contains("zero remaining"), "{error}");
}

/// The v3 launch-plan target is 8192 and its endpoint comparison runs at elapsed
/// 4096, which leaves remaining 4096 > 0: permitted by the exact-clock admission and
/// explicitly not a zero-remaining endpoint.
#[test]
fn v3_style_elapsed_4096_of_target_8192_is_permitted_with_remaining_4096() {
    let layout = Layout::new(N).expect("fixture layout");
    let coefficients = fixtures::synthetic_state(layout);
    let mid = fixtures::write_fixture(
        &fixture_root(),
        "v3-endpoint-4096",
        &coefficients,
        "MATCHED_SPATIAL",
        "cox-matthews",
        [384; 3],
        4096,
        TARGET,
        N,
    )
    .expect("fixture write");
    let value = parse(execute(
        Mode::Preflight,
        &mid.manifest_path,
        16,
        1,
        1 << 40,
        "owned-radix",
    ));
    assert_eq!(value["clock"]["target"].as_u64(), Some(TARGET as u64));
    assert_eq!(value["clock"]["elapsed"].as_u64(), Some(4096));
    assert_eq!(value["clock"]["remaining"].as_u64(), Some(4096));
    assert_eq!(value["fits"], true);
}


/// One loose-tolerance accepted transactional step from rest on an owned-radix RHS:
/// an actual committed CM state for observer comparison. The observer backend is
/// chosen separately; integration only supplies the observed state.
fn committed_state(dimensions: [usize; 3]) -> (Domain, SpectralState) {
    let domain = Domain::new(dimensions, [1.0; 3], 1.0).expect("domain");
    let side = dimensions[0].max(16);
    let samples = Layout::new([side; 3]).expect("samples");
    let backend = FftBackend::OwnedRadix;
    let catalog = nsbu_solver::spectral::FftCatalog::new(backend, 0).expect("catalog");
    let force_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, 2, backend).unwrap();
    let rhs_bytes =
        SpectralRhs::<ParallelReducedV2Force>::reservation(domain, force_limits).unwrap();
    let attempt_bytes =
        AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews).unwrap();
    let resources = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: rhs_bytes,
            diagnostics: attempt_bytes,
            overhead: 64 * 1024,
        },
        1 << 40,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-20, TARGET).unwrap();
    let mut state = SpectralState::from_rest(resources, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(resources, clock, Epoch(0)).unwrap();
    let mut attempts = AttemptWorkspace::new_with_method(resources, Method::CoxMatthews).unwrap();
    let force = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        2,
        &catalog,
        force_limits.storage_bytes,
    )
    .unwrap();
    let mut rhs = SpectralRhs::new_with_catalog(domain, force, 0.3, &catalog, rhs_bytes).unwrap();
    let result = attempts
        .try_advance(
            &state,
            &mut candidate,
            8,
            Tolerances {
                absolute: [1.0, 1.0],
                relative: [1.0, 1.0],
            },
            &mut rhs,
        )
        .expect("attempt");
    let accepted = result.accepted.expect("loose-tolerance acceptance");
    nsbu_solver::integrators::transaction::commit_candidate(
        resources,
        &mut state,
        &mut candidate,
        accepted,
    )
    .expect("commit");
    assert_eq!(state.clock().elapsed(), 8);
    (domain, state)
}

fn committed_n8_state() -> (Domain, Layout, SpectralState) {
    let observer_samples = Layout::new([32; 3]).expect("observer samples");
    let (domain, state) = committed_state(N);
    (domain, observer_samples, state)
}

#[test]
fn matches_the_existing_observer_on_an_actual_state_and_preserves_input() {
    let (domain, observer_samples, state) = committed_n8_state();
    let backend = FftBackend::OwnedRadix;
    let catalog = nsbu_solver::spectral::FftCatalog::new(backend, 0).expect("catalog");
    let observer_bytes =
        crate::control::ReducedObserver::preflight(domain, observer_samples, 2, backend).unwrap();

    let before = state_hash(&state);
    let mut control =
        crate::control::ReducedObserver::new(domain, observer_samples, 2, &catalog, observer_bytes)
            .unwrap();
    let control_balance: BalanceSample = control.sample(&state).expect("control sample").balance;

    let ledger = OfflineObserver::preflight(domain, observer_samples, 2, backend, 0).unwrap();
    assert_eq!(
        OfflineObserver::new(
            domain,
            observer_samples,
            2,
            &catalog,
            0,
            &ledger,
            ledger.total_bytes - 1,
        )
        .err(),
        Some(SolverError::ResourceLimit)
    );
    let mut offline = OfflineObserver::new(
        domain,
        observer_samples,
        2,
        &catalog,
        0,
        &ledger,
        ledger.total_bytes,
    )
    .expect("offline observer");
    let borrowed = [
        state.component(0).unwrap(),
        state.component(1).unwrap(),
        state.component(2).unwrap(),
    ];
    let offline_balance = offline
        .observe(state.clock(), borrowed)
        .expect("offline sample");
    assert_eq!(control_balance, offline_balance);
    assert_eq!(state_hash(&state), before);

    let short = vec![Complex64::new(0.0, 0.0); domain.layout().half_len() - 1];
    assert_eq!(
        offline
            .observe(state.clock(), [&short, &short, &short])
            .err(),
        Some(SolverError::InvalidPayload)
    );
    assert_eq!(state_hash(&state), before);
}


#[test]
fn repeated_observations_reuse_one_observer_matching_the_control_lifecycle() {
    let (domain, samples, state) = committed_n8_state();
    let backend = FftBackend::OwnedRadix;
    let catalog = nsbu_solver::spectral::FftCatalog::new(backend, 0).expect("catalog");
    let observer_bytes =
        crate::control::ReducedObserver::preflight(domain, samples, 2, backend).unwrap();
    let mut control =
        crate::control::ReducedObserver::new(domain, samples, 2, &catalog, observer_bytes).unwrap();
    let ledger = OfflineObserver::preflight(domain, samples, 2, backend, 0).unwrap();
    let mut offline =
        OfflineObserver::new(domain, samples, 2, &catalog, 0, &ledger, ledger.total_bytes)
            .expect("offline observer");
    let before = state_hash(&state);
    let borrowed = [
        state.component(0).unwrap(),
        state.component(1).unwrap(),
        state.component(2).unwrap(),
    ];
    for round in 0..3 {
        let expected = control.sample(&state).expect("control sample").balance;
        let observed = offline
            .observe(state.clock(), borrowed)
            .expect("reused observer stays inside the force contract");
        assert_eq!(expected, observed, "round {round}");
    }
    assert_eq!(state_hash(&state), before);
}

fn fixture_plan_sha(fixture: &Fixture) -> String {
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(fixture.manifest_path.with_extension("json")).unwrap(),
    )
    .unwrap();
    manifest["plan_sha256"]
        .as_str()
        .expect("plan sha")
        .to_owned()
}

fn state_hash(state: &SpectralState) -> String {
    fixtures::coefficient_hash([
        state.component(0).unwrap(),
        state.component(1).unwrap(),
        state.component(2).unwrap(),
    ])
}

#[test]
fn frozen_case_admission_admits_baseline_and_refuses_single_case_hash_change() {
    let fixture = matched_fixture("case-admission");
    let manifest =
        crate::decode::read_manifest(&fixture.manifest_path).expect("decode baseline manifest");
    crate::admitted_profile(&manifest)
        .expect("the frozen similarity-mms-v2 case baseline is admitted");

    let bytes = std::fs::read(&fixture.manifest_path).expect("read baseline manifest");
    let mut value: Value = serde_json::from_slice(&bytes).expect("baseline manifest json");
    let current = value["evolution"]["case_sha256"]
        .as_str()
        .expect("case hash");
    let replaced = if current.starts_with('0') { 'f' } else { '0' };
    value["evolution"]["case_sha256"] = Value::String(format!("{replaced}{}", &current[1..]));
    let renamed = fixture.manifest_path.with_file_name("case-tampered.json");
    std::fs::write(
        &renamed,
        serde_json::to_vec_pretty(&value).expect("serialize"),
    )
    .expect("write tampered manifest");
    let tampered = crate::decode::read_manifest(&renamed).expect("tampered manifest still decodes");
    let error = crate::admitted_profile(&tampered)
        .expect_err("a single changed case-hash character must refuse");
    assert!(error.contains("case"), "{error}");
}

#[path = "avx_tests.rs"]
mod avx_tests;

#[path = "resource_tests.rs"]
mod resource_tests;
