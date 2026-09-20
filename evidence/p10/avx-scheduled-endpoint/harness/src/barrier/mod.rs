//! Authenticated producer-side capture barrier.
//!
//! When (and only when) the supervisor arms it through a complete, exact
//! environment binding, the solver pauses AFTER the durable commit of the
//! armed clock bundle and BEFORE the next attempt begins.  The pause is
//! released only by a token the solver can authenticate: it re-derives the
//! HMAC-like SHA-256 token from a shared 32-byte secret (environment), the
//! handshake nonce, the armed clock and the committed state hash, so a stray,
//! forged or replayed file can never start attempt 2.  Every refusal path is
//! fail-closed: missing, malformed, mismatched or late input keeps the solver
//! parked until the absolute deadline and then exits without attempting again.
//!
//! The barrier is an explicit one-shot state machine (see [`BarrierPhase`]):
//! after one authenticated release the barrier is permanently `Completed`,
//! so no later commit — at the armed clock or any other clock — can publish
//! another armed receipt, park again, or accept any further token.
//!
//! Token derivation (identical in the Python supervisor):
//! `token = SHA256(secret || action || '\n' || nonce || '\n' || clock
//!           || '\n' || state_sha256 || '\n' || secret)` in hex.

use crate::{artifact, error::HarnessError, schedule};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const DIR_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_DIR";
pub const NONCE_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_NONCE";
pub const SECRET_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_SECRET";
pub const CLOCK_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_CLOCK";
pub const DEADLINE_ENV: &str = "NSBU_N512_TEMPORAL_BARRIER_DEADLINE";
pub const ARMED_RECEIPT: &str = "barrier-armed.json";
pub const RELEASE_FILE: &str = "release.token";
pub const ABORT_FILE: &str = "abort.token";

const HEX64: usize = 64;
const MAX_TOKEN_BYTES: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn derive_token(secret: &[u8; 32], action: &str, nonce: &str, clock: u128, state: &str) -> String {
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

fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0xf)]));
    }
    text
}

