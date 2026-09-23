//! Canonical token derivation and filesystem-token reading (frozen bytes).
use super::Barrier;
use crate::error::HarnessError;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub const HEX64: usize = 64;
const MAX_TOKEN_BYTES: usize = 256;

pub fn derive_token(
    secret: &[u8; 32],
    action: &str,
    nonce: &str,
    clock: u128,
    state: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(action.as_bytes());
    hasher.update(b"\n");
    hasher.update(nonce.as_bytes());
    hasher.update(b"\n");
    hasher.update(clock.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(state.as_bytes());
    hasher.update(b"\n");
    hasher.update(secret);
    to_hex(&hasher.finalize())
}

pub(super) fn release_token(barrier: &Barrier, state: &str) -> String {
    derive_token(
        &barrier.secret,
        "release",
        &barrier.nonce,
        barrier.armed_clock,
        state,
    )
}

pub(super) fn abort_token(barrier: &Barrier, state: &str) -> String {
    derive_token(
        &barrier.secret,
        "abort",
        &barrier.nonce,
        barrier.armed_clock,
        state,
    )
}

pub(super) fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0xf)]));
    }
    text
}

pub(super) fn is_hex64(text: &str) -> bool {
    text.len() == HEX64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn parse_secret_env(value: &str) -> Result<[u8; 32], HarnessError> {
    if !is_hex64(value) {
        return Err(HarnessError::Barrier("barrier_secret_malformed"));
    }
    let raw = hex::decode_lossy(value);
    let Some(raw) = raw else {
        return Err(HarnessError::Barrier("barrier_secret_malformed"));
    };
    let mut secret = [0_u8; 32];
    secret.copy_from_slice(&raw);
    Ok(secret)
}

pub(super) mod hex {
    pub fn decode_lossy(text: &str) -> Option<[u8; 32]> {
        let mut raw = [0_u8; 32];
        for (index, byte) in raw.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(raw)
    }
}

/// A valid token is exactly one lowercase 64-hex line within 256 bytes;
/// anything else reads as absent so the barrier never trusts malformed input.
pub(super) fn read_token(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_TOKEN_BYTES as u64 {
        return None;
    }
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    let line = text.strip_suffix('\n').unwrap_or(&text);
    if line.is_empty() || line.contains('\n') || !is_hex64(line) {
        return None;
    }
    Some(line.to_owned())
}
