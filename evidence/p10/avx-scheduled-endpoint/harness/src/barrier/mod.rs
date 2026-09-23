//! Authenticated producer-side capture barrier with an opt-in, fail-closed
//! inherited decision channel bound to the reviewed run identity.
//!
//! When (and only when) the supervisor arms it through a complete, exact
//! environment binding, the solver pauses AFTER the durable commit of the
//! armed clock bundle and BEFORE the next attempt begins.  The pause is
//! released only by input the solver can authenticate: the token is
//! re-derived from a shared 32-byte secret, the handshake nonce, the armed
//! clock and the committed state hash, so a stray, forged or replayed file
//! can never start attempt 2.  Every refusal path is fail-closed: missing,
//! malformed, mismatched or late input ends the run without attempting
//! again.
//!
//! The barrier is an explicit one-shot state machine (see [`BarrierPhase`]):
//! after one authenticated release the barrier is permanently `Completed`,
//! so no later commit — at the armed clock or any other clock — can publish
//! another armed receipt, park again, or accept any further decision.
//!
//! Token derivation (identical in the Python supervisor):
//! `token = SHA256(secret || action || '\n' || nonce || '\n' || clock
//!           || '\n' || state_sha256 || '\n' || secret)` in hex.
//!
//! ## Opt-in inherited decision channel (identity-bound)
//!
//! The legacy two-file path is preserved for non-opted (legacy) runs.  The
//! channel is selected ONLY by the canonical child environment AND ONLY when
//! the caller supplies the immutable reviewed run identity
//! ([`Barrier::from_env_with_identity`]): presence of any channel variable
//! without that proof refuses the opt-in (`barrier_channel_identity_unavailable`)
//! — never a fallback.  Admission is additionally restricted to the reviewed
//! N256/M512 capture profile (`barrier_channel_unsupported_profile`) and to
//! the exact reviewed first clock (N256 clock 64,
//! `barrier_channel_clock_unreviewed`); the driver then requires the single
//! first-step decision to have been ACTUALLY consumed before the run may
//! finish ([`Barrier::require_channel_decision_or_unarmed`]).  The parser is presence-aware over ALL barrier
//! variables (invalid Unicode, a lone `CHANNEL_FD`, an invalid number, a
//! missing identity field or any legacy/channel disagreement is a hard
//! refusal, never `Ok(None)` and never filesystem fallback).  When opted in:
//!
//!   * the armed commit STILL reads the actual durable committed state and
//!     STILL publishes the create-only, fsynced armed receipt;
//!   * BEFORE any channel decision the barrier additionally binds the
//!     channel to the ACTUAL durable first (REST) record and to the durable
//!     armed record: the reviewed identity must appear verbatim in both, and
//!     the channel `rest` binding is the SHA-256 of the durable REST record
//!     bytes — not an environment or frame string;
//!   * the *decision wait* consumes AT MOST ONE authenticated decision over
//!     the inherited descriptor via the narrow [`DecisionTransport`]
//!     interface ([`crate::decision`]);
//!   * the barrier performs every authentication itself — exact token
//!     re-derivation plus equality of source/profile/rest/nonce/state/clock/
//!     attempt/deadline to the durable binding — then rechecks the deadline
//!     at the irreversible acceptance transition and closes forever;
//!   * there is NO filesystem-token fallback for an opted-in run: neither
//!     `release.token` nor `abort.token` is ever consulted in channel mode.
//!
//! The concrete socket transport lives behind the narrow interface
//! ([`crate::transport`]) and is a separately-reviewed CANDIDATE; the
//! barrier's acceptance logic does not depend on its correctness.

use crate::{
    artifact,
    decision::{Decision, DecisionTransport},
    error::HarnessError,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::io::RawFd,
    path::{Path, PathBuf},
};

mod channel;
mod durable;
mod env;
mod token;

use durable::now_epoch;
#[cfg(test)]
pub(crate) use env::{
    ABORT_FILE, ARMED_RECEIPT, CHANNEL_FD_ENV, CLOCK_ENV, DEADLINE_ENV, DIR_ENV, NONCE_ENV,
    PROFILE_ENV, RELEASE_FILE, REST_ENV, SECRET_ENV, SOURCE_ENV,
};
#[cfg(not(test))]
use env::{ABORT_FILE, ARMED_RECEIPT, RELEASE_FILE};
#[cfg(test)]
pub use token::derive_token;

