//! Presence-aware, fail-closed parsing of the canonical barrier environment.
//!
//! The sealed review counterexamples this parser closes:
//!   * `CHANNEL_FD` present BY ITSELF previously returned `Ok(None)`
//!     (no barrier) — it is now a hard refusal;
//!   * non-UTF-8 values were previously indistinguishable from absence and
//!     silently selected the legacy path — invalid Unicode now refuses with a
//!     distinct reason and never selects any fallback;
//!   * an invalid number (signs, spaces, non-digits, empty) never falls back:
//!     every numeric field is parsed strictly;
//!   * any disagreement between present-but-incomplete legacy and channel
//!     variables is a refusal; presence of ANY barrier variable requires the
//!     COMPLETE legacy binding;
//!   * channel opt-in without the caller-supplied reviewed identity proof is
//!     refused (`barrier_channel_identity_unavailable`), and with the proof
//!     the env `SOURCE`/`PROFILE` must equal the frozen identity components
//!     and the COMPILED profile (`barrier_channel_source_unbound` /
//!     `barrier_channel_profile_unbound`), `REST` must be a 64-hex digest
//!     (its equality to the durable REST record is checked at consumption);
//!   * channel opt-in is admitted ONLY for the reviewed N256/M512 capture
//!     profile and ONLY with the armed clock equal to the exact reviewed
//!     first clock (N256 clock 64) — every other profile refuses
//!     (`barrier_channel_unsupported_profile`) and every other clock refuses
//!     (`barrier_channel_clock_unreviewed`) BEFORE execution.

use super::{durable, token, Barrier, BarrierPhase, ChannelConfig};
use crate::error::HarnessError;
use std::{path::Path, time::Duration};

pub const DIR_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_DIR";
pub const NONCE_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_NONCE";
pub const SECRET_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_SECRET";
pub const CLOCK_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_CLOCK";
pub const DEADLINE_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_DEADLINE";
pub const ARMED_RECEIPT: &str = "barrier-armed.json";
pub const RELEASE_FILE: &str = "release.token";
pub const ABORT_FILE: &str = "abort.token";

// Canonical opt-in channel selection for the actual N256 child environment.
// The descriptor number of the inherited decision endpoint is the sole switch;
// its presence forbids the legacy two-file mode for this run.
pub const CHANNEL_FD_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_CHANNEL_FD";
pub const SOURCE_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_SOURCE";
pub const PROFILE_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_PROFILE";
pub const REST_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_REST";

pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(500);

const LEGACY_KEYS: [&str; 5] = [DIR_ENV, NONCE_ENV, SECRET_ENV, CLOCK_ENV, DEADLINE_ENV];
const CHANNEL_KEYS: [&str; 4] = [CHANNEL_FD_ENV, SOURCE_ENV, PROFILE_ENV, REST_ENV];

/// The inherited decision channel is admitted ONLY for the reviewed
/// N256/M512 capture-profile binary.  The broader `capture_offline` family
/// (the N512 parallel-capture profiles) and the plain `test` cfg are NOT
/// admission: outside the reviewed feature the full opt-in refuses with
/// `barrier_channel_unsupported_profile` in the very binary that would have
/// accepted it (the sealed review's N512 counterexample is a denial test).
const CHANNEL_SUPPORTED: bool = cfg!(feature = "n256-m512-piecewise-cadv33");

/// Raw environment slot with invalid-Unicode PRESENCE preserved: `var_os`
/// distinguishes "not present" from "present but not valid Unicode", which
/// `std::env::var` collapses — the exact defect from the sealed review.
enum Slot {
    Absent,
    Text(String),
    NonUnicode,
}

impl Slot {
    fn is_present(&self) -> bool {
        matches!(self, Self::Text(_))
    }
}

fn probe(key: &str) -> Slot {
    match std::env::var_os(key) {
        None => Slot::Absent,
        Some(raw) => match raw.into_string() {
            Ok(text) => Slot::Text(text),
            Err(_) => Slot::NonUnicode,
        },
    }
}

fn require_text<'a>(slot: &'a Slot, missing: &'static str) -> Result<&'a str, HarnessError> {
    match slot {
        Slot::Text(value) => Ok(value),
        _ => Err(HarnessError::Barrier(missing)),
    }
}

/// Strict, non-empty ASCII digits only: no sign, no whitespace, no empty.
fn digits_to_u128(value: &str) -> Option<u128> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse::<u128>().ok())?
}

fn digits_to_u64(value: &str) -> Option<u64> {
    digits_to_u128(value).and_then(|value| u64::try_from(value).ok())
}

/// A source/profile component of the reviewed run identity string.
struct IdentityAnchor<'a> {
    source: &'a str,
    profile: &'a str,
}

