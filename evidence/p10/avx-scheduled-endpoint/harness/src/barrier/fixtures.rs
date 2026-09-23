//! Shared barrier-test scaffolding: serialized environment access, unique
//! temporary directories, the fake decision transport, and setups whose
//! durable records carry the reviewed run identity exactly as the solver's
//! own transactional publications do.

use super::*;
use crate::decision::{DecisionAck, DecisionTransport, RawDecision, TransportError};
use std::sync::Mutex;

/// The canonical environment parser reads process-global variables, so every
/// test that touches them must serialise to avoid cross-test interference.
static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn temp_dir(label: &str) -> PathBuf {
    let nonce = durable::now_epoch();
    let path = std::env::temp_dir().join(format!(
        "p10-barrier-{label}-{}-{nonce}-{}",
        std::process::id(),
        thread_seq()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

thread_local! {
    static SEQ: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
fn thread_seq() -> u64 {
    SEQ.with(|seq| {
        let value = seq.get();
        seq.set(value + 1);
        value
    })
}

/// Identity components the channel binds against.  `PROF` MUST equal the
/// compiled profile constant — the parser refuses any other profile.
pub(super) const SRC: &str = "p10-channel-parser-source";
pub(super) const PROF: &str = crate::config::PROFILE;

pub(super) fn reviewed_identity() -> String {
    format!("source={SRC};case=fixture;profile={PROF};schema=barrier-fixture-v1")
}

#[derive(Default)]
pub(super) struct FakeTransport {
    pub script: Vec<Result<Option<RawDecision>, TransportError>>,
    pub acks: Vec<DecisionAck>,
    pub receive_calls: usize,
    pub closed: usize,
}

impl DecisionTransport for FakeTransport {
    fn receive(&mut self, _deadline_epoch: u64) -> Result<Option<RawDecision>, TransportError> {
        self.receive_calls += 1;
        if self.script.is_empty() {
            return Ok(None);
        }
        self.script.remove(0)
    }
    fn acknowledge(&mut self, ack: &DecisionAck) {
        self.acks.push(ack.clone());
    }
    fn close(&mut self) {
        self.closed += 1;
    }
}

pub(super) fn channel_barrier(
    dir: &Path,
    nonce: &str,
    secret: [u8; 32],
    armed_clock: u128,
    deadline_epoch: u64,
    rest: &str,
) -> Barrier {
    Barrier {
        dir: dir.to_path_buf(),
        nonce: nonce.to_string(),
        secret,
        armed_clock,
        deadline_epoch,
        phase: BarrierPhase::Waiting,
        channel: Some(ChannelConfig {
            fd: -1,
            source: SRC.to_string(),
            profile: PROF.to_string(),
            rest: rest.to_string(),
            reviewed_identity: reviewed_identity(),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn frame(
    secret: &[u8; 32],
    action: &str,
    nonce: &str,
    clock: u128,
    state: &str,
    attempt: usize,
    deadline_epoch: u64,
    rest: &str,
) -> RawDecision {
    frame_with(
        secret,
        action,
        nonce,
        clock,
        state,
        attempt,
        deadline_epoch,
        SRC,
        PROF,
        rest,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn frame_with(
    secret: &[u8; 32],
    action: &str,
    nonce: &str,
    clock: u128,
    state: &str,
    attempt: usize,
    deadline_epoch: u64,
    source: &str,
    profile: &str,
    rest: &str,
) -> RawDecision {
    RawDecision {
        action: action.to_string(),
        nonce_hex: nonce.to_string(),
        clock: clock as u32,
        state_hex: state.to_string(),
        token_hex: token::derive_token(secret, action, nonce, clock, state),
        source: source.to_string(),
        profile: profile.to_string(),
        rest: rest.to_string(),
        attempt,
        deadline_epoch,
    }
}

pub(super) struct Setup {
    pub dir: PathBuf,
    pub output: PathBuf,
    pub nonce: String,
    pub secret: [u8; 32],
    pub state: String,
    pub clock: u128,
    pub deadline: u64,
    /// SHA-256 of the durable REST record bytes, exactly as the channel must
    /// bind it (env value, armed-record equality, frame value).
    pub rest: String,
}

/// A durable-commit-shaped output tree: the armed step bundle and the REST
/// record BOTH carry the reviewed run identity verbatim, the way the solver's
/// transactional publications do.  `rest` is the hash of the REST record
/// bytes as they exist on disk after this call.
pub(super) fn armed_setup(label: &str) -> Setup {
    let dir = temp_dir(&format!("{label}-dir"));
    let output = temp_dir(&format!("{label}-out"));
    let secret = [11_u8; 32];
    let nonce = "c".repeat(64);
    let state = "a".repeat(64);
    let clock = 32;
    let deadline = durable::now_epoch() + 60;
    let identity = reviewed_identity();
    let bundle = output.join(format!("step-001-clock-{clock:04}"));
    fs::create_dir_all(&bundle).unwrap();
    fs::write(
        bundle.join("record.json"),
        format!("{{\n  \"identity\": \"{identity}\",\n  \"state_sha256\": \"{state}\"\n}}\n"),
    )
    .unwrap();
    let rest_record = format!(
        "{{\n  \"identity\": \"{identity}\",\n  \"clock\": 0,\n  \"qualification\": false\n}}\n"
    );
    let rest_path = output.join(durable::REST_RECORD);
    if let Some(parent) = rest_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&rest_path, &rest_record).unwrap();
    let rest = durable::file_sha256(&rest_path).unwrap();
    Setup {
        dir,
        output,
        nonce,
        secret,
        state,
        clock,
        deadline,
        rest,
    }
}

pub(super) fn cleanup(setup: &Setup) {
    fs::remove_dir_all(&setup.dir).unwrap();
    fs::remove_dir_all(&setup.output).unwrap();
}

pub(super) fn reason(error: HarnessError) -> String {
    error.to_string()
}
