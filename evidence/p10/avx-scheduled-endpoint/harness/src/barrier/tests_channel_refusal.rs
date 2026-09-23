//! Opt-in inherited decision-channel tests (fail-closed refusals): every
//! deviation — wrong binding field, wrong token, malformed/ambiguous/expired/
//! EOF input, and a frame bound to a foreign REST hash — is refused with a
//! distinct reason, an honest acknowledgment, and permanent disarmament.

use super::durable::now_epoch;
use super::fixtures::*;
use super::*;
use crate::decision::{
    token_digest, RawDecision, TransportError, ACK_ACCEPTED, ACK_DISARMED, ACK_REFUSED,
};

#[test]
fn frame_rest_must_equal_the_durable_rest_hash_after_binding() {
    // Durable binding passes (env hash == durable bytes); the FRAME carries a
    // different rest value: refused at consumption, never accepted.
    let s = armed_setup("chan-frame-rest");
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame_with(
            &s.secret,
            "release",
            &s.nonce,
            s.clock,
            &s.state,
            1,
            s.deadline,
            SRC,
            PROF,
            &"7".repeat(64),
        )))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_rest_mismatch");
    assert_eq!(fake.acks.len(), 1);
    assert_eq!(fake.acks[0].status, ACK_REFUSED);
    cleanup(&s);
}

#[test]
fn wrong_binding_field_fails_closed_before_token_check() {
    let cases: Vec<(&str, RawDecision, &str)> = vec![
        (
            "clock",
            frame_with(
                &[11_u8; 32],
                "release",
                &"c".repeat(64),
                33,
                &"a".repeat(64),
                1,
                0,
                SRC,
                PROF,
                &"a".repeat(64),
            ),
            "barrier:barrier_clock_mismatch",
        ),
        (
            "nonce",
            frame_with(
                &[11_u8; 32],
                "release",
                &"d".repeat(64),
                32,
                &"a".repeat(64),
                1,
                0,
                SRC,
                PROF,
                &"a".repeat(64),
            ),
            "barrier:barrier_nonce_mismatch",
        ),
        (
            "state",
            frame_with(
                &[11_u8; 32],
                "release",
                &"c".repeat(64),
                32,
                &"b".repeat(64),
                1,
                0,
                SRC,
                PROF,
                &"a".repeat(64),
            ),
            "barrier:barrier_state_mismatch",
        ),
        (
            "source",
            frame_with(
                &[11_u8; 32],
                "release",
                &"c".repeat(64),
                32,
                &"a".repeat(64),
                1,
                0,
                "OTHER",
                PROF,
                &"a".repeat(64),
            ),
            "barrier:barrier_source_mismatch",
        ),
        (
            "profile",
            frame_with(
                &[11_u8; 32],
                "release",
                &"c".repeat(64),
                32,
                &"a".repeat(64),
                1,
                0,
                SRC,
                "OTHER",
                &"a".repeat(64),
            ),
            "barrier:barrier_profile_mismatch",
        ),
        (
            "attempt",
            frame_with(
                &[11_u8; 32],
                "release",
                &"c".repeat(64),
                32,
                &"a".repeat(64),
                2,
                0,
                SRC,
                PROF,
                &"a".repeat(64),
            ),
            "barrier:barrier_attempt_mismatch",
        ),
    ];
    for (label, mut bad, want) in cases {
        let s = armed_setup(&format!("chan-bad-{label}"));
        bad.deadline_epoch = s.deadline; // only the target field differs
        bad.rest = s.rest.clone(); // rest is correct; only the target differs
        let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
        let mut fake = FakeTransport {
            script: vec![Ok(Some(bad))],
            ..FakeTransport::default()
        };
        let error = barrier
            .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
            .unwrap_err();
        assert_eq!(reason(error), want, "case {label}");
        assert_eq!(barrier.phase, BarrierPhase::Disarmed);
        assert_eq!(fake.acks.len(), 1);
        assert_eq!(fake.acks[0].status, ACK_REFUSED);
        assert_eq!(fake.acks[0].token_digest, [0_u8; 32]);
        cleanup(&s);
    }
}

#[test]
fn non_release_abort_action_is_refused_before_the_token_check() {
    // Every field and the token would be valid for the given action, yet the
    // action itself is outside the reviewed vocabulary: refused as malformed.
    let s = armed_setup("chan-bad-action");
    let mut bad = frame(
        &s.secret, "halt", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
    );
    bad.token_hex = super::token::derive_token(&s.secret, "halt", &s.nonce, s.clock, &s.state);
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(bad))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_action_malformed");
    assert_eq!(barrier.phase, BarrierPhase::Disarmed);
    assert_eq!(fake.acks.len(), 1);
    assert_eq!(fake.acks[0].status, ACK_REFUSED);
    assert_eq!(fake.acks[0].token_digest, [0_u8; 32]);
    cleanup(&s);
}

