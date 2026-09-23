//! Integration tests for the REAL `RunOwners::execute`/`attempt` dispatch
//! under the N256 capture profile.  These replace the sealed review's
//! rejected synthetic attempt model: the SAME generic driver code paths that
//! production uses are exercised end-to-end (REST publication, per-attempt
//! `try_advance` against a real `AttemptWorkspace`, transactional commit,
//! step-bundle publication, terminal marker, and barrier arming after the
//! first durable commit) with a safe, non-numerical zero right-hand side.

use super::{barrier, config, schedule, RunOwners};
use crate::publication::Frontiers;
use crate::timed_rhs::{ProviderFacts, TimedRhs};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        kernel::{RhsBounds, RightHandSide},
        transaction::CandidateState,
    },
    Complex64, SolverError,
};
use std::{fs, path::PathBuf};

/// A zero right-hand side with declared, bounded costs: the physical steady
/// state of zero velocity is a true solution, so every attempt commits with
/// zero error indicators through the library's real acceptance machinery.
pub(crate) struct StubRhs;

impl RightHandSide for StubRhs {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 1,
        })
    }

    fn begin_attempt(&mut self, _clock: TickClock, _ticks: u128) -> Result<(), SolverError> {
        Ok(())
    }

    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for component in output {
            for value in component.iter_mut() {
                *value = Complex64::new(0.0, 0.0);
            }
        }
        Ok(())
    }
}

impl ProviderFacts for TimedRhs<StubRhs> {
    fn hit_miss(&self) -> [usize; 2] {
        [0, 0]
    }
}

fn tiny_plan() -> ResourcePlan {
    ResourcePlan::new(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 1 << 20,
            overhead: 1 << 20,
        },
        4 << 20,
        Epoch(0),
    )
    .unwrap()
}

pub(crate) fn tiny_run() -> RunOwners<StubRhs> {
    let plan = tiny_plan();
    // The production tick clock (owners::states) with target slack above the
    // endpoint, exactly as every real N256 run restores it.
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    RunOwners {
        resources: plan,
        state: SpectralState::from_rest(plan, clock, Epoch(0)).unwrap(),
        candidate: CandidateState::new(plan, clock, Epoch(0)).unwrap(),
        attempts: AttemptWorkspace::new(plan).unwrap(),
        rhs: TimedRhs::new(StubRhs),
        observer: None,
        identity: config::identity(),
        balances: Vec::new(),
        frontiers: Frontiers::default(),
    }
}

