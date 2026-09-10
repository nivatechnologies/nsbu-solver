//! Public process behavior for the bounded smooth diagnostic.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_PATH: AtomicUsize = AtomicUsize::new(0);

fn invoke(arguments: &[&str]) -> Output {
    invoke_in(arguments, None)
}

fn invoke_in(arguments: &[&str], directory: Option<&std::path::Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nsbu"));
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    command
        .args(arguments)
        .output()
        .expect("start the built NSBU executable")
}

fn stdout(result: Output) -> String {
    String::from_utf8(result.stdout).expect("UTF-8 command output")
}

fn checkpoint_path(label: &str) -> PathBuf {
    let number = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "nsbu-cli-{label}-{}-{number}.bin",
        std::process::id()
    ))
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
    assert!(!text.contains("\"checkpoint\""));

    let checkpoint = invoke(&[
        "smooth",
        "--dry-run",
        "--checkpoint",
        "/tmp/ignored-checkpoint",
        "--checkpoint-after",
        "1",
    ]);
    assert!(checkpoint.status.success());
    assert!(stdout(checkpoint).contains("\"checkpoint\":{\"maximum_bytes\""));
}

#[test]
fn checkpoint_dry_run_and_resume_dry_run_do_not_skip_admission_rules() {
    let result = invoke(&[
        "smooth",
        "--dry-run",
        "--memory-cap",
        "1389032",
        "--checkpoint",
        "/tmp/ignored-checkpoint",
        "--checkpoint-after",
        "1",
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert!(stdout(result).contains("resource_limit"));

    let result = invoke(&["resume", "--dry-run", "--checkpoint", "/tmp/missing"]);
    assert_eq!(result.status.code(), Some(2));
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

#[test]
fn checkpoints_resume_both_methods_with_the_same_final_ledger() {
    for method in ["cm", "ho"] {
        let path = checkpoint_path(method);
        let path_text = path.to_str().unwrap();
        let saved = invoke(&[
            "smooth",
            "--method",
            method,
            "--checkpoint",
            path_text,
            "--checkpoint-after",
            "2",
        ]);
        assert!(saved.status.success(), "{}", stdout(saved));
        let resumed = invoke(&["resume", "--method", method, "--checkpoint", path_text]);
        assert!(resumed.status.success(), "{}", stdout(resumed));
        let uninterrupted = invoke(&["smooth", "--method", method]);
        assert!(uninterrupted.status.success());
        let resumed_text = stdout(resumed).replace("external_unverified", "internal_from_rest");
        assert_eq!(resumed_text, stdout(uninterrupted));
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn checkpoint_output_is_never_overwritten() {
    let path = checkpoint_path("existing");
    fs::write(&path, b"preserve this checkpoint").unwrap();
    let result = invoke(&[
        "smooth",
        "--checkpoint",
        path.to_str().unwrap(),
        "--checkpoint-after",
        "1",
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert!(stdout(result).contains("checkpoint_output_exists"));
    assert_eq!(fs::read(&path).unwrap(), b"preserve this checkpoint");
    fs::remove_file(path).unwrap();
}

#[test]
fn bare_output_name_uses_the_current_directory() {
    let directory = std::env::temp_dir().join(format!("nsbu-cli-bare-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let result = invoke_in(
        &[
            "smooth",
            "--checkpoint",
            "saved.bin",
            "--checkpoint-after",
            "1",
        ],
        Some(&directory),
    );
    assert!(result.status.success(), "{}", stdout(result));
    assert!(directory.join("saved.bin").is_file());
    fs::remove_file(directory.join("saved.bin")).unwrap();
    fs::remove_dir(directory).unwrap();
}

#[test]
fn resume_refuses_corrupt_over_cap_and_wrong_profile_files() {
    let missing = checkpoint_path("missing");
    let result = invoke(&["resume", "--checkpoint", missing.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1));
    assert!(stdout(result).contains("checkpoint_metadata_failed"));

    let directory = std::env::temp_dir();
    let result = invoke(&["resume", "--checkpoint", directory.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1));
    assert!(stdout(result).contains("checkpoint_not_regular_file"));

    let corrupt = checkpoint_path("corrupt");
    fs::write(&corrupt, b"not a checkpoint").unwrap();
    let result = invoke(&["resume", "--checkpoint", corrupt.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1));
    assert!(stdout(result).contains("checkpoint_invalid"));
    fs::remove_file(corrupt).unwrap();

    let path = checkpoint_path("profile");
    let saved = invoke(&[
        "smooth",
        "--checkpoint",
        path.to_str().unwrap(),
        "--checkpoint-after",
        "2",
    ]);
    assert!(saved.status.success());
    let wrong = invoke(&[
        "resume",
        "--checkpoint",
        path.to_str().unwrap(),
        "--method",
        "ho",
    ]);
    assert_eq!(wrong.status.code(), Some(1));
    let wrong_text = stdout(wrong);
    assert!(
        wrong_text.contains("checkpoint_invalid")
            || wrong_text.contains("checkpoint_profile_mismatch")
    );
    let capped = invoke(&[
        "resume",
        "--checkpoint",
        path.to_str().unwrap(),
        "--memory-cap",
        "1",
    ]);
    assert_eq!(capped.status.code(), Some(1));
    assert!(stdout(capped).contains("resource_limit"));
    fs::remove_file(path).unwrap();
}

#[test]
fn checkpoint_count_is_admitted_before_running_and_zero_saves_rest() {
    let path = checkpoint_path("count");
    for dry in [false, true] {
        let mut arguments = vec![
            "smooth",
            "--checkpoint",
            path.to_str().unwrap(),
            "--checkpoint-after",
            "5",
        ];
        if dry {
            arguments.push("--dry-run");
        }
        let result = invoke(&arguments);
        assert_eq!(result.status.code(), Some(1));
        assert!(stdout(result).contains("checkpoint_step_out_of_range"));
        assert!(!path.exists());
    }
    let saved = invoke(&[
        "smooth",
        "--checkpoint",
        path.to_str().unwrap(),
        "--checkpoint-after",
        "0",
    ]);
    assert!(saved.status.success());
    assert!(stdout(saved).contains("\"committed\":\"0\""));
    let resumed = invoke(&["resume", "--checkpoint", path.to_str().unwrap()]);
    assert!(resumed.status.success());
    assert!(stdout(resumed).contains("\"actual_elapsed_ticks\":\"256\""));
    fs::remove_file(path).unwrap();
}