#[test]
fn deadline_field_mismatch_is_refused_before_the_token_check() {
    // A frame whose deadline differs from the durable binding's is refused as
    // a mismatch (not as expiry) even when the frame deadline is still future
    // and its token is valid for every token-relevant field.
    let s = armed_setup("chan-bad-deadline");
    let mut bad = frame(
        &s.secret, "release", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
    );
    bad.deadline_epoch = s.deadline + 10;
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(bad))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_deadline_mismatch");
    assert_eq!(barrier.phase, BarrierPhase::Disarmed);
    assert_eq!(fake.acks.len(), 1);
    assert_eq!(fake.acks[0].status, ACK_REFUSED);
    assert_eq!(fake.acks[0].token_digest, [0_u8; 32]);
    cleanup(&s);
}

#[test]
fn wrong_secret_token_is_refused_even_with_everything_else_correct() {
    let s = armed_setup("chan-badtoken");
    let wrong = [99_u8; 32];
    let mut bad = frame(
        &wrong, "release", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
    );
    bad.deadline_epoch = s.deadline;
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(bad))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_token_mismatch");
    cleanup(&s);
}

#[test]
fn malformed_ambiguous_expired_eof_all_fail_closed() {
    for (label, result, want, expect_ack) in [
        (
            "malformed",
            Err(TransportError::Malformed),
            "barrier:barrier_channel_malformed",
            true,
        ),
        (
            "ambiguous",
            Err(TransportError::Ambiguous),
            "barrier:barrier_channel_ambiguous",
            true,
        ),
        (
            "expired",
            Err(TransportError::Expired),
            "barrier:barrier_expired",
            true,
        ),
        (
            "eof",
            Ok::<Option<RawDecision>, TransportError>(None),
            "barrier:barrier_channel_eof",
            false,
        ),
    ] {
        let s = armed_setup(&format!("chan-{label}"));
        let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
        let mut fake = FakeTransport {
            script: vec![result],
            ..FakeTransport::default()
        };
        let error = barrier
            .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
            .unwrap_err();
        assert_eq!(reason(error), want, "case {label}");
        assert_eq!(barrier.phase, BarrierPhase::Disarmed);
        assert_eq!(fake.acks.len(), usize::from(expect_ack));
        assert_eq!(fake.closed, 1);
        cleanup(&s);
    }
}

#[test]
fn late_decision_is_refused_at_the_acceptance_recheck() {
    let s = armed_setup("chan-late");
    let past = now_epoch().saturating_sub(1);
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, past, &s.rest);
    // A perfectly-correct frame delivered after the deadline must not release.
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &s.secret, "release", &s.nonce, s.clock, &s.state, 1, past, &s.rest,
        )))],
        ..FakeTransport::default()
    };
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_expired");
    assert_eq!(barrier.phase, BarrierPhase::Disarmed);
    cleanup(&s);
}

#[test]
fn acknowledgment_identifies_only_the_accepted_decision() {
    let s = armed_setup("chan-ack-rel");
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
    let release_token =
        super::token::derive_token(&s.secret, "release", &s.nonce, s.clock, &s.state);
    assert_eq!(fake.acks.len(), 1);
    assert_eq!(fake.acks[0].status, ACK_ACCEPTED);
    assert_eq!(fake.acks[0].action, Some("release"));
    assert_eq!(fake.acks[0].nonce_hex, s.nonce);
    assert_eq!(fake.acks[0].token_digest, token_digest(&release_token));

    let s = armed_setup("chan-ack-abort");
    let mut barrier = channel_barrier(&s.dir, &s.nonce, s.secret, s.clock, s.deadline, &s.rest);
    let mut fake = FakeTransport {
        script: vec![Ok(Some(frame(
            &s.secret, "abort", &s.nonce, s.clock, &s.state, 1, s.deadline, &s.rest,
        )))],
        ..FakeTransport::default()
    };
    barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    let abort_token = super::token::derive_token(&s.secret, "abort", &s.nonce, s.clock, &s.state);
    assert_eq!(fake.acks[0].status, ACK_DISARMED);
    assert_eq!(fake.acks[0].action, Some("abort"));
    assert_eq!(fake.acks[0].token_digest, token_digest(&abort_token));
    assert_ne!(fake.acks[0].token_digest, token_digest(&release_token));
    cleanup(&s);
}

#[test]
fn injecting_a_transport_into_a_filesystem_run_is_refused() {
    let s = armed_setup("chan-inject-fs");
    let mut barrier = Barrier {
        dir: s.dir.clone(),
        nonce: s.nonce.clone(),
        secret: s.secret,
        armed_clock: s.clock,
        deadline_epoch: s.deadline,
        phase: BarrierPhase::Waiting,
        channel: None,
    };
    let mut fake = FakeTransport::default();
    let error = barrier
        .after_commit_with_transport(&s.output, 1, s.clock, &mut fake)
        .unwrap_err();
    assert_eq!(reason(error), "barrier:barrier_channel_unconfigured");
    assert_eq!(fake.receive_calls, 0);
    cleanup(&s);
}

#[test]
fn channel_ack_status_bytes_are_distinct_constants() {
    use crate::decision::{ACK_EXPIRED, ACK_REFUSED};
    assert_ne!(ACK_ACCEPTED, ACK_DISARMED);
    assert_ne!(ACK_EXPIRED, ACK_REFUSED);
    assert_eq!(ACK_ACCEPTED, b'A');
}