/// Explicit one-shot lifecycle of an armed barrier.  Every transition except
/// `Waiting -> Armed` is terminal or self-latching: the barrier can never
/// return to `Waiting`, so a second arming is structurally impossible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BarrierPhase {
    /// No durable commit has yet reached the armed clock.
    Waiting,
    /// The create-only armed receipt is published and a decision is awaited.
    Armed,
    /// One authenticated release was consumed; permanently inert.
    Completed,
    /// An authenticated abort or the absolute deadline ended the barrier.
    Disarmed,
}

/// Selected when the canonical child environment opts into the inherited
/// decision channel AND the caller proved the immutable reviewed run
/// identity.  `source`/`profile` are the frozen identity components (env and
/// frame values are compared against them, never each other alone); `rest` is
/// the SHA-256 of the durable REST record bytes required at consumption;
/// `reviewed_identity` is the full identity string the durable records must
/// carry verbatim.
#[derive(Clone, Debug)]
pub struct ChannelConfig {
    pub fd: RawFd,
    pub source: String,
    pub profile: String,
    pub rest: String,
    pub reviewed_identity: String,
}

/// One armed barrier; `None` (env entirely absent) means the capture path is
/// unchanged.  A partially present binding is a parse-time refusal, never
/// `None`.
#[derive(Debug)]
pub struct Barrier {
    dir: PathBuf,
    nonce: String,
    secret: [u8; 32],
    armed_clock: u128,
    deadline_epoch: u64,
    phase: BarrierPhase,
    channel: Option<ChannelConfig>,
}

impl Barrier {
    /// Legacy-only entry point: parse the complete binding or refuse.  If ANY
    /// channel variable is present the opt-in is REFUSED
    /// (`barrier_channel_identity_unavailable`) because this call supplies no
    /// reviewed-run-identity proof; the legacy two-file path is unaffected.
    #[allow(dead_code)]
    pub fn from_env() -> Result<Option<Self>, HarnessError> {
        env::parse(None)
    }

    /// Parse the complete binding WITH the immutable reviewed run identity
    /// (the actual `RunOwners::identity` string).  Channel opt-in requires
    /// this proof; without it the opt-in is refused, never downgraded.
    pub fn from_env_with_identity(reviewed_identity: &str) -> Result<Option<Self>, HarnessError> {
        env::parse(Some(reviewed_identity))
    }

    /// The current lifecycle phase (observable for tests).
    #[allow(dead_code)]
    pub fn phase(&self) -> BarrierPhase {
        self.phase
    }

    /// Whether this armed barrier opted into the inherited decision channel.
    #[allow(dead_code)]
    pub fn is_channel_mode(&self) -> bool {
        self.channel.is_some()
    }

    /// End-of-run requirement for a channel run: it may only finish after
    /// its single canonical first-step decision has ACTUALLY been consumed
    /// (phase `Completed`).  If any gate never armed — an unreachable clock,
    /// a foreign durable frontier, or any other admission deviation that
    /// somehow reached execution — the run REFUSES instead of finishing.
    /// Legacy (non-channel) runs are unchanged.
    pub fn require_channel_decision_or_unarmed(&self) -> Result<(), HarnessError> {
        if self.channel.is_none() {
            return Ok(());
        }
        match self.phase {
            BarrierPhase::Completed => Ok(()),
            BarrierPhase::Waiting => Err(HarnessError::Barrier("barrier_channel_never_armed")),
            BarrierPhase::Armed => Err(HarnessError::Barrier("barrier_channel_never_resolved")),
            BarrierPhase::Disarmed => Err(HarnessError::Barrier("barrier_disarmed")),
        }
    }

    /// Called after every durable commit.  While `Waiting` it is untouched
    /// until the durable clock first equals the armed clock; then it arms,
    /// parks until authentication, and latches: an authenticated release
    /// makes it permanently `Completed`, while an abort or any refusal leaves
    /// it permanently `Disarmed` (fail-closed).
    pub fn after_commit(
        &mut self,
        output: &Path,
        attempt: usize,
        clock: u128,
    ) -> Result<(), HarnessError> {
        self.commit_gate(output, attempt, clock, None)
    }

