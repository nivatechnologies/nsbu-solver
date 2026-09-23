//! Both sealed-review counterexamples, PROCESS-DENIED through the REAL
//! tiny-RunOwners `execute` dispatch in the exact profile process that the
//! review exploited.
//!
//! Counterexample #1 (reviewed N256 profile): a complete channel opt-in at
//! CLOCK=1 with descriptor 2147483647 SILENTLY COMPLETED all 48 attempts to
//! the terminal with no armed receipt and no decision.  The repaired
//! admission must refuse the identical request BEFORE any attempt runs.
//!
//! Counterexample #2 (N512 capture profile): the review compiled this same
//! harness with `n512-m512-piecewise-cadv33` (a `capture_offline` binary),
//! drove the same dispatch with a COMPLETE, otherwise-valid opt-in bound to
//! that compiled profile, and the N512 binary ACCEPTED it and completed its
//! whole 48-attempt schedule.  Channel admission is now restricted to the
//! reviewed N256/M512 feature; the byte-for-byte same request must refuse in
//! the N512 process before a single attempt runs.

use super::{barrier, config, schedule};
use crate::run_owners_test::{barrier_now, identity_component, sha256_hex, temp_root, tiny_run};
use std::fs;

fn refused_before_any_attempt(label: &str, clock: &str, expected: &str) {
    let _guard = barrier::env_guard();
    let root = temp_root(&format!("deny-{label}"));
    let output = root.join("out");
    let barrier_dir = root.join("barrier");
    fs::create_dir(&output).unwrap();
    fs::create_dir(&barrier_dir).unwrap();
    let identity = config::identity();
    crate::step_artifact::publish_rest(&output, &identity).unwrap();
    let rest_hash = sha256_hex(&output.join("rest.json"));
    fs::remove_file(output.join("rest.json")).unwrap();
    let pairs = [
        (barrier::DIR_ENV, barrier_dir.to_string_lossy().into_owned()),
        (barrier::NONCE_ENV, "a".repeat(64)),
        (barrier::SECRET_ENV, "b".repeat(64)),
        (barrier::CLOCK_ENV, clock.to_string()),
        (barrier::DEADLINE_ENV, (barrier_now() + 60).to_string()),
        (barrier::CHANNEL_FD_ENV, "2147483647".into()),
        (barrier::SOURCE_ENV, identity_component(&identity, "source")),
        (
            barrier::PROFILE_ENV,
            identity_component(&identity, "profile"),
        ),
        (barrier::REST_ENV, rest_hash),
    ];
    for (key, value) in &pairs {
        std::env::set_var(key, value);
    }
    let parsed = barrier::Barrier::from_env_with_identity(&identity);
    assert!(
        matches!(&parsed, Err(error) if error.to_string() == expected),
        "channel opt-in must refuse with {expected}, got {parsed:?}"
    );
    let mut run = tiny_run();
    let result = run.execute(&output);
    for (key, _) in &pairs {
        std::env::remove_var(key);
    }
    let error = result.expect_err("channel opt-in must refuse, never complete");
    assert_eq!(error.to_string(), expected);
    // NOTHING ran: zero attempted attempts, no step bundle, no terminal
    // marker, no armed receipt — the review's 48-attempt bypass is closed.
    assert_eq!(run.frontiers.attempted, 0, "no attempt may be attempted");
    assert_eq!(run.frontiers.durable_attempt, 0);
    assert_eq!(run.frontiers.durable_clock, 0);
    assert!(!output
        .join(format!("step-001-clock-{:04}", schedule::step(0).unwrap()))
        .exists());
    assert!(!output.join("endpoint-capture-complete.json").exists());
    assert!(!barrier_dir.join(barrier::ARMED_RECEIPT).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unreachable_channel_clock_is_refused_before_any_attempt() {
    refused_before_any_attempt(
        "clock1",
        "1",
        if cfg!(feature = "n256-m512-piecewise-cadv33") {
            "barrier:barrier_channel_clock_unreviewed"
        } else {
            "barrier:barrier_channel_unsupported_profile"
        },
    );
}

/// Compiled ONLY in the N512 parallel-capture test process — the exact
/// binary that accepted channel mode before this repair.
#[cfg(feature = "n512-m512-parallel-capture")]
#[test]
fn full_channel_opt_in_is_denied_in_the_n512_capture_profile_process() {
    assert_eq!(
        config::PROFILE,
        "n512-m512-h64to2048-h128to4096-cadv33-w3-pfft1ed6995"
    );
    refused_before_any_attempt(
        "n512",
        &schedule::step(0).unwrap().to_string(),
        "barrier:barrier_channel_unsupported_profile",
    );
}
