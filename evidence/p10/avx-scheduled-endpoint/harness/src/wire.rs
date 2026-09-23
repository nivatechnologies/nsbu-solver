//! CANDIDATE wire codec for the inherited decision channel.
//!
//! NOT ACCEPTED.  The independent reviews left the concrete wire and deadline
//! handling open; this module exists ONLY so the barrier has a byte-transport
//! behind the narrow [`crate::decision`] interface and so the
//! descriptor-inheritance subprocess tests can drive a real socketpair.
//! Nothing in `barrier/` depends on this codec's correctness: the barrier
//! re-derives the token and compares every field itself.  The frozen 110-byte
//! decision frame and 74-byte decision-only acknowledgment are preserved as the
//! versioned core (`N256DCv1`/`N256ACv1`); the `v2` frame adds a strictly
//! bounded source/profile/rest/attempt/deadline binding extension.  The exact
//! identity length caps below are provisional and are part of the pending
//! transport repair, not claims of this consumer.

use crate::decision::{hex32_to_bytes, DecisionAck};

pub const DECISION_MAGIC: &[u8; 8] = b"N256DCv2";
pub const ACK_MAGIC: &[u8; 8] = b"N256ACv1";
pub const ACTION_RELEASE: u8 = b'R';
pub const ACTION_ABORT: u8 = b'A';

pub const ACK_ACCEPTED: u8 = b'A';
pub const ACK_DISARMED: u8 = b'X';
pub const ACK_EXPIRED: u8 = b'E';
pub const ACK_REFUSED: u8 = b'F';

const HEADER_LEN: usize = 8 + 1 + 1 + 4 + 32 + 32 + 32 + 4 + 8; // 154
pub const MAX_ID: usize = 512;
pub const MAX_FRAME: usize = HEADER_LEN + 3 * (1 + MAX_ID); // 1693
pub const ACK_LEN: usize = 8 + 1 + 1 + 32 + 32; // 74

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    Oversize,
    Truncated,
    VersionMismatch,
    ReservedNonzero,
    ActionMalformed,
    ClockMalformed,
    FieldMalformed,
    StatusMalformed,
}

fn action_byte(action: &str) -> Result<u8, CodecError> {
    match action {
        "release" => Ok(ACTION_RELEASE),
        "abort" => Ok(ACTION_ABORT),
        _ => Err(CodecError::ActionMalformed),
    }
}

fn action_name(byte: u8) -> Result<&'static str, CodecError> {
    match byte {
        ACTION_RELEASE => Ok("release"),
        ACTION_ABORT => Ok("abort"),
        _ => Err(CodecError::ActionMalformed),
    }
}

#[allow(dead_code)]
fn push_id(out: &mut Vec<u8>, id: &str) -> Result<(), CodecError> {
    if id.len() > MAX_ID || id.len() > u8::MAX as usize {
        return Err(CodecError::FieldMalformed);
    }
    out.push(id.len() as u8);
    out.extend_from_slice(id.as_bytes());
    Ok(())
}

/// Serialize the versioned, bounded decision frame carrying the action, its
/// canonical derived token, and the full durable binding extension.  Used by
/// the supervisor side and the harness tests (not by the child binary).
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn encode_decision(
    action: &str,
    nonce_hex: &str,
    clock: u32,
    state_hex: &str,
    token_hex: &str,
    attempt: usize,
    deadline_epoch: u64,
    source: &str,
    profile: &str,
    rest: &str,
) -> Result<Vec<u8>, CodecError> {
    if clock == 0 || clock >= 4096 {
        return Err(CodecError::ClockMalformed);
    }
    let action = action_byte(action)?;
    let nonce = hex32_to_bytes(nonce_hex).ok_or(CodecError::FieldMalformed)?;
    let state = hex32_to_bytes(state_hex).ok_or(CodecError::FieldMalformed)?;
    let token = hex32_to_bytes(token_hex).ok_or(CodecError::FieldMalformed)?;
    let attempt = u32::try_from(attempt).map_err(|_| CodecError::FieldMalformed)?;
    let mut out = Vec::with_capacity(HEADER_LEN + 64);
    out.extend_from_slice(DECISION_MAGIC);
    out.push(action);
    out.push(0);
    out.extend_from_slice(&clock.to_be_bytes());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&state);
    out.extend_from_slice(&token);
    out.extend_from_slice(&attempt.to_be_bytes());
    out.extend_from_slice(&deadline_epoch.to_be_bytes());
    push_id(&mut out, source)?;
    push_id(&mut out, profile)?;
    push_id(&mut out, rest)?;
    Ok(out)
}

