//! Process-level contract for the fixed exact-v2 diagnostic command.
use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nsbu"))
        .args(arguments)
        .output()
        .expect("start nsbu")
}
fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}
fn maximum_numeric_field(text: &str, key: &str) -> f64 {
    let marker = format!("\"{key}\":");
    text.match_indices(&marker)
        .map(|(start, _)| {
            text[start + marker.len()..]
                .split([',', '}'])
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap()
        })
        .fold(0.0, f64::max)
}
fn string_field<'a>(text: &'a str, key: &str) -> &'a str {
    let marker = format!("\"{key}\":\"");
    let start = text.find(&marker).unwrap() + marker.len();
    text[start..].split('"').next().unwrap()
}

#[test]
fn help_and_malformed_arguments_preserve_the_fixed_command_boundary() {
    let global = invoke(&["--help"]);
    assert!(global.status.success());
    assert!(stdout(&global).contains("nsbu diagnose-v2 [--dry-run]"));

    for arguments in [["diagnose-v2", "--help"], ["diagnose-v2", "-h"]] {
        let output = invoke(&arguments);
        assert!(output.status.success());
        let text = stdout(&output);
        assert!(text.contains("N=4/8/12"));
        assert!(text.contains("UnqualifiedDiagnostic"));
    }
    for arguments in [
        &["diagnose-v2", "--bad"][..],
        &["diagnose-v2", "--dry-run", "--dry-run"][..],
        &["diagnose-v2", "--grid", "8"][..],
    ] {
        let output = invoke(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("Unsupported diagnose-v2 arguments"));
    }
}

#[test]
fn dry_run_admits_the_complete_profile_before_driver_allocation() {
    let output = invoke(&["diagnose-v2", "--dry-run"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = stdout(&output);
    assert!(text.contains("\"status\":\"admitted\""));
    assert!(text.contains("\"scientific_status\":\"UnqualifiedDiagnostic\""));
    assert!(text.contains("\"event_attempts\":\"7\""));
    assert!(text.contains("\"accepted_events\":\"3\""));
    assert!(text.contains("\"residual_events\":\"4\""));
    assert!(text.contains("\"force_resolution\""));
    assert!(text.contains("\"reference_precision\""));
    assert!(text.contains("\"type\":\"work_preflight\""));
    assert!(text.contains("\"residual_provider_work\":\""));
    assert!(text.contains(&format!(
        "\"case_sha256\":\"{}\"",
        nsbu_benchmarks::CASE_SHA256
    )));
    for key in ["family_identity", "probe_identity"] {
        let identity = string_field(&text, key);
        assert_eq!(identity.len(), 64);
        assert!(identity.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    assert!(text.contains("\"branch_profiles\":[[4,16,\"cm\"]"));
    assert!(text.contains("\"force_grid\":12,\"workers\":0"));
    assert!(text.contains("\"accepted_clocks\":[\"0\",\"64\",\"128\"]"));
    assert!(text.contains("\"physical_samples\":[12, 12, 12]"));
    assert!(text.contains("\"pressure_samples\":[24, 24, 24]"));
    assert!(text.contains("\"status\":\"dry_run\""));
    assert!(text.contains("\"driver_allocated\":false"));
    assert!(!text.contains("\"type\":\"diagnostic_summary\""));
}

#[test]
fn actual_command_reports_all_raw_summaries_without_qualifying_a_window() {
    let output = invoke(&["diagnose-v2"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(output.stderr.is_empty());
    let text = stdout(&output);
    println!("{text}");
    assert_eq!(text.matches("\"type\":\"diagnostic_summary\"").count(), 7);
    assert_eq!(text.matches("\"path\":\"accepted\"").count(), 3);
    assert_eq!(text.matches("\"path\":\"residual\"").count(), 4);
    assert_eq!(text.matches("\"bitwise_equal_nodes\":6").count(), 3);
    assert!(text.contains("\"clock_ticks\":\"127\""));
    assert!(text.contains("\"physical_pair_max_rms\":{\"velocity\":"));
    assert!(text.contains("\"pressure_pair_max_rms\":{\"pressure\":"));
    assert!(text.contains("\"reference_branch_max_rms\":{\"velocity\":"));
    let residual = maximum_numeric_field(&text, "residual_max_l2");
    assert!(residual.is_finite() && residual > 0.05 && residual < 0.052);
    assert!(text.contains("\"status\":\"completed\""));
    assert!(text.contains("\"events\":7"));
    assert!(text.contains("\"qualified_windows\":0"));
    assert!(text.contains("\"pde_qualified\":false"));
}
