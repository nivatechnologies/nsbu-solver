//! Public process behavior for the commands actually available in P01.
use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nsbu"))
        .args(arguments)
        .output()
        .expect("start the built NSBU executable")
}

#[test]
fn help_explains_the_current_capability() {
    for arguments in [&[][..], &["--help"][..], &["-h"][..]] {
        let result = invoke(arguments);
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        let text = String::from_utf8(result.stdout).expect("UTF-8 help");
        assert!(text.contains("NSBU Solver"));
        assert!(text.contains("Numerical simulation commands are not implemented yet."));
    }
}

#[test]
fn version_identifies_the_development_package() {
    for flag in ["--version", "-V"] {
        let result = invoke(&[flag]);
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        assert_eq!(result.stdout, b"NSBU Solver 0.1.0-alpha.0\n");
    }
}

#[test]
fn unsupported_and_extra_arguments_are_refused() {
    for arguments in [
        &["run"][..],
        &["--help", "extra"][..],
        &["--version", "extra"][..],
        &["--invalid"][..],
        &[""][..],
    ] {
        let result = invoke(arguments);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8(result.stderr)
            .expect("UTF-8 error")
            .contains("Unsupported arguments"));
    }
}