    /// Test hook running the SAME real durable-commit gate with an explicitly
    /// supplied decision transport.  Requires opt-in channel mode (a caller
    /// must not inject a transport into a filesystem run).
    #[allow(dead_code)]
    pub fn after_commit_with_transport<T: DecisionTransport>(
        &mut self,
        output: &Path,
        attempt: usize,
        clock: u128,
        transport: &mut T,
    ) -> Result<(), HarnessError> {
        if self.channel.is_none() {
            return Err(HarnessError::Barrier("barrier_channel_unconfigured"));
        }
        self.commit_gate(
            output,
            attempt,
            clock,
            Some(transport as &mut dyn DecisionTransport),
        )
    }

    fn commit_gate(
        &mut self,
        output: &Path,
        attempt: usize,
        clock: u128,
        injected: Option<&mut dyn DecisionTransport>,
    ) -> Result<(), HarnessError> {
        match self.phase {
            BarrierPhase::Completed => return Ok(()),
            BarrierPhase::Disarmed => return Err(HarnessError::Barrier("barrier_disarmed")),
            BarrierPhase::Armed => return Err(HarnessError::Barrier("barrier_reentrant_armed")),
            BarrierPhase::Waiting if clock != self.armed_clock => return Ok(()),
            BarrierPhase::Waiting => {}
        }
        // Channel mode intercepts exactly ONE canonical first-step event:
        // the reviewed first clock committed by attempt 1.  An attempt other
        // than 1 reaching this gate with the armed clock is a schedule the
        // reviewed admission never described — disarm fail-closed.
        if self.channel.is_some() && attempt != 1 {
            self.phase = BarrierPhase::Disarmed;
            return Err(HarnessError::Barrier("barrier_channel_first_step_mismatch"));
        }
        // FROZEN arming path: read the ACTUAL durable committed state.
        let (state, record) = durable::durable_state(output, attempt, clock)?;
        // Channel mode additionally binds the durable REST record and the
        // armed record to the reviewed identity BEFORE the armed receipt is
        // published and long before any channel byte is read.  Missing or
        // foreign durable proof refuses the decision; nothing falls back.
        let rest_hash = if self.channel.is_some() {
            durable::bind_channel_records(output, record.as_str(), self.channel())?
        } else {
            String::new()
        };
        self.publish_armed(attempt, clock, &state)?;
        self.phase = BarrierPhase::Armed;
        println!(
            "temporal_barrier armed attempt={attempt} clock={clock} state_sha256={state} deadline={}",
            self.deadline_epoch
        );
        // Decision consumption.  Opt-in channel mode NEVER reads a filesystem
        // token; the legacy two-file path exists only when not opted in.
        if self.channel.is_some() {
            let decision = channel::consume(self, injected, attempt, &state, &rest_hash);
            match decision {
                Ok(Decision::Release) => {
                    println!(
                        "temporal_barrier released attempt={attempt} clock={}",
                        self.armed_clock
                    );
                    self.phase = BarrierPhase::Completed;
                    Ok(())
                }
                Ok(Decision::Abort) => {
                    self.phase = BarrierPhase::Disarmed;
                    Err(HarnessError::Barrier("barrier_aborted_by_supervisor"))
                }
                Err(reason) => {
                    self.phase = BarrierPhase::Disarmed;
                    Err(HarnessError::Barrier(reason))
                }
            }
        } else {
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
    }

    fn channel(&self) -> &ChannelConfig {
        self.channel
            .as_ref()
            .unwrap_or_else(|| unreachable!("caller checked channel mode"))
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
        let release = token::release_token(self, state);
        let abort = token::abort_token(self, state);
        loop {
            if now_epoch() >= self.deadline_epoch {
                return Err(HarnessError::Barrier("barrier_expired"));
            }
            if token::read_token(&self.dir.join(ABORT_FILE)).as_deref() == Some(abort.as_str()) {
                return Err(HarnessError::Barrier("barrier_aborted_by_supervisor"));
            }
            if token::read_token(&self.dir.join(RELEASE_FILE)).as_deref() == Some(release.as_str())
            {
                println!(
                    "temporal_barrier released attempt=1 clock={}",
                    self.armed_clock
                );
                return Ok(());
            }
            std::thread::sleep(env::POLL_INTERVAL);
        }
    }
}

#[cfg(test)]
mod fixtures;
#[cfg(all(test, capture_offline))]
pub(crate) use fixtures::env_guard;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_channel;
#[cfg(test)]
mod tests_channel_refusal;
#[cfg(test)]
mod tests_parser;
