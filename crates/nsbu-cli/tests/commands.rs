//! Public process behavior for the bounded smooth diagnostic.
use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nsbu"))
        .args(arguments)
        .output()
        .expect("start the built NSBU executable")
}

fn stdout(result: Output) -> String {
    String::from_utf8(result.stdout).expect("UTF-8 command output")
}

#[test]
fn help_and_version_describe_the_available_command() {
    for arguments in [&[][..], &["--help"][..], &["-h"][..]] {
        let result = invoke(arguments);
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        let text = stdout(result);
        assert!(text.contains("nsbu smooth [OPTIONS]"));
        assert!(text.contains("viscosity 0.3, advective guard 1.0"));
        assert!(text.contains("not PDE-qualified"));
    }
    for flag in ["--version", "-V"] {
        let result = invoke(&[flag]);
        assert!(result.status.success());
        assert_eq!(result.stdout, b"NSBU Solver 0.1.0-alpha.0\n");
    }
}

#[test]
fn invalid_syntax_has_usage_exit_code() {
    for arguments in [
        &["run"][..],
        &["smooth", "--method", "bad"][..],
        &["smooth", "--dry-run", "--dry-run"][..],
        &["smooth", "--grid"][..],
    ] {
        let result = invoke(arguments);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8(result.stderr)
            .expect("UTF-8 error")
            .contains("Unsupported arguments"));
    }
}

#[test]
fn dry_run_reports_an_allocation_free_bounded_plan() {
    let result = invoke(&["smooth", "--dry-run", "--method", "ho"]);
    assert!(result.status.success());
    let text = stdout(result);
    assert!(text.contains("\"status\":\"dry_run\""));
    assert!(text.contains("\"method\":\"ho\""));
    assert!(text.contains("\"actual_elapsed_ticks\":\"0\""));
    assert!(text.contains("\"rhs_calls\":\"60\""));
    assert!(text.contains("\"lengths\":[1.0,1.0,1.0]"));
    assert!(text.contains("\"advective_guard\":1.0"));
    assert!(text.contains("\"bounded_observer\""));
    assert!(text.contains("\"origin_status\":\"internal_from_rest\""));
    assert!(text.contains("\"qualification_status\":\"unqualified\""));
    assert!(text.contains("\"pde_qualified\":false"));
    assert!(!text.contains("checkpoint"));
}

#[test]
fn invalid_limits_and_resource_cap_are_structured_refusals() {
    for arguments in [
        &["smooth", "--step", "3"][..],
        &["smooth", "--domain", "6"][..],
        &["smooth", "--memory-cap", "1"][..],
        &["smooth", "--attempt-cap", "3"][..],
    ] {
        let result = invoke(arguments);
        assert_eq!(result.status.code(), Some(1));
        let text = stdout(result);
        assert!(text.contains("\"status\":\"refused\""));
        assert!(text.contains("\"pde_qualified\":false"));
    }
}

#[test]
fn cm_and_ho_complete_independent_from_rest_runs() {
    for (method, calls) in [("cm", "48"), ("ho", "60")] {
        let result = invoke(&["smooth", "--method", method]);
        assert!(result.status.success());
        let text = stdout(result);
        assert!(text.contains("\"status\":\"completed\""));
        assert!(text.contains("\"stop_reason\":\"endpoint_reached\""));
        assert!(text.contains("\"actual_elapsed_ticks\":\"256\""));
        assert!(text.contains("\"started\":\"4\",\"committed\":\"4\""));
        assert!(text.contains(&format!("\"rhs_calls\":\"{calls}\"")));
        assert!(text.contains("\"last_accepted_sample\":{\"sampled\":true"));
        assert!(text.contains("\"observer_charges\":{\"samples\":\"4\""));
    }
}