pub(crate) fn temp_root(label: &str) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "p10-runowners-{label}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn real_execute_dispatch_commits_the_full_schedule_through_production_paths() {
    let root = temp_root("full");
    let output = root.join("out");
    fs::create_dir(&output).unwrap();
    let mut run = tiny_run();
    let identity = run.identity.clone();
    run.execute(&output).expect("real dispatch completes");
    // The metadata-only REST record is the FIRST publication and carries the
    // ACTUAL run identity (this is the durable object the channel binds to).
    let rest = fs::read_to_string(output.join("rest.json")).unwrap();
    assert!(rest.contains(&identity));
    // The final scheduled attempt published its durable bundle with the
    // identity and a real state hash, and the terminal marker closed the run.
    let last_bundle = output.join(format!(
        "step-{:03}-clock-{:04}",
        schedule::MAXIMUM_ATTEMPTS,
        schedule::ENDPOINT
    ));
    let record = fs::read_to_string(last_bundle.join("record.json")).unwrap();
    assert!(record.contains(&identity));
    assert!(record.contains("\"state_sha256\""));
    assert!(output.join("endpoint-capture-complete.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn armed_barrier_intercepts_the_first_real_durable_commit() {
    let _guard = barrier::env_guard();
    let root = temp_root("barrier");
    let output = root.join("out");
    fs::create_dir(&output).unwrap();
    let barrier_dir = temp_root("barrier-dir");
    let first_clock = schedule::step(0).unwrap();
    // Arm the canonical barrier at the FIRST committed clock, two seconds
    // ahead of now: the real execute loop must publish the durable bundle,
    // ARM (durable reads + create-only receipt) and then fail closed at the
    // absolute deadline with no token in sight.
    std::env::set_var(barrier::DIR_ENV, &barrier_dir);
    std::env::set_var(barrier::NONCE_ENV, "a".repeat(64));
    std::env::set_var(barrier::SECRET_ENV, "b".repeat(64));
    std::env::set_var(barrier::CLOCK_ENV, first_clock.to_string());
    std::env::set_var(barrier::DEADLINE_ENV, (barrier_now() + 2).to_string());
    let error = {
        let mut run = tiny_run();
        let error = run
            .execute(&output)
            .expect_err("expired barrier must stop the run");
        assert!(
            error.to_string().contains("barrier_expired"),
            "expected barrier_expired, got {error}"
        );
        error
    };
    std::env::remove_var(barrier::DIR_ENV);
    std::env::remove_var(barrier::NONCE_ENV);
    std::env::remove_var(barrier::SECRET_ENV);
    std::env::remove_var(barrier::CLOCK_ENV);
    std::env::remove_var(barrier::DEADLINE_ENV);
    let _ = error;
    // The armed commit was REALLY durable: step bundle + armed receipt exist
    // with the actual run identity and hash before the refusal happened.
    let bundle = output.join(format!("step-001-clock-{first_clock:04}"));
    let record = fs::read_to_string(bundle.join("record.json")).unwrap();
    assert!(record.contains(&config::identity()));
    let receipt =
        fs::read_to_string(barrier_dir.join(barrier::ARMED_RECEIPT)).expect("armed receipt");
    assert!(receipt.contains(&format!("\"clock\": {first_clock}")));
    assert!(!output.join("endpoint-capture-complete.json").exists());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(barrier_dir).unwrap();
}

pub(crate) fn barrier_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

// ---- real-inherited-socket end-to-end over the child execution path -------

// The reviewed channel protocol exists ONLY in the N256/M512 binary; the
// real-inherited-socket end-to-end (and its private helpers) therefore run
// only in the reviewed profile.  The N512 build instead proves refusal
// (run_owners_n512_deny_test / unreachable_channel_clock_*).
#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn child_solve(output: &std::path::Path) -> ! {
    let mut run = tiny_run();
    match run.execute(output) {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("child: {error}");
            std::process::exit(7);
        }
    }
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn child_mode() -> Option<String> {
    std::env::var("P10_CHILD_MODE").ok()
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn hex64_field(text: &str, marker: &str) -> String {
    let start = text.find(marker).unwrap() + marker.len();
    text[start..start + 64].to_owned()
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn number_field(text: &str, key: &str) -> u64 {
    let marker = format!("\"{key}\": ");
    let start = text.find(&marker).unwrap() + marker.len();
    let end = start
        + text[start..]
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or_else(|| text[start..].len());
    text[start..end].parse().unwrap()
}

pub(crate) fn sha256_hex(path: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn identity_component(identity: &str, key: &str) -> String {
    let marker = format!("{key}=");
    for field in identity.split(';') {
        if let Some(value) = field.strip_prefix(&marker) {
            return value.to_owned();
        }
    }
    panic!("identity lacks {key}");
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn socket_channel_end_to_end(action: &str, child_test_name: &str) {
    // Child-mode branch: this process IS the solver child.
    if let Some(mode) = child_mode() {
        assert_eq!(mode, action);
        child_solve(std::path::Path::new(
            &std::env::var("P10_CHILD_OUTPUT").unwrap(),
        ));
    }
    let identity = config::identity();
    let root = temp_root(&format!("sock-{action}"));
    let barrier_dir = root.join("barrier");
    let child_output = root.join("child-out");
    fs::create_dir_all(&barrier_dir).unwrap();
    fs::create_dir(&child_output).unwrap();
    // The REST record bytes are deterministic for a given identity; publish
    // into a private root to learn the durable hash the child must bind.
    let rest_probe = root.join("rest-probe");
    fs::create_dir(&rest_probe).unwrap();
    crate::step_artifact::publish_rest(&rest_probe, &identity).unwrap();
    let rest_hash = sha256_hex(&rest_probe.join("rest.json"));
    // Kernel socketpair; the child-side descriptor is inherited (not CLOEXEC).
    let mut fds = [0_i32; 2];
    // SAFETY: standard socketpair into a valid 2-element array.
    let rc = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    };
    assert_eq!(rc, 0, "socketpair");
    let (parent_fd, child_fd) = (fds[0], fds[1]);
    // SAFETY: clear CLOEXEC on the child descriptor for inheritance.
    assert_eq!(unsafe { libc::fcntl(child_fd, libc::F_SETFD, 0) }, 0);
    let clock = crate::schedule::step(0).unwrap();
    let deadline = barrier_now() + 90;
    let nonce = "e".repeat(64);
    let secret = "c".repeat(64);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(child_test_name)
        .arg("--nocapture")
        .env("P10_CHILD_MODE", action)
        .env("P10_CHILD_OUTPUT", &child_output)
        .env(barrier::DIR_ENV, &barrier_dir)
        .env(barrier::NONCE_ENV, &nonce)
        .env(barrier::SECRET_ENV, &secret)
        .env(barrier::CLOCK_ENV, clock.to_string())
        .env(barrier::DEADLINE_ENV, deadline.to_string())
        .env(barrier::CHANNEL_FD_ENV, child_fd.to_string())
        .env(barrier::SOURCE_ENV, identity_component(&identity, "source"))
        .env(
            barrier::PROFILE_ENV,
            identity_component(&identity, "profile"),
        )
        .env(barrier::REST_ENV, &rest_hash)
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("spawn solver child");
    // Supervisor: wait for the REAL armed receipt (durable commit proof).
    let receipt_path = barrier_dir.join(barrier::ARMED_RECEIPT);
    let receipt = wait_for_file(&receipt_path);
    let attempt = number_field(&receipt, "attempt");
    let receipt_clock = number_field(&receipt, "clock");
    let receipt_nonce = hex64_field(&receipt, "\"nonce\": \"");
    let receipt_state = hex64_field(&receipt, "\"state_sha256\": \"");
    assert_eq!(receipt_nonce, nonce);
    assert_eq!(receipt_clock as u128, clock);
    // Send exactly one authenticated decision over the inherited endpoint.
    let token = barrier::derive_token(
        &nonce_bytes(&secret).try_into().unwrap(),
        action,
        &nonce,
        clock,
        &receipt_state,
    );
    let frame = crate::wire::encode_decision(
        action,
        &nonce,
        receipt_clock as u32,
        &receipt_state,
        &token,
        attempt as usize,
        deadline,
        &identity_component(&identity, "source"),
        &identity_component(&identity, "profile"),
        &rest_hash,
    )
    .unwrap();
    // SAFETY: SEQPACKET write of one complete frame on the parent endpoint.
    let sent = unsafe {
        libc::send(
            parent_fd,
            frame.as_ptr() as *const libc::c_void,
            frame.len(),
            0,
        )
    };
    assert_eq!(sent as usize, frame.len(), "decision send");
    // Read the one decision-scoped acknowledgment.
    let mut ack = vec![0_u8; crate::wire::ACK_LEN];
    // SAFETY: bounded blocking recv of the fixed-size ack datagram.
    let got = unsafe {
        libc::recv(
            parent_fd,
            ack.as_mut_ptr() as *mut libc::c_void,
            ack.len(),
            0,
        )
    };
    assert_eq!(got as usize, crate::wire::ACK_LEN, "ack recv");
    assert_eq!(&ack[..8], crate::wire::ACK_MAGIC);
    assert_eq!(
        ack[8],
        if action == "release" {
            crate::decision::ACK_ACCEPTED
        } else {
            crate::decision::ACK_DISARMED
        }
    );
    assert_eq!(ack[10..42], nonce_bytes(&nonce)[..], "ack nonce");
    assert_eq!(
        ack[42..74],
        crate::decision::token_digest(&token)[..],
        "ack token digest"
    );
    // SAFETY: the socket is fully consumed once the child exits.
    unsafe { libc::close(parent_fd) };
    let done = child.wait_with_output().expect("reap solver child");
    assert!(
        done.status.success() == (action == "release"),
        "child exit for {action}: {:?}",
        String::from_utf8_lossy(&done.stderr)
    );
    // Exactly one armed receipt ever; the endpoint marker reflects the action.
    assert_eq!(
        fs::read_dir(&barrier_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("barrier-armed"))
            .count(),
        1
    );
    if action == "release" {
        assert!(child_output.join("endpoint-capture-complete.json").exists());
    } else {
        assert!(!child_output.join("endpoint-capture-complete.json").exists());
        assert!(child_output
            .join(format!("step-001-clock-{clock:04}"))
            .exists());
    }
    // SAFETY: release the child-side descriptor kept open in this parent.
    unsafe { libc::close(child_fd) };
    fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn nonce_bytes(nonce_hex: &str) -> Vec<u8> {
    nonce_hex
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
fn wait_for_file(path: &std::path::Path) -> String {
    for _ in 0..600 {
        if let Ok(text) = fs::read_to_string(path) {
            return text;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("armed receipt never appeared: {path:?}");
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
#[test]
fn socket_channel_release_completes_child_process() {
    socket_channel_end_to_end(
        "release",
        "run_owners_test::socket_channel_release_completes_child_process",
    );
}

#[cfg(feature = "n256-m512-piecewise-cadv33")]
#[test]
fn socket_channel_abort_stops_child_process() {
    socket_channel_end_to_end(
        "abort",
        "run_owners_test::socket_channel_abort_stops_child_process",
    );
}