/// One datagram decoded into a raw, unverified decision. `truncated` is the
/// kernel MSG_TRUNC flag; any length/version/reserved/encoding deviation is a
/// fail-closed refusal.  The caller (the barrier) performs all authentication.
pub struct Decoded {
    pub action: &'static str,
    pub nonce_hex: String,
    pub clock: u32,
    pub state_hex: String,
    pub token_hex: String,
    pub attempt: usize,
    pub deadline_epoch: u64,
    pub source: String,
    pub profile: String,
    pub rest: String,
}

pub fn decode_decision(raw: &[u8], truncated: bool) -> Result<Decoded, CodecError> {
    if truncated || raw.len() > MAX_FRAME {
        return Err(CodecError::Oversize);
    }
    if raw.len() < HEADER_LEN + 3 {
        return Err(CodecError::Truncated);
    }
    if &raw[0..8] != DECISION_MAGIC {
        return Err(CodecError::VersionMismatch);
    }
    if raw[9] != 0 {
        return Err(CodecError::ReservedNonzero);
    }
    let action = action_name(raw[8])?;
    let clock = u32::from_be_bytes([raw[10], raw[11], raw[12], raw[13]]);
    if clock == 0 || clock >= 4096 {
        return Err(CodecError::ClockMalformed);
    }
    let nonce_hex = to_hex(&raw[14..46]);
    let state_hex = to_hex(&raw[46..78]);
    let token_hex = to_hex(&raw[78..110]);
    let attempt = u32::from_be_bytes([raw[110], raw[111], raw[112], raw[113]]) as usize;
    let mut deadline_bytes = [0_u8; 8];
    deadline_bytes.copy_from_slice(&raw[114..122]);
    let deadline_epoch = u64::from_be_bytes(deadline_bytes);
    let mut cursor = 122_usize;
    let source = read_id(raw, &mut cursor)?;
    let profile = read_id(raw, &mut cursor)?;
    let rest = read_id(raw, &mut cursor)?;
    if cursor != raw.len() {
        return Err(CodecError::FieldMalformed);
    }
    Ok(Decoded {
        action,
        nonce_hex,
        clock,
        state_hex,
        token_hex,
        attempt,
        deadline_epoch,
        source,
        profile,
        rest,
    })
}

fn read_id(raw: &[u8], cursor: &mut usize) -> Result<String, CodecError> {
    let len = *raw.get(*cursor).ok_or(CodecError::Truncated)? as usize;
    *cursor += 1;
    if len > MAX_ID {
        return Err(CodecError::FieldMalformed);
    }
    let end = cursor.checked_add(len).ok_or(CodecError::Truncated)?;
    if end > raw.len() {
        return Err(CodecError::Truncated);
    }
    let text = std::str::from_utf8(&raw[*cursor..end])
        .map_err(|_| CodecError::FieldMalformed)?
        .to_owned();
    *cursor = end;
    Ok(text)
}

/// Serialize the fixed, decision-only acknowledgment (identifies only the
/// accepted decision; proves nothing about attempt 2/completion/cleanup).
pub fn encode_ack(ack: &DecisionAck) -> Result<Vec<u8>, CodecError> {
    let status = match ack.status {
        ACK_ACCEPTED | ACK_DISARMED | ACK_EXPIRED | ACK_REFUSED => ack.status,
        _ => return Err(CodecError::StatusMalformed),
    };
    let action_byte = match ack.action {
        None => 0,
        Some(action) => action_byte(action)?,
    };
    let nonce = hex32_to_bytes(&ack.nonce_hex).ok_or(CodecError::FieldMalformed)?;
    let mut out = Vec::with_capacity(ACK_LEN);
    out.extend_from_slice(ACK_MAGIC);
    out.push(status);
    out.push(action_byte);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ack.token_digest);
    Ok(out)
}

pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0xf)]));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::barrier::derive_token;
    use crate::decision::{token_digest, DecisionAck, ACK_ACCEPTED};

    fn sample_frame() -> Vec<u8> {
        let secret = [0x11_u8; 32];
        let nonce = "cd".repeat(32);
        let state = "ab".repeat(32);
        let token = derive_token(&secret, "release", &nonce, 32, &state);
        encode_decision(
            "release",
            &nonce,
            32,
            &state,
            &token,
            1,
            9_000_000_000,
            "src",
            "prof",
            "rest",
        )
        .unwrap()
    }

    #[test]
    fn decision_frame_round_trips_every_bound_field() {
        let bytes = sample_frame();
        assert!(bytes.len() <= MAX_FRAME);
        let decoded = decode_decision(&bytes, false).unwrap();
        assert_eq!(decoded.action, "release");
        assert_eq!(decoded.clock, 32);
        assert_eq!(decoded.source, "src");
        assert_eq!(decoded.profile, "prof");
        assert_eq!(decoded.rest, "rest");
        assert_eq!(decoded.attempt, 1);
        assert_eq!(decoded.deadline_epoch, 9_000_000_000);
    }

    #[test]
    fn codec_rejects_every_malformed_frame_variant() {
        let good = sample_frame();
        assert!(matches!(
            decode_decision(&good[..good.len() - 1], false),
            Err(CodecError::Truncated)
        ));
        assert!(matches!(
            decode_decision(&good, true),
            Err(CodecError::Oversize)
        ));
        let mut bad_magic = good.clone();
        bad_magic[0] = b'X';
        assert!(matches!(
            decode_decision(&bad_magic, false),
            Err(CodecError::VersionMismatch)
        ));
        let mut bad_reserved = good.clone();
        bad_reserved[9] = 1;
        assert!(matches!(
            decode_decision(&bad_reserved, false),
            Err(CodecError::ReservedNonzero)
        ));
        let mut bad_action = good.clone();
        bad_action[8] = b'Z';
        assert!(matches!(
            decode_decision(&bad_action, false),
            Err(CodecError::ActionMalformed)
        ));
    }

    #[test]
    fn codec_enforces_clock_range_on_encode() {
        let secret = [0x11_u8; 32];
        let nonce = "cd".repeat(32);
        let state = "ab".repeat(32);
        let token = derive_token(&secret, "release", &nonce, 32, &state);
        assert_eq!(
            encode_decision("release", &nonce, 0, &state, &token, 1, 0, "s", "p", "r"),
            Err(CodecError::ClockMalformed)
        );
        assert_eq!(
            encode_decision("release", &nonce, 4096, &state, &token, 1, 0, "s", "p", "r"),
            Err(CodecError::ClockMalformed)
        );
    }

    #[test]
    fn ack_is_bounded_and_identifies_only_the_decision() {
        let nonce = "cd".repeat(32);
        let digest = token_digest(&derive_token(
            &[0x11_u8; 32],
            "release",
            &nonce,
            32,
            &"ab".repeat(32),
        ));
        let ack = DecisionAck {
            status: ACK_ACCEPTED,
            action: Some("release"),
            nonce_hex: nonce.clone(),
            token_digest: digest,
        };
        let bytes = encode_ack(&ack).unwrap();
        assert_eq!(bytes.len(), ACK_LEN);
        assert_eq!(&bytes[0..8], b"N256ACv1");
        assert_eq!(bytes[8], ACK_ACCEPTED);
        assert_eq!(bytes[9], b'R');
        assert_eq!(token_digest("not-hex"), [0_u8; 32]);
    }

    #[test]
    fn release_and_abort_digests_differ() {
        let secret = [0x11_u8; 32];
        let nonce = "cd".repeat(32);
        let state = "ab".repeat(32);
        let rel = token_digest(&derive_token(&secret, "release", &nonce, 32, &state));
        let abo = token_digest(&derive_token(&secret, "abort", &nonce, 32, &state));
        assert_ne!(rel, abo);
    }
}