impl<'a> IdentityAnchor<'a> {
    fn parse(identity: &'a str) -> Option<IdentityAnchor<'a>> {
        let mut source = None;
        let mut profile = None;
        for field in identity.split(';') {
            if let Some(value) = field.strip_prefix("source=") {
                source = Some(value);
            } else if let Some(value) = field.strip_prefix("profile=") {
                profile = Some(value);
            }
        }
        match (source, profile) {
            (Some(s), Some(p)) if !s.is_empty() && !p.is_empty() => Some(Self {
                source: s,
                profile: p,
            }),
            _ => None,
        }
    }
}

/// Parse the complete binding or refuse; a partially armed barrier is a
/// configuration error, never a silent fall-through.  Absent EVERY barrier
/// variable (legacy and channel alike) is the ordinary (unarmed) path.
pub(super) fn parse(reviewed_identity: Option<&str>) -> Result<Option<Barrier>, HarnessError> {
    let legacy: Vec<Slot> = LEGACY_KEYS.iter().map(|key| probe(key)).collect();
    let channel: [Slot; 4] = CHANNEL_KEYS
        .iter()
        .map(|key| probe(key))
        .collect::<Vec<Slot>>()
        .try_into()
        .unwrap_or([Slot::Absent, Slot::Absent, Slot::Absent, Slot::Absent]);
    if legacy
        .iter()
        .chain(channel.iter())
        .any(|slot| matches!(slot, Slot::NonUnicode))
    {
        return Err(HarnessError::Barrier("barrier_env_non_unicode"));
    }
    let legacy_present = legacy.iter().any(Slot::is_present);
    let channel_present = channel.iter().any(Slot::is_present);
    if !legacy_present && !channel_present {
        return Ok(None);
    }
    // Without the legacy anchor directory the run is a PARTIAL arm —
    // including the sealed counterexample "CHANNEL_FD present by itself",
    // which is a refusal here and can never return `Ok(None)`.
    if !legacy[0].is_present() {
        return Err(HarnessError::Barrier("barrier_partial_environment"));
    }
    let dir = require_text(&legacy[0], "barrier_dir_missing")?;
    let nonce = require_text(&legacy[1], "barrier_nonce_missing")?;
    if !token::is_hex64(nonce) {
        return Err(HarnessError::Barrier("barrier_nonce_malformed"));
    }
    let secret = token::parse_secret_env(require_text(&legacy[2], "barrier_secret_missing")?)?;
    let clock_text = require_text(&legacy[3], "barrier_clock_malformed")?;
    let armed_clock = digits_to_u128(clock_text)
        .filter(|&clock| clock > 0 && clock < crate::schedule::ENDPOINT)
        .ok_or(HarnessError::Barrier("barrier_clock_malformed"))?;
    let deadline_text = require_text(&legacy[4], "barrier_deadline_not_future")?;
    let deadline_epoch = digits_to_u64(deadline_text)
        .filter(|&deadline| deadline > durable::now_epoch())
        .ok_or(HarnessError::Barrier("barrier_deadline_not_future"))?;
    if !Path::new(dir).is_dir() {
        return Err(HarnessError::Barrier("barrier_dir_missing"));
    }
    let channel_config = if channel_present {
        Some(parse_channel(&channel, reviewed_identity, armed_clock)?)
    } else {
        None
    };
    Ok(Some(Barrier {
        dir: dir.into(),
        nonce: nonce.to_owned(),
        secret,
        armed_clock,
        deadline_epoch,
        phase: BarrierPhase::Waiting,
        channel: channel_config,
    }))
}

/// The channel block parses ONLY with: the reviewed N256/M512 capture-profile
/// binary, the armed clock equal to the EXACT reviewed first clock (N256
/// clock 64 — attempt 1's committed clock; any unreachable or later-reachable
/// clock refuses BEFORE execution), a supplied reviewed-identity proof, the
/// full four-variable opt-in, a strict descriptor number and env identity
/// values equal to the FROZEN identity components.  Any other combination is
/// a refusal.
fn parse_channel(
    channel: &[Slot; 4],
    reviewed_identity: Option<&str>,
    armed_clock: u128,
) -> Result<ChannelConfig, HarnessError> {
    if !CHANNEL_SUPPORTED {
        return Err(HarnessError::Barrier("barrier_channel_unsupported_profile"));
    }
    // The channel intercepts exactly ONE canonical first-step event: the
    // first committed durable clock of the reviewed schedule.  A clock the
    // schedule can never durably present (e.g. 1), or one only reachable at
    // a later attempt, is a refusal here — never a silent 48-attempt run to
    // the terminal without a decision.
    let reviewed_clock = crate::schedule::step(0).unwrap_or(u128::MAX);
    if armed_clock != reviewed_clock {
        return Err(HarnessError::Barrier("barrier_channel_clock_unreviewed"));
    }
    let Some(reviewed) = reviewed_identity else {
        return Err(HarnessError::Barrier(
            "barrier_channel_identity_unavailable",
        ));
    };
    let anchor = IdentityAnchor::parse(reviewed)
        .ok_or(HarnessError::Barrier("barrier_identity_proof_malformed"))?;
    let [fd_slot, source_slot, profile_slot, rest_slot] = channel;
    if !fd_slot.is_present() {
        // A channel identity without its descriptor is a partial arm.
        return Err(HarnessError::Barrier("barrier_channel_partial"));
    }
    let fd_text = require_text(fd_slot, "barrier_channel_fd_malformed")?;
    let fd = digits_to_u128(fd_text)
        .and_then(|fd| i32::try_from(fd).ok())
        .filter(|&fd| fd >= 3)
        .ok_or(HarnessError::Barrier("barrier_channel_fd_malformed"))?;
    let source = require_text(source_slot, "barrier_channel_source_missing")?;
    let profile = require_text(profile_slot, "barrier_channel_profile_missing")?;
    let rest = require_text(rest_slot, "barrier_channel_rest_missing")?;
    if source != anchor.source {
        return Err(HarnessError::Barrier("barrier_channel_source_unbound"));
    }
    if profile != anchor.profile || profile != crate::config::PROFILE {
        return Err(HarnessError::Barrier("barrier_channel_profile_unbound"));
    }
    if !token::is_hex64(rest) {
        return Err(HarnessError::Barrier("barrier_channel_rest_malformed"));
    }
    Ok(ChannelConfig {
        fd,
        source: source.to_owned(),
        profile: profile.to_owned(),
        rest: rest.to_owned(),
        reviewed_identity: reviewed.to_owned(),
    })
}
