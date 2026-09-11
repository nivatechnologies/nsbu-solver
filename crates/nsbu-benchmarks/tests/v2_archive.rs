//! Focused bounded round trip coverage for the exact-v2 external archive.
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

fn round_trip(method: Method) {
    let plan = Plan::from_rest(settings(method), 1 << 26).unwrap();
    let mut run = Run::from_rest(plan).unwrap();
    assert!(matches!(
        run.step().unwrap(),
        nsbu_solver::experiment::control::Outcome::Committed(_)
    ));
    let mut uninterrupted = Run::from_rest(plan).unwrap();
    uninterrupted.step().unwrap();
    uninterrupted.step().unwrap();
    let size = archive::encoded_len(&run).unwrap();
    let mut bytes = vec![0xa5; size + 7];
    let written = archive::write(&run, &mut bytes).unwrap();
    assert_eq!(written, size);
    assert_eq!(&bytes[size..], &[0xa5; 7]);
    let imported = archive::read(&bytes[..size], plan, size, 1 << 26).unwrap();
    assert_eq!(
        imported.origin(),
        nsbu_benchmarks::v2_run::Origin::ExternalUnverified
    );
    assert_eq!(
        imported.history().records().len(),
        run.history().records().len()
    );
    assert_eq!(imported.work(), run.work());
    let mut continued = imported.continue_unverified(1 << 26).unwrap();
    assert!(matches!(
        continued.step().unwrap(),
        nsbu_solver::experiment::control::Outcome::Committed(_)
    ));
    equivalent(&continued, &uninterrupted);
}

fn equivalent(left: &Run, right: &Run) {
    let size = archive::encoded_len(left).unwrap();
    assert_eq!(size, archive::encoded_len(right).unwrap());
    let mut left_bytes = vec![0; size];
    let mut right_bytes = vec![0; size];
    archive::write(left, &mut left_bytes).unwrap();
    archive::write(right, &mut right_bytes).unwrap();
    assert_eq!(left_bytes, right_bytes);
    assert_eq!(left.state().clock(), right.state().clock());
    assert_eq!(left.state().epoch(), right.state().epoch());
    assert_eq!(
        left.state().accepted_steps(),
        right.state().accepted_steps()
    );
    for axis in 0..3 {
        let a = left.state().component(axis).unwrap();
        let b = right.state().component(axis).unwrap();
        for (x, y) in a.iter().zip(b) {
            assert_eq!(x.re.to_bits(), y.re.to_bits());
            assert_eq!(x.im.to_bits(), y.im.to_bits());
        }
    }
    assert_eq!(left.work(), right.work());
    assert_eq!(left.observer_work(), right.observer_work());
    assert_eq!(
        left.history().records().len(),
        right.history().records().len()
    );
    for (a, b) in left
        .history()
        .records()
        .iter()
        .zip(right.history().records())
    {
        assert_eq!(a.start, b.start);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.sample, b.sample);
    }
}

#[test]
fn round_trip_preserves_cm_full_state() {
    round_trip(Method::CoxMatthews);
}

#[test]
fn round_trip_preserves_ho_full_state() {
    round_trip(Method::HochbruckOstermann);
}

#[test]
fn rejects_short_output_hash_and_caps() {
    let plan = Plan::from_rest(settings(Method::HochbruckOstermann), 1 << 26).unwrap();
    let run = Run::from_rest(plan).unwrap();
    let size = archive::encoded_len(&run).unwrap();
    assert!(archive::write(&run, &mut vec![0; size - 1]).is_err());
    let mut bytes = vec![0; size];
    archive::write(&run, &mut bytes).unwrap();
    assert!(archive::read(&bytes, plan, size - 1, 1 << 26).is_err());
    assert!(archive::read(&bytes, plan, size, plan.resources().total() - 1).is_err());
    bytes[20] ^= 1;
    assert!(archive::read(&bytes, plan, size, 1 << 26).is_err());

    // A rehashed semantic mutation must still be rejected by plan identity checks.
    archive::write(&run, &mut bytes).unwrap();
    bytes[123..139].copy_from_slice(&1u128.to_le_bytes());
    let digest = Sha256::digest(&bytes[..size - 32]);
    bytes[size - 32..].copy_from_slice(&digest);
    assert!(archive::read(&bytes, plan, size, 1 << 26).is_err());
}

#[test]
fn hochbruck_round_trip_keeps_full_work_ledger() {
    let plan = Plan::from_rest(settings(Method::HochbruckOstermann), 1 << 26).unwrap();
    let mut run = Run::from_rest(plan).unwrap();
    run.step().unwrap();
    let size = archive::encoded_len(&run).unwrap();
    let mut bytes = vec![0; size];
    assert_eq!(archive::write(&run, &mut bytes).unwrap(), size);
    let imported = archive::read(&bytes, plan, size, 1 << 26).unwrap();
    assert_eq!(imported.work(), run.work());
    assert_eq!(imported.observer_work(), run.observer_work());
    let resumed = imported.continue_unverified(1 << 26).unwrap();
    assert_eq!(resumed.state().clock(), run.state().clock());
    assert_eq!(
        resumed.history().controller().committed(),
        run.history().controller().committed()
    );
}