fn is_hex64(text: &str) -> bool {
    text.len() == HEX64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_secret_env(value: &str) -> Result<[u8; 32], HarnessError> {
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

mod hex {
    pub fn decode_lossy(text: &str) -> Option<[u8; 32]> {
        let mut raw = [0_u8; 32];
        for (index, byte) in raw.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(raw)
    }
}

/// Explicit one-shot lifecycle of an armed barrier.  Every transition except
/// `Waiting -> Armed` is terminal or self-latching: the barrier can never
/// return to `Waiting`, so a second arming is structurally impossible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BarrierPhase {
    /// No durable commit has yet reached the armed clock.
    Waiting,
    /// The create-only armed receipt is published and a token is being awaited.
    Armed,
    /// One authenticated release was consumed; permanently inert.
    Completed,
    /// An authenticated abort or the absolute deadline ended the barrier.
    Disarmed,
}

/// One armed barrier; `None` (env absent) means the capture path is unchanged.
pub struct Barrier {
    dir: PathBuf,
    nonce: String,
    secret: [u8; 32],
    armed_clock: u128,
    deadline_epoch: u64,
    phase: BarrierPhase,
}

impl Barrier {
    /// Parse the complete binding or refuse; a partially armed barrier is a
    /// configuration error, never a silent fall-through.  Absent directory
    /// with every other variable absent is the ordinary (unarmed) path.
    pub fn from_env() -> Result<Option<Self>, HarnessError> {
        let dir = match std::env::var(DIR_ENV) {
            Ok(dir) => dir,
            Err(_) => {
                for key in [NONCE_ENV, SECRET_ENV, CLOCK_ENV, DEADLINE_ENV] {
                    if std::env::var(key).is_ok() {
                        return Err(HarnessError::Barrier("barrier_partial_environment"));
                    }
                }
                return Ok(None);
            }
        };
        let nonce = std::env::var(NONCE_ENV).map_err(|_| HarnessError::Barrier("barrier_nonce_missing"))?;
        if !is_hex64(&nonce) {
            return Err(HarnessError::Barrier("barrier_nonce_malformed"));
        }
        let secret = parse_secret_env(
            &std::env::var(SECRET_ENV).map_err(|_| HarnessError::Barrier("barrier_secret_missing"))?,
        )?;
        let armed_clock = std::env::var(CLOCK_ENV)
            .ok()
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|value| value.parse::<u128>().ok())
            .filter(|&clock| clock > 0 && clock < schedule::ENDPOINT)
            .ok_or(HarnessError::Barrier("barrier_clock_malformed"))?;
        let deadline_epoch = std::env::var(DEADLINE_ENV)
            .ok()
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|&deadline| deadline > now_epoch())
            .ok_or(HarnessError::Barrier("barrier_deadline_not_future"))?;
        if !Path::new(&dir).is_dir() {
            return Err(HarnessError::Barrier("barrier_dir_missing"));
        }
        Ok(Some(Self {
            dir: PathBuf::from(dir),
            nonce,
            secret,
            armed_clock,
            deadline_epoch,
            phase: BarrierPhase::Waiting,
        }))
    }

    /// Called after every durable commit.  While `Waiting` it is untouched
    /// until the durable clock first equals the armed clock; then it arms,
    /// parks until authentication, and latches: an authenticated release
    /// makes it permanently `Completed` (immediate `Ok`, no receipt, no park,
    /// no token accepted for every later commit), while an abort or expiry
    /// leaves it permanently `Disarmed` (fail-closed refusal).
    pub fn after_commit(&mut self, output: &Path, attempt: usize, clock: u128) -> Result<(), HarnessError> {
        match self.phase {
            BarrierPhase::Completed => return Ok(()),
            BarrierPhase::Disarmed => return Err(HarnessError::Barrier("barrier_disarmed")),
            BarrierPhase::Armed => return Err(HarnessError::Barrier("barrier_reentrant_armed")),
            BarrierPhase::Waiting if clock != self.armed_clock => return Ok(()),
            BarrierPhase::Waiting => {}
        }
        let state = durable_state_sha(output, attempt, clock)?;
        self.publish_armed(attempt, clock, &state)?;
        self.phase = BarrierPhase::Armed;
        println!(
            "temporal_barrier armed attempt={attempt} clock={clock} state_sha256={state} deadline={}",
            self.deadline_epoch
        );
        match self.wait_for_token(&state) {
            Ok(()) => {
                self.phase = BarrierPhase::Completed;
                Ok(())
            }
            Err(error) => {
                self.phase = BarrierPhase::Disarmed;
                Err(error)
            }
        }
    }

    fn publish_armed(&self, attempt: usize, clock: u128, state: &str) -> Result<(), HarnessError> {
        debug_assert_eq!(self.phase, BarrierPhase::Waiting);
        let receipt = format!(
            concat!(
                "{{\n  \"schema\": \"p10-avx-n512-temporal-barrier-armed-v1\",\n",
                "  \"attempt\": {},\n  \"clock\": {},\n  \"nonce\": {},\n",
                "  \"state_sha256\": {},\n  \"deadline_epoch\": {},\n",
                "  \"qualification\": false\n}}\n"
            ),
            attempt,
            clock,
            artifact::json_string(&self.nonce),
            artifact::json_string(state),
            self.deadline_epoch,
        );
        let path = self.dir.join(ARMED_RECEIPT);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| HarnessError::Barrier("barrier_armed_receipt_exists"))?;
        file.write_all(receipt.as_bytes())?;
        file.sync_all()?;
        // The armed receipt is the supervisor's durability signal: the file
        // bytes AND the directory entry must both survive a host crash, so
        // the containing directory is fsynced before the solver parks.
        fs::File::open(&self.dir)
            .map_err(|_| HarnessError::Barrier("barrier_armed_receipt_dir_unopenable"))?
            .sync_all()
            .map_err(|_| HarnessError::Barrier("barrier_armed_receipt_dir_unsynced"))?;
        Ok(())
    }

    fn wait_for_token(&self, state: &str) -> Result<(), HarnessError> {
        let release = derive_token(&self.secret, "release", &self.nonce, self.armed_clock, state);
        let abort = derive_token(&self.secret, "abort", &self.nonce, self.armed_clock, state);
        loop {
            if now_epoch() >= self.deadline_epoch {
                return Err(HarnessError::Barrier("barrier_expired"));
            }
            if read_token(&self.dir.join(ABORT_FILE)).as_deref() == Some(abort.as_str()) {
                return Err(HarnessError::Barrier("barrier_aborted_by_supervisor"));
            }
            if read_token(&self.dir.join(RELEASE_FILE)).as_deref() == Some(release.as_str()) {
                println!("temporal_barrier released attempt=1 clock={}", self.armed_clock);
                return Ok(());
            }
            sleep(POLL_INTERVAL);
        }
    }
}

/// A valid token is exactly one lowercase 64-hex line within 256 bytes;
/// anything else reads as absent so the barrier never trusts malformed input.
fn read_token(path: &Path) -> Option<String> {
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

fn durable_state_sha(output: &Path, attempt: usize, clock: u128) -> Result<String, HarnessError> {
    let bundle = output.join(format!("step-{attempt:03}-clock-{clock:04}"));
    if !bundle.is_dir() {
        return Err(HarnessError::Barrier("barrier_bundle_not_durable"));
    }
    let record = fs::read_to_string(bundle.join("record.json"))
        .map_err(|_| HarnessError::Barrier("barrier_record_unreadable"))?;
    record_state_sha(&record)
}

fn record_state_sha(record: &str) -> Result<String, HarnessError> {
    let marker = "\"state_sha256\": \"";
    let start = record
        .find(marker)
        .ok_or(HarnessError::Barrier("barrier_record_state_missing"))?
        + marker.len();
    let end = start + HEX64;
    let candidate = record
        .get(start..end)
        .ok_or(HarnessError::Barrier("barrier_record_state_missing"))?;
    if !is_hex64(candidate) || record[end..].find(marker).is_some() {
        return Err(HarnessError::Barrier("barrier_record_state_malformed"));
    }
    Ok(candidate.to_owned())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
