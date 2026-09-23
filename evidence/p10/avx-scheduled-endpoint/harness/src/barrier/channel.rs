//! Consumption of AT MOST ONE authenticated decision over the narrow
//! transport, authenticated entirely against the durable binding.
//!
//! Every deviation is a fail-closed reason (never a release, never a
//! filesystem fallback).  The `rest` field is compared to the SHA-256 of the
//! ACTUAL durable REST record (computed before the armed receipt was
//! published); `source`/`profile` are compared to the frozen components of
//! the reviewed run identity; the token is re-derived from the durable state
//! hash; the deadline is rechecked at the irreversible acceptance transition;
//! then the channel is closed forever.

use super::Barrier;
use crate::decision::{
    token_digest, Decision, DecisionAck, DecisionBinding, DecisionTransport, TransportError,
    ACK_ACCEPTED, ACK_DISARMED, ACK_EXPIRED, ACK_REFUSED,
};
use crate::transport::SocketTransport;

pub(super) fn consume(
    barrier: &Barrier,
    injected: Option<&mut dyn DecisionTransport>,
    attempt: usize,
    state: &str,
    rest_hash: &str,
) -> Result<Decision, &'static str> {
    let config = barrier
        .channel
        .as_ref()
        .ok_or("barrier_channel_unconfigured")?;
    let binding = DecisionBinding {
        source: &config.source,
        profile: &config.profile,
        rest: rest_hash,
        nonce: &barrier.nonce,
        state,
        clock: barrier.armed_clock,
        attempt,
        deadline_epoch: barrier.deadline_epoch,
    };
    match injected {
        Some(transport) => consume_with(barrier, transport, &binding),
        None => {
            let mut socket = SocketTransport::new(config.fd);
            consume_with(barrier, &mut socket, &binding)
        }
    }
}

/// Fail-closed refusal: acknowledge with the given status, this run's nonce
/// and a ZERO token digest (no decision was accepted, so nothing is
/// identified), then close forever.
fn refuse(
    transport: &mut dyn DecisionTransport,
    status: u8,
    nonce: &str,
    reason: &'static str,
) -> Result<Decision, &'static str> {
    transport.acknowledge(&DecisionAck {
        status,
        action: None,
        nonce_hex: nonce.to_owned(),
        token_digest: [0_u8; 32],
    });
    transport.close();
    Err(reason)
}

fn consume_with(
    barrier: &Barrier,
    transport: &mut dyn DecisionTransport,
    binding: &DecisionBinding<'_>,
) -> Result<Decision, &'static str> {
    let raw = match transport.receive(binding.deadline_epoch) {
        Ok(Some(raw)) => raw,
        // A clean EOF acknowledges nothing (there is nothing to confirm) and
        // still closes the channel permanently.
        Ok(None) => {
            transport.close();
            return Err("barrier_channel_eof");
        }
        Err(TransportError::Expired) => {
            return refuse(transport, ACK_EXPIRED, binding.nonce, "barrier_expired")
        }
        Err(TransportError::Eof) => {
            transport.close();
            return Err("barrier_channel_eof");
        }
        Err(TransportError::Malformed) => {
            return refuse(
                transport,
                ACK_REFUSED,
                binding.nonce,
                "barrier_channel_malformed",
            )
        }
        Err(TransportError::Ambiguous) => {
            return refuse(
                transport,
                ACK_REFUSED,
                binding.nonce,
                "barrier_channel_ambiguous",
            )
        }
        Err(TransportError::Io(_)) => {
            transport.close();
            return Err("barrier_channel_io");
        }
    };
    // Full fail-closed binding verification against the durable truth.  The
    // frame's source/profile/rest are compared to the REVIEWED identity
    // components and the durable REST-record hash, never to each other alone.
    if !matches!(raw.action.as_str(), "release" | "abort") {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_action_malformed",
        );
    }
    if raw.nonce_hex != binding.nonce {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_nonce_mismatch",
        );
    }
    if u128::from(raw.clock) != binding.clock {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_clock_mismatch",
        );
    }
    if raw.state_hex != binding.state {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_state_mismatch",
        );
    }
    if raw.source != binding.source {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_source_mismatch",
        );
    }
    if raw.profile != binding.profile {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_profile_mismatch",
        );
    }
    if raw.rest != binding.rest {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_rest_mismatch",
        );
    }
    if raw.attempt != binding.attempt {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_attempt_mismatch",
        );
    }
    if raw.deadline_epoch != binding.deadline_epoch {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_deadline_mismatch",
        );
    }
    let token = super::token::derive_token(
        &barrier.secret,
        &raw.action,
        binding.nonce,
        binding.clock,
        binding.state,
    );
    if token != raw.token_hex {
        return refuse(
            transport,
            ACK_REFUSED,
            binding.nonce,
            "barrier_token_mismatch",
        );
    }
    // Irreversible acceptance transition: recheck the deadline right here,
    // before the accepted action can take effect.
    if super::durable::now_epoch() >= binding.deadline_epoch {
        return refuse(transport, ACK_EXPIRED, binding.nonce, "barrier_expired");
    }
    let decision = if raw.action == "release" {
        Decision::Release
    } else {
        Decision::Abort
    };
    // Acceptance is latched BEFORE the acknowledgment is emitted; the ack
    // identifies ONLY the accepted action, the binding nonce and the SHA-256
    // of that decision's derived token.  It proves nothing about attempt 2,
    // completion or cleanup.
    transport.acknowledge(&DecisionAck {
        status: match decision {
            Decision::Release => ACK_ACCEPTED,
            Decision::Abort => ACK_DISARMED,
        },
        action: Some(decision.action()),
        nonce_hex: binding.nonce.to_owned(),
        token_digest: token_digest(&token),
    });
    transport.close();
    Ok(decision)
}
