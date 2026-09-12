//! Focused process-level coverage for the exact-v2 CLI.

use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, SpectralState, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nsbu"))
        .args(args)
        .output()
        .unwrap()
}
fn out(o: Output) -> String {
    String::from_utf8(o.stdout).unwrap()
}
fn number_field(s: &str, key: &str) -> usize {
    let marker = format!("\"{key}\":\"");
    let start = s.find(&marker).expect("field present") + marker.len();
    s[start..].split('"').next().unwrap().parse().unwrap()
}
fn checkpoint() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("nsbu-v2-cli-{}-{n}.bin", std::process::id()))
}
fn small(method: &str) -> Vec<&str> {
    vec![
        "v2",
        "--method",
        method,
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
    ]
}

#[test]
fn dry_run_declares_exact_v2_admission_without_running() {
    let o = invoke(&[
        "v2",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
    ]);
    assert!(o.status.success());
    let s = out(o);
    assert!(s.contains("\"status\":\"dry_run\""));
    assert!(s.contains("\"name\":\"similarity-mms-v2\""));
    assert!(s.contains("\"grid\":4") && s.contains("\"force_grid\":4"));
    assert!(s.contains("\"actual_elapsed_ticks\":\"0\""));
    assert!(s.contains("\"clock_target_ticks\":\"8192\""));
    assert!(!s.contains("\"actual_charged_work\""));
    assert!(!s.contains("\"origin_status\":\"external_unverified\""));
}

#[test]
fn cm_and_ho_complete_two_exact_steps() {
    for method in ["cm", "ho"] {
        let o = invoke(&small(method));
        assert!(o.status.success(), "{method}: {}", out(o));
        let s = out(o);
        assert!(s.contains("\"status\":\"completed\""));
        assert!(s.contains("\"method\":\""));
        assert!(s.contains("\"actual_elapsed_ticks\":\"256\""));
        assert!(s.contains("\"started\":\"2\",\"committed\":\"2\""));
        assert!(s.contains("\"accepted_pde_windows\":0"));
    }
}

