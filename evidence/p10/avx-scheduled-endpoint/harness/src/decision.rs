//! The NARROW decision-consumption interface the patched barrier depends on.
//!
//! This module is the barrier's entire view of "the transport".  The barrier
//! knows NOTHING about sockets, framing or deadline polling; it only calls the
//! three methods of [`DecisionTransport`] and then performs every trust
//! decision itself (token re-derivation, exact binding equality against the
//! durable reviewed-identity proof, deadline recheck at the irreversible
//! transition, one-shot latching and permanent close).  That split is
//! deliberate: the independent reviews (`harness/reviewed/` FAIL reports) left
//! the concrete SOCK_SEQPACKET wire/deadline code NOT accepted, so the barrier
//! must not be welded to it.  A transport that delivers bytes is a *pluggable
//! implementer* behind this trait (see the clearly-marked candidate
//! `transport.rs`); the authentication that qualifies this consumer lives here
//! and in `barrier/`.
//!
//! Contract every implementer MUST satisfy (and which the barrier separately
//! re-verifies, so a buggy or hostile transport can never force a release):
//!   * deliver AT MOST ONE decision for the armed commit;
//!   * never fall back to a filesystem token;
//!   * honour the shared absolute `deadline_epoch` passed to [`receive`];
//!   * treat truncated / oversized / malformed / EOF / ambiguous input as an
//!     error, never as a release.

use sha2::{Digest, Sha256};

/// The durable truth the barrier computes at the armed commit and requires the
/// delivered decision to be bound to, exactly.  A decision minted for any other
/// source/profile/rest/nonce/state/clock/attempt/deadline must be refused.
/// `source`/`profile` are the frozen identity components of the reviewed run
/// identity and `rest` is the SHA-256 of the ACTUAL durable first (REST)
/// record, not a caller-chosen string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionBinding<'a> {
    pub source: &'a str,
    pub profile: &'a str,
    pub rest: &'a str,
    pub nonce: &'a str,
    pub state: &'a str,
    pub clock: u128,
    pub attempt: usize,
    pub deadline_epoch: u64,
}

/// The single authenticated decision the barrier accepts (exactly one).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Release,
    Abort,
}

impl Decision {
    pub const fn action(self) -> &'static str {
        match self {
            Decision::Release => "release",
            Decision::Abort => "abort",
        }
    }
}

/// A raw, UNVERIFIED decision delivered by a transport.  The barrier re-derives
/// the token and compares every field to the durable [`DecisionBinding`];
/// nothing here is trusted on arrival.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawDecision {
    pub action: String,
    pub nonce_hex: String,
    pub clock: u32,
    pub state_hex: String,
    pub token_hex: String,
    pub source: String,
    pub profile: String,
    pub rest: String,
    pub attempt: usize,
    pub deadline_epoch: u64,
}

/// Fail-closed transport outcomes.  Only `Ok(Some(..))` can lead to acceptance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportError {
    /// Peer closed without delivering a decision.
    Eof,
    /// The shared absolute deadline passed before a decision arrived.
    Expired,
    /// Truncated / oversized / version / reserved / action / clock / encoding fault.
    Malformed,
    /// More than one candidate decision, or a contradictory acknowledgement —
    /// the outcome cannot be classified with certainty.  Constructed only by
    /// test transports exercising the barrier's refusal of ambiguous input.
    #[allow(dead_code)]
    Ambiguous,
    /// A transport I/O failure.
    Io(i32),
}

/// Acknowledgment status bytes identifying ONLY the accepted decision.  The
/// barrier constructs these; the concrete transport serialises them.  They
/// carry no information about attempt 2, completion or cleanup.
pub const ACK_ACCEPTED: u8 = b'A';
pub const ACK_DISARMED: u8 = b'X';
pub const ACK_EXPIRED: u8 = b'E';
pub const ACK_REFUSED: u8 = b'F';

/// Bounded, decision-only acknowledgment.  `token_digest` is SHA-256 over the
/// accepted decision's derived token bytes (all-zero when no decision was
/// accepted), so a release acknowledgment can never be mistaken for abort.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionAck {
    pub status: u8,
    pub action: Option<&'static str>,
    pub nonce_hex: String,
    pub token_digest: [u8; 32],
}

/// SHA-256 over the raw bytes of a 64-hex token; all-zero on a malformed token.
pub fn token_digest(token_hex: &str) -> [u8; 32] {
    match hex32_to_bytes(token_hex) {
        Some(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            hasher.finalize().into()
        }
        None => [0_u8; 32],
    }
}

/// Parse a lowercase-hex64 value into 32 raw bytes, or `None`.
pub fn hex32_to_bytes(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut raw = [0_u8; 32];
    for (index, byte) in raw.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(raw)
}

/// The narrow boundary.  The candidate socket transport and the tests'
/// scripted fakes both implement only this.
pub trait DecisionTransport {
    /// Deliver at most one raw decision, or `Ok(None)` on a clean EOF, under
    /// the shared absolute deadline.  Implementers must not consult the
    /// filesystem.
    fn receive(&mut self, deadline_epoch: u64) -> Result<Option<RawDecision>, TransportError>;
    /// Emit a bounded acknowledgment identifying only the accepted decision
    /// (best effort; the barrier already latched its irreversible decision).
    fn acknowledge(&mut self, ack: &DecisionAck);
    /// Permanently close so no later proposal or re-arm can reuse the endpoint.
    fn close(&mut self);
}
