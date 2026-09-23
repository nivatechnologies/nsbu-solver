//! Parser tests closing the three sealed counterexamples plus the full
//! fail-closed opt-in matrix.  Every case asserts a DISTINCT refusal reason
//! or the exact intended parse — never a silent fallback.

use super::durable::now_epoch;
use super::fixtures::*;
use super::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

fn clear_all() {
    for key in [
        DIR_ENV,
        NONCE_ENV,
        SECRET_ENV,
        CLOCK_ENV,
        DEADLINE_ENV,
        CHANNEL_FD_ENV,
        SOURCE_ENV,
        PROFILE_ENV,
        REST_ENV,
    ] {
        std::env::remove_var(key);
    }
}

fn set_legacy(dir: &std::path::Path) {
    std::env::set_var(DIR_ENV, dir);
    std::env::set_var(NONCE_ENV, "a".repeat(64));
    std::env::set_var(SECRET_ENV, "b".repeat(64));
    // The first committed clock of the COMPILED schedule: legal as a legacy
    // arming everywhere, and the ONLY clock channel mode may ever arm at.
    std::env::set_var(CLOCK_ENV, crate::schedule::step(0).unwrap().to_string());
    std::env::set_var(DEADLINE_ENV, (now_epoch() + 60).to_string());
}

/// Without the reviewed N256/M512 feature even a correctly-proven opt-in is
/// refused by the profile gate FIRST (the sealed review's N512 counterexample
/// is exactly this case); with the feature the identity guard applies.
#[cfg(feature = "n256-m512-piecewise-cadv33")]
const NO_PROOF_REASON: &str = "barrier_channel_identity_unavailable";
#[cfg(not(feature = "n256-m512-piecewise-cadv33"))]
const NO_PROOF_REASON: &str = "barrier_channel_unsupported_profile";

/// Counterexample 1 (sealed): `CHANNEL_FD` present BY ITSELF previously
/// produced NO barrier (`Ok(None)`).  It is now a hard refusal.
#[test]
fn channel_descriptor_alone_never_selects_the_unarmed_path() {
    let _guard = env_guard();
    clear_all();
    std::env::set_var(CHANNEL_FD_ENV, "3");
    let result = Barrier::from_env();
    assert!(
        matches!(
            result,
            Err(HarnessError::Barrier("barrier_partial_environment"))
        ),
        "lone CHANNEL_FD must refuse, got {result:?}"
    );
    // ...and it must NEVER be Ok(None), with or without an identity proof.
    assert!(Barrier::from_env_with_identity(&reviewed_identity()).is_err());
    clear_all();
}

/// Counterexample 2 (sealed): a full legacy binding plus a NON-UNICODE
/// `CHANNEL_FD` previously fell through to legacy arming.  Invalid Unicode
/// anywhere now refuses with its own reason — no fallback to any path.
#[test]
fn non_unicode_channel_value_refuses_instead_of_falling_back() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("nonunicode");
    set_legacy(&dir);
    std::env::set_var(CHANNEL_FD_ENV, OsString::from_vec(b"3\xff\xfe".to_vec()));
    assert!(
        matches!(
            Barrier::from_env(),
            Err(HarnessError::Barrier("barrier_env_non_unicode"))
        ),
        "non-UTF-8 CHANNEL_FD must refuse, never select legacy arming"
    );
    // The same holds inside the legacy set itself.
    std::env::remove_var(CHANNEL_FD_ENV);
    std::env::set_var(SECRET_ENV, OsString::from_vec(vec![0x62, 0xff]));
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_env_non_unicode"))
    ));
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}

