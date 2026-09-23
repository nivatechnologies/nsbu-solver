//! Opt-in inherited decision-channel tests (acceptances and durable
//! binding): the real durable-commit gate is driven with an injected
//! transport, proving arming from the ACTUAL durable state, durable
//! REST/armed identity binding, and one-shot consumption.

use super::durable::now_epoch;
use super::fixtures::*;
use super::*;
use std::fs;

#[test]
fn channel_release_arms_from_durable_state_completes_one_shot() {
    let s = armed_setup("chan-rel");
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &s.secret, "release", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
        )))],
        ..FakeTransport::default()
    };
    barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap();
    assert_eq!(barrier.phase, BarrierPhase::Completed);
    assert_eq!(fake.receive_calls, 1);
    assert_eq!(fake.closed, 1);
    // The REAL durable armed receipt was published before the wait.
    let receipt = fs::read_to_string(s.dir.join(ARMED_RECEIPT)).unwrap();
    assert!(receipt.contains("\"clock\": 32") && receipt.contains("\"attempt\": 1"));
    assert!(receipt.contains(&format!("\"state_sha256\": \"{}\"", s.state)));
    // Exactly one accepted decision: a later commit never re-arms or consumes.
    let before = fake.receive_calls;
    barrier
        .after_commit_with_transport(&s.output, 2, s.clock + 1, &mut fake)
        .unwrap();
    barrier
        .after_commit_with_transport(&s.output, 3, s.clock, &mut fake)
        .unwrap();
    assert_eq!(fake.receive_calls, before);
    assert_eq!(
        fs::read_dir(&s.dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("barrier-armed"))
            .count(),
        1
    );
    cleanup(&s);
}

#[test]
fn channel_abort_disarms_and_stays_disarmed() {
    let s = armed_setup("chan-abort");
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &s.secret, "abort", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
        )))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_aborted_by_supervisor");
    assert_eq!(barrier.phase, BarrierPhase::Disarmed);
    assert_eq!(
        fs::read_dir(&s.dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("barrier-armed"))
            .count(),
        1
    );
    // Disarming is terminal: the channel is never re-read and a later commit
    // fails closed forever (the queued slot is still unused).
    let later = barrier
        .after_commit_with_transport(&s.output, 2, s.clock + 1, &mut fake)
        .unwrap_err();
    assert_eq!(reason(later), "barrier:barrier_disarmed");
    assert_eq!(fake.receive_calls, 1);
    assert_eq!(fake.script.len(), 0);
    cleanup(&s);
}

#[test]
fn both_orders_first_accepted_decision_wins() {
    // release-then-abort: release wins and the abort is never consumed.
    let s = armed_setup("chan-order1");
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![
            Ok(Some(frame(
                &s.secret, "release", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
            ))),
            Ok(Some(frame(
                &s.secret, "abort", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
            ))),
        ],
        ..FakeTransport::default()
    };
    barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap();
    assert_eq!(barrier.phase, BarrierPhase::Completed);
    assert_eq!(fake.script.len(), 1); // abort still queued, never reached
    cleanup(&s);

    // abort-then-release: abort wins; the release can never undo it.
    let s = armed_setup("chan-order2");
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![
            Ok(Some(frame(
                &s.secret, "abort", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
            ))),
            Ok(Some(frame(
                &s.secret, "release", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
            ))),
        ],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_aborted_by_supervisor");
    assert_eq!(barrier.phase, BarrierPhase::Disarmed);
    assert_eq!(fake.script.len(), 1); // release still queued, never reached
    cleanup(&s);
}

#[test]
fn durable_bundle_must_exist_before_any_decision_is_consumed() {
    // No armed bundle on disk: the REAL durable read refuses before touching
    // the channel — the observer cannot arm on a caller-supplied state alone.
    let dir = temp_dir("chan-nodir-dir");
    let output = temp_dir("chan-nodir-out"); // intentionally no step bundle
    let secret = [11_u8; 32];
    let nonce = "c".repeat(64);
    let deadline = now_epoch() + 60;
    let mut barrier = channel_barrier(&dir, &nonce, secret, 32, deadline, &"9".repeat(64));
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &secret,
            "release",
            &nonce,
            32,
            &"a".repeat(64),
            1,
            deadline,
            &"9".repeat(64),
        )))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&output, 1, 32, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_bundle_not_durable");
    assert_eq!(fake.receive_calls, 0); // channel never consulted
    assert_eq!(barrier.phase, BarrierPhase::Waiting);
    assert_eq!(
        fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("barrier-armed"))
            .count(),
        0
    );
    fs::remove_dir_all(dir).unwrap();
    fs::remove_dir_all(output).unwrap();
}

