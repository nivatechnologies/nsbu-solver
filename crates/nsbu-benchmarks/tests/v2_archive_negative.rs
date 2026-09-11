//! Adversarial semantic and resource-bound checks for exact-v2 archives.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{archive, Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};
use sha2::{Digest, Sha256};

fn settings(method: Method) -> Settings {
    Settings {
        domain: Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method,
            limits: RunLimits {
                endpoint: 256,
                step_ticks: 128,
                maximum_attempts: 2,
            },
            tolerances: Tolerances {
                absolute: [1.0; 2],
                relative: [0.0; 2],
            },
        },
        advective_limit: 0.3,
    }
}
fn payload() -> (Plan, Run, Vec<u8>) {
    let plan = Plan::from_rest(settings(Method::CoxMatthews), 1 << 26).unwrap();
    let mut run = Run::from_rest(plan).unwrap();
    run.step().unwrap();
    let n = archive::encoded_len(&run).unwrap();
    let mut bytes = vec![0; n];
    archive::write(&run, &mut bytes).unwrap();
    (plan, run, bytes)
}
fn rehash(bytes: &mut [u8]) {
    let n = bytes.len();
    let hash = Sha256::digest(&bytes[..n - 32]);
    bytes[n - 32..].copy_from_slice(&hash);
}

#[test]
fn terminal_rejection_and_refusal_are_preserved() {
    let mut reject = settings(Method::CoxMatthews);
    reject.configuration.tolerances.absolute = [1e-40; 2];
    let reject_plan = Plan::from_rest(reject, 1 << 26).unwrap();
    let mut rejected = Run::from_rest(reject_plan).unwrap();
    let rejected_outcome = rejected.step().unwrap();
    assert!(matches!(
        rejected_outcome,
        nsbu_solver::experiment::control::Outcome::Rejected(_)
    ));
    let mut refusal = settings(Method::CoxMatthews);
    refusal.advective_limit = 1e-30;
    let refusal_plan = Plan::from_rest(refusal, 1 << 26).unwrap();
    let mut refused = Run::from_rest(refusal_plan).unwrap();
    let refused_outcome = refused.step().unwrap();
    assert!(matches!(
        refused_outcome,
        nsbu_solver::experiment::control::Outcome::Refused { .. }
    ));
    for (plan, run) in [(reject_plan, rejected), (refusal_plan, refused)] {
        let n = archive::encoded_len(&run).unwrap();
        let mut bytes = vec![0; n];
        archive::write(&run, &mut bytes).unwrap();
        let imported = archive::read(&bytes, plan, n, 1 << 26).unwrap();
        assert_eq!(imported.work(), run.work());
        assert_eq!(imported.history().records().len(), 1);
        let mut resumed = imported.continue_unverified(1 << 26).unwrap();
        assert!(resumed.step().is_err());
        assert_eq!(resumed.state().clock(), run.state().clock());
    }
}

#[test]
fn foreign_plans_and_reservation_caps_are_rejected() {
    let (plan, _run, bytes) = payload();
    assert!(archive::read(
        &bytes,
        plan,
        bytes.len(),
        archive::read_reservation(plan, 1).unwrap() - 1
    )
    .is_err());
    let mut changed = plan.settings();
    changed.configuration.method = Method::HochbruckOstermann;
    let foreign = Plan::from_rest(changed, 1 << 26).unwrap();
    assert!(archive::read(&bytes, foreign, bytes.len(), 1 << 26).is_err());
    changed = plan.settings();
    changed.force.samples = Layout::new([8; 3]).unwrap();
    let foreign = Plan::from_rest(changed, 1 << 26).unwrap();
    assert!(archive::read(&bytes, foreign, bytes.len(), 1 << 26).is_err());
    changed = plan.settings();
    changed.initial_clock = TickClock::from_rest(-19, 4096).unwrap();
    let foreign = Plan::from_rest(changed, 1 << 26).unwrap();
    assert!(archive::read(&bytes, foreign, bytes.len(), 1 << 26).is_err());
    assert!(archive::read(
        &bytes,
        plan,
        bytes.len(),
        archive::read_reservation(plan, 1).unwrap()
    )
    .is_ok());
}

