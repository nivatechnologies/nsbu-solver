//! Focused process-level coverage for the exact-v2 CLI.

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