#[test]
fn opt_in_channel_ignores_filesystem_token_entirely() {
    // A VALID release.token is planted, but channel mode is EOF-only: it must
    // fail closed on EOF and never read the file — no filesystem fallback.
    let s = armed_setup("chan-nofs");
    fs::write(
        s.dir.join(RELEASE_FILE),
        format!(
            "{}\n",
            super::token::derive_token(&s.secret, "release", &s.nonce, s.clock, &s.state)
        ),
    )
    .unwrap();
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(None)],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_channel_eof");
    assert_eq!(barrier.phase, BarrierPhase::Disarmed);
    cleanup(&s);
}

#[test]
fn missing_durable_rest_record_refuses_before_any_channel_byte() {
    // The armed bundle is durable but the FIRST (REST) record is absent: the
    // channel binding refuses BEFORE the armed receipt and before reading the
    // channel; nothing falls back to the environment value.
    let s = armed_setup("chan-norest");
    fs::remove_file(s.output.join(super::durable::REST_RECORD)).unwrap();
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &s.secret, "release", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
        )))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_rest_record_unavailable");
    assert_eq!(fake.receive_calls, 0);
    assert_eq!(barrier.phase, BarrierPhase::Waiting);
    assert!(!s.dir.join(ARMED_RECEIPT).exists());
    cleanup(&s);
}

#[test]
fn foreign_identity_in_durable_records_refuses_the_decision() {
    // REST record carries a DIFFERENT run identity than the reviewed proof.
    let s = armed_setup("chan-forg-rest");
    fs::write(
        s.output.join(super::durable::REST_RECORD),
        format!(
            "{{\n  \"identity\": \"source={SRC};case=fixture;profile={PROF};schema=OTHER-v1\",\n  \"clock\": 0\n}}\n"
        ),
    )
    .unwrap();
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport::default();
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(
        reason(error),
        "barrier:barrier_rest_record_identity_mismatch"
    );
    assert_eq!(fake.receive_calls, 0);
    assert!(!s.dir.join(ARMED_RECEIPT).exists());
    cleanup(&s);

    // Armed step record carries a foreign identity (REST record is correct).
    let s = armed_setup("chan-forg-armed");
    fs::write(
        s.output
            .join(format!("step-001-clock-{:04}", s.clock))
            .join("record.json"),
        format!(
            "{{\n  \"identity\": \"source={SRC};case=fixture;profile={PROF};schema=OTHER-v1\",\n  \"state_sha256\": \"{}\"\n}}\n",
            s.state
        ),
    )
    .unwrap();
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport::default();
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(
        reason(error),
        "barrier:barrier_armed_record_identity_mismatch"
    );
    assert_eq!(fake.receive_calls, 0);
    assert!(!s.dir.join(ARMED_RECEIPT).exists());
    cleanup(&s);
}

#[test]
fn env_rest_hash_must_equal_the_durable_rest_record_bytes() {
    // The environment hash is well-formed but does not match the SHA-256 of
    // the durable REST record bytes on disk: the decision is refused before
    // any channel byte is read — an arbitrary hex64 can never authorize.
    let s = armed_setup("chan-restunbound");
    let mut barrier = channel_barrier(
        &s.dir,
        &s.nonce,
        s.secret,
        s.clock,
        s.deadline,
        &"f".repeat(64),
    );
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &s.secret,
            "release",
            &s.nonce,
            s.clock,
            &s.state,
            1,
            s.deadline,
            &"f".repeat(64),
        )))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_channel_rest_unbound");
    assert_eq!(fake.receive_calls, 0);
    assert!(!s.dir.join(ARMED_RECEIPT).exists());
    cleanup(&s);
}