/// Counterexample 3 (sealed): a well-formed channel opt-in with an ARBITRARY
/// identity was previously accepted.  With the reviewed-identity proof the
/// env source/profile must match the frozen identity components (and the
/// compiled profile), and REST must be a hex digest — any deviation is a
/// distinct hard refusal.
#[cfg(feature = "n256-m512-piecewise-cadv33")]
#[test]
fn arbitrary_identity_is_never_accepted() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("arb-id");
    set_legacy(&dir);
    // Without ANY reviewed-identity proof the opt-in is refused outright.
    std::env::set_var(CHANNEL_FD_ENV, "7");
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "a".repeat(64));
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier(
            "barrier_channel_identity_unavailable"
        ))
    ));
    // With the proof, an ARBITRARY source string (not the identity
    // component) is refused — this is the sealed counterexample itself.
    std::env::set_var(SOURCE_ENV, "arbitrary-source");
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_source_unbound"))
    ));
    // A PROFILE that is neither the identity component nor the compiled
    // profile is refused.
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, "not-the-compiled-profile");
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_profile_unbound"))
    ));
    // A REST value that is not a 64-hex digest is refused (its equality to
    // the durable REST record is verified separately at consumption).
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "not-a-hash");
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_rest_malformed"))
    ));
    // With every binding correct the opt-in parses to channel mode.
    std::env::set_var(REST_ENV, "a".repeat(64));
    let parsed = Barrier::from_env_with_identity(&reviewed_identity())
        .unwrap()
        .expect("fully bound channel barrier");
    assert!(parsed.is_channel_mode());
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
#[test]
fn channel_environment_requires_full_binding() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("chan-env");
    set_legacy(&dir);
    // descriptor set but identity variables missing -> per-variable refusal
    std::env::set_var(CHANNEL_FD_ENV, "7");
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_source_missing"))
    ));
    // identity variables set without descriptor -> partial arm
    std::env::remove_var(CHANNEL_FD_ENV);
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "a".repeat(64));
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_partial"))
    ));
    // valid channel opt-in parses to channel mode
    std::env::set_var(CHANNEL_FD_ENV, "7");
    let parsed = Barrier::from_env_with_identity(&reviewed_identity())
        .unwrap()
        .expect("channel barrier");
    assert!(parsed.is_channel_mode());
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
#[test]
fn channel_descriptor_parsing_is_strict() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("fd-strict");
    set_legacy(&dir);
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "a".repeat(64));
    for bad in ["", "3.5", " 3", "+3", "-1", "0", "2", "0x3", "3_4", "٣"] {
        std::env::set_var(CHANNEL_FD_ENV, bad);
        assert!(
            matches!(
                Barrier::from_env_with_identity(&reviewed_identity()),
                Err(HarnessError::Barrier("barrier_channel_fd_malformed"))
            ),
            "descriptor {bad:?} must be malformed"
        );
    }
    // 3 (the lowest legal inherited descriptor) and large valid values pass.
    for good in ["3", "7", "2147483647"] {
        std::env::set_var(CHANNEL_FD_ENV, good);
        let parsed = Barrier::from_env_with_identity(&reviewed_identity())
            .unwrap()
            .expect("bound channel barrier");
        assert!(parsed.is_channel_mode(), "descriptor {good:?} is legal");
    }
    // Beyond i32 the descriptor cannot be a raw fd.
    std::env::set_var(CHANNEL_FD_ENV, "2147483648");
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_fd_malformed"))
    ));
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn legacy_only_binding_still_parses_and_channel_never_downgrades() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("legacy-ok");
    set_legacy(&dir);
    // Pure legacy (no channel variables): unchanged, non-channel barrier.
    let parsed = Barrier::from_env()
        .unwrap()
        .expect("legacy barrier parses unchanged");
    assert!(!parsed.is_channel_mode());
    // Channel variables without a proof refuse even when the legacy quad is
    // complete: the legacy path is never silently selected instead.
    std::env::set_var(CHANNEL_FD_ENV, "7");
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "a".repeat(64));
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier(reason)) if reason == NO_PROOF_REASON
    ));
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}

/// Sealed-review counterexample #2 at parser level, PROCESS-DENIED in every
/// non-reviewed-profile build (default and the N512 capture profiles): the
/// COMPLETE otherwise-valid opt-in bound to the compiled profile is refused
/// with its own distinct reason — never accepted, never downgraded.
#[cfg(not(feature = "n256-m512-piecewise-cadv33"))]
#[test]
fn channel_opt_in_refuses_outside_the_reviewed_profile() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("profile-deny");
    set_legacy(&dir);
    std::env::set_var(CHANNEL_FD_ENV, "7");
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "a".repeat(64));
    assert!(matches!(
        Barrier::from_env_with_identity(&reviewed_identity()),
        Err(HarnessError::Barrier("barrier_channel_unsupported_profile"))
    ));
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_channel_unsupported_profile"))
    ));
    // Legacy mode is completely unaffected by the channel refusal.
    for key in [CHANNEL_FD_ENV, SOURCE_ENV, PROFILE_ENV, REST_ENV] {
        std::env::remove_var(key);
    }
    let parsed = Barrier::from_env()
        .unwrap()
        .expect("legacy binding still parses");
    assert!(!parsed.is_channel_mode());
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}

/// Channel mode arms ONLY at the exact reviewed first clock of the reviewed
/// schedule.  Unreachable clocks (1, 63, 65), later-reachable clocks (128,
/// 2048) and endpoint-adjacent values all refuse BEFORE execution; the
/// reviewed first clock parses.  This closes the review's silent-48-attempt
/// bypass at admission, in the reviewed N256 profile itself.
#[cfg(feature = "n256-m512-piecewise-cadv33")]
#[test]
fn channel_mode_requires_the_exact_reviewed_first_clock() {
    let _guard = env_guard();
    clear_all();
    let dir = temp_dir("clock-deny");
    set_legacy(&dir);
    std::env::set_var(CHANNEL_FD_ENV, "7");
    std::env::set_var(SOURCE_ENV, SRC);
    std::env::set_var(PROFILE_ENV, PROF);
    std::env::set_var(REST_ENV, "a".repeat(64));
    assert_eq!(crate::schedule::step(0).unwrap(), 64);
    for bad in ["1", "63", "65", "128", "2048", "4095"] {
        std::env::set_var(CLOCK_ENV, bad);
        assert!(
            matches!(
                Barrier::from_env_with_identity(&reviewed_identity()),
                Err(HarnessError::Barrier("barrier_channel_clock_unreviewed"))
            ),
            "armed clock {bad} must refuse as unreviewed"
        );
    }
    // Values outside the legacy clock domain never even reach channel gates.
    for illegal in ["0", "4096", "999999999999999999999"] {
        std::env::set_var(CLOCK_ENV, illegal);
        assert!(
            matches!(
                Barrier::from_env_with_identity(&reviewed_identity()),
                Err(HarnessError::Barrier("barrier_clock_malformed"))
            ),
            "armed clock {illegal} must be malformed"
        );
    }
    // The reviewed first clock (N256: 64) arms.
    std::env::set_var(CLOCK_ENV, "64");
    let parsed = Barrier::from_env_with_identity(&reviewed_identity())
        .unwrap()
        .expect("reviewed first clock admits channel mode");
    assert!(parsed.is_channel_mode());
    clear_all();
    fs::remove_dir_all(dir).unwrap();
}