#[test]
fn checkpoint_resume_preserves_charges_and_profile() {
    for method in ["cm", "ho"] {
        let path = checkpoint();
        let p = path.to_str().unwrap();
        let saved = invoke(&[
            "v2",
            "--method",
            method,
            "--endpoint-ticks",
            "256",
            "--attempts",
            "4",
            "--checkpoint",
            p,
            "--checkpoint-after",
            "1",
        ]);
        assert!(saved.status.success(), "{}", out(saved));
        let resumed = invoke(&[
            "resume-v2",
            "--method",
            method,
            "--endpoint-ticks",
            "256",
            "--attempts",
            "4",
            "--checkpoint",
            p,
        ]);
        assert!(resumed.status.success(), "{}", out(resumed));
        let full = invoke(&[
            "v2",
            "--method",
            method,
            "--endpoint-ticks",
            "256",
            "--attempts",
            "4",
        ]);
        assert!(full.status.success());
        let resumed_s = out(resumed).replace("external_unverified", "internal_from_rest");
        assert_eq!(resumed_s, out(full));
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn checkpoint_refuses_foreign_profile_and_does_not_overwrite() {
    let path = checkpoint();
    let p = path.to_str().unwrap();
    let saved = invoke(&[
        "v2",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
        "--checkpoint",
        p,
        "--checkpoint-after",
        "1",
    ]);
    assert!(saved.status.success());
    let before = fs::read(&path).unwrap();
    let wrong = invoke(&[
        "resume-v2",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
        "--force-grid",
        "8",
        "--checkpoint",
        p,
    ]);
    assert_eq!(wrong.status.code(), Some(1));
    let wrong_s = out(wrong);
    assert!(wrong_s.contains("checkpoint_invalid") || wrong_s.contains("profile_mismatch"));
    let overwrite = invoke(&[
        "v2",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
        "--checkpoint",
        p,
        "--checkpoint-after",
        "1",
    ]);
    assert_eq!(overwrite.status.code(), Some(1));
    assert!(out(overwrite).contains("checkpoint_output_exists"));
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_file(path).unwrap();
}

#[test]
fn invalid_v2_flags_and_caps_are_nonzero_structured_refusals() {
    for args in [
        vec!["v2", "--step-ticks", "3"],
        vec!["v2", "--tick-exponent", "-19"],
        vec!["v2", "--grid", "6"],
        vec!["v2", "--workers", "999999"],
        vec!["v2", "--memory-cap", "1"],
    ] {
        let o = invoke(&args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert!(out(o).contains("\"status\":\"refused\""));
    }
    let dry = invoke(&[
        "v2",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
    ]);
    let cap = number_field(&out(dry), "total_bytes");
    let cap_text = (cap - 1).to_string();
    let refused = invoke(&[
        "v2",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
        "--memory-cap",
        &cap_text,
    ]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(out(refused).contains("resource_limit"));
}

#[test]
fn terminal_failure_returns_nonzero_and_reports_reason() {
    let o = invoke(&["v2", "--endpoint-ticks", "256", "--attempts", "1"]);
    assert_eq!(o.status.code(), Some(1));
    let s = out(o);
    assert!(s.contains("\"status\":\"failed\"") || s.contains("\"status\":\"refused\""));
}

#[test]
fn checkpoint_count_and_total_buffer_budget_are_preflighted() {
    let path = checkpoint();
    let name = path.to_str().unwrap();
    let excessive = invoke(&[
        "v2",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "2",
        "--checkpoint",
        name,
        "--checkpoint-after",
        "3",
    ]);
    assert_eq!(excessive.status.code(), Some(1));
    assert!(!path.exists());
    let dry = invoke(&[
        "v2",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "2",
        "--checkpoint",
        name,
        "--checkpoint-after",
        "0",
    ]);
    assert!(dry.status.success());
    let report = out(dry);
    let cap =
        number_field(&report, "total_bytes") + number_field(&report, "checkpoint_buffer_bytes") - 1;
    let capped = invoke(&[
        "v2",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "2",
        "--checkpoint",
        name,
        "--checkpoint-after",
        "0",
        "--memory-cap",
        &cap.to_string(),
    ]);
    assert_eq!(capped.status.code(), Some(1));
    assert!(!path.exists());
    for command in ["v2", "resume-v2"] {
        assert!(invoke(&[command, "--help"]).status.success());
    }
}

#[test]
fn cached_cm_and_ho_preserve_direct_terminal_clock_and_report_actual_ledgers() {
    for method in ["cm", "ho"] {
        let direct = invoke(&small(method));
        assert!(direct.status.success(), "{method}: {}", out(direct));
        let cached = invoke(&[
            "v2",
            "--cache-force",
            "--method",
            method,
            "--endpoint-ticks",
            "256",
            "--attempts",
            "4",
        ]);
        assert!(cached.status.success(), "{method}: {}", out(cached));
        let direct_s = out(direct);
        let cached_s = out(cached);
        for field in [
            "actual_elapsed_ticks",
            "remaining_ticks",
            "started",
            "committed",
            "rejected",
            "refused",
        ] {
            assert_eq!(
                number_field(&direct_s, field),
                number_field(&cached_s, field)
            );
        }
        assert!(cached_s.contains("\"kind\":\"attempt_local_original_force_cache\""));
        assert!(cached_s.contains("\"archive_support\":\"unsupported\""));
        assert!(cached_s.contains("\"cache_ledger_scope\":\"current_attempt_only\""));
        assert!(cached_s.contains("\"actual_charged_work\""));
        assert!(cached_s.contains("\"current_attempt_cache_work\""));
        assert!(number_field(&cached_s, "calls") > 0);
        assert!(number_field(&cached_s, "provider_evaluations") > 0);
        assert!(number_field(&cached_s, "coefficient_words_copied") > 0);
    }
}

#[test]
fn cached_admission_obeys_cap_and_refuses_all_archive_paths_before_io() {
    let dry = invoke(&[
        "v2",
        "--cache-force",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
    ]);
    assert!(dry.status.success(), "{}", out(dry));
    let report = out(dry);
    assert!(report.contains("\"integration_force_policy\""));
    let cap = number_field(&report, "total_bytes");
    let capped = invoke(&[
        "v2",
        "--cache-force",
        "--dry-run",
        "--endpoint-ticks",
        "256",
        "--attempts",
        "4",
        "--memory-cap",
        &(cap - 1).to_string(),
    ]);
    assert_eq!(capped.status.code(), Some(1));
    assert!(out(capped).contains("resource_limit"));

    let path = checkpoint();
    let missing = path.to_str().unwrap();
    for args in [
        vec![
            "v2",
            "--cache-force",
            "--checkpoint",
            missing,
            "--checkpoint-after",
            "0",
        ],
        vec!["v2", "--cache-force", "--checkpoint", missing],
        vec!["v2", "--cache-force", "--checkpoint-after", "0"],
        vec!["resume-v2", "--cache-force", "--checkpoint", missing],
        vec!["resume-v2", "--cache-force"],
    ] {
        let refused = invoke(&args);
        assert_eq!(refused.status.code(), Some(1), "{args:?}");
        assert!(out(refused).contains("cached_force_checkpoint_unsupported"));
        assert!(!path.exists());
    }
}

fn run_settings(method: Method) -> Settings {
    Settings {
        domain: Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method,
            limits: RunLimits {
                endpoint: 256,
                step_ticks: 128,
                maximum_attempts: 4,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn assert_state_bits(left: &SpectralState, right: &SpectralState) {
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.epoch(), right.epoch());
    assert_eq!(left.accepted_steps(), right.accepted_steps());
    for axis in 0..3 {
        for (a, b) in left
            .component(axis)
            .unwrap()
            .iter()
            .zip(right.component(axis).unwrap())
        {
            assert_eq!(
                (a.re.to_bits(), a.im.to_bits()),
                (b.re.to_bits(), b.im.to_bits())
            );
        }
    }
}

#[test]
fn cached_cm_and_ho_runs_preserve_direct_terminal_state_bits() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let settings = run_settings(method);
        let mut direct =
            Run::from_rest(Plan::from_rest(settings, 64 * 1024 * 1024).unwrap()).unwrap();
        let mut cached =
            Run::from_rest(Plan::from_rest_cached(settings, 64 * 1024 * 1024).unwrap()).unwrap();
        while direct.history().controller().stopped().is_none() {
            direct.step().unwrap();
            cached.step().unwrap();
        }
        assert_state_bits(direct.state(), cached.state());
        assert_eq!(
            direct.history().controller().clock(),
            cached.history().controller().clock()
        );
        assert_eq!(
            direct.history().controller().attempted(),
            cached.history().controller().attempted()
        );
        assert!(cached.cache_work().unwrap().calls > 0);
    }
}