#[test]
fn rehashed_work_and_physical_counter_tampering_is_rejected() {
    let (plan, run, original) = payload();
    let physical =
        nsbu_solver::checkpoint::physical::PhysicalArchive::encoded_len(run.state()).unwrap();
    let history = nsbu_solver::checkpoint::history::encoded_len(run.history()).unwrap();
    let work = 384 + physical + history;
    for offset in [work, work + 16, work + 32, work + 48, work + 64, work + 80] {
        let mut bytes = original.clone();
        bytes[offset..offset + 16].copy_from_slice(&99u128.to_le_bytes());
        rehash(&mut bytes);
        assert!(archive::read(&bytes, plan, bytes.len(), 1 << 26).is_err());
    }
    // Physical epoch and accepted-step words are authenticated but independently validated.
    for offset in [384 + 302, 384 + 318] {
        let mut bytes = original.clone();
        bytes[offset..offset + 16].copy_from_slice(&9u128.to_le_bytes());
        rehash(&mut bytes);
        assert!(archive::read(&bytes, plan, bytes.len(), 1 << 26).is_err());
    }
    // A plausible partial integration charge still cannot accompany a committed record.
    let mut bytes = original.clone();
    bytes[work..work + 16].copy_from_slice(&1u128.to_le_bytes());
    bytes[work + 16..work + 32].copy_from_slice(&64u128.to_le_bytes());
    bytes[work + 32..work + 48].copy_from_slice(&13u128.to_le_bytes());
    rehash(&mut bytes);
    assert!(archive::read(&bytes, plan, bytes.len(), 1 << 26).is_err());
    // A committed record cannot claim zero observation charge.
    let mut bytes = original;
    for offset in [work + 48, work + 64, work + 80] {
        bytes[offset..offset + 16].copy_from_slice(&0u128.to_le_bytes());
    }
    rehash(&mut bytes);
    assert!(archive::read(&bytes, plan, bytes.len(), 1 << 26).is_err());
}

#[test]
fn rehashed_header_and_clock_mutations_are_rejected() {
    let (plan, _run, original) = payload();
    for offset in [0usize, 8, 10, 74, 91, 191, 199, 207] {
        let mut bytes = original.clone();
        bytes[offset] ^= 1;
        rehash(&mut bytes);
        assert!(archive::read(&bytes, plan, bytes.len(), 1 << 26).is_err());
    }
    for offset in [336usize, 352, 368] {
        let mut bytes = original.clone();
        let value = u128::from_le_bytes(bytes[offset..offset + 16].try_into().unwrap());
        bytes[offset..offset + 16].copy_from_slice(&(value + 1).to_le_bytes());
        rehash(&mut bytes);
        assert!(archive::read(&bytes, plan, bytes.len(), 1 << 26).is_err());
    }
    let base = 384;
    // Physical header offsets: exponent250, target254, elapsed270, remaining286.
    // Changing elapsed and remaining together preserves the clock identity, so
    // the owner's history binding must reject this independently valid payload.
    let mut bytes = original.clone();
    let elapsed = base + 270;
    let remaining = base + 286;
    bytes[elapsed..elapsed + 16].copy_from_slice(&129u128.to_le_bytes());
    bytes[remaining..remaining + 16].copy_from_slice(&8063u128.to_le_bytes());
    rehash(&mut bytes);
    assert_eq!(
        archive::read(&bytes, plan, bytes.len(), 1 << 26).unwrap_err(),
        nsbu_solver::checkpoint::CheckpointError::InvalidHistory(
            nsbu_solver::SolverError::InvalidPayload
        )
    );
    let mut bytes = original.clone();
    bytes[base + 250..base + 254].copy_from_slice(&(-19i32).to_le_bytes());
    rehash(&mut bytes);
    assert_eq!(
        archive::read(&bytes, plan, bytes.len(), 1 << 26).unwrap_err(),
        nsbu_solver::checkpoint::CheckpointError::InvalidHistory(
            nsbu_solver::SolverError::InvalidPayload
        )
    );
    let mut bytes = original;
    bytes[base + 254..base + 270].copy_from_slice(&16384u128.to_le_bytes());
    bytes[remaining..remaining + 16].copy_from_slice(&16256u128.to_le_bytes());
    rehash(&mut bytes);
    assert_eq!(
        archive::read(&bytes, plan, bytes.len(), 1 << 26).unwrap_err(),
        nsbu_solver::checkpoint::CheckpointError::InvalidHistory(
            nsbu_solver::SolverError::InvalidPayload
        )
    );
}

#[test]
fn short_write_leaves_sentinel_untouched_and_truncation_fails() {
    let (plan, run, bytes) = payload();
    let n = bytes.len();
    let mut short = vec![0x5a; n - 1];
    assert!(archive::write(&run, &mut short).is_err());
    assert!(short.iter().all(|&x| x == 0x5a));
    assert!(archive::read(&bytes[..n - 1], plan, n, 1 << 26).is_err());
}
