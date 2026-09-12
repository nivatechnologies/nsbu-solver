//! Focused bounded round trip coverage for the exact-v2 external archive.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{archive, Plan, Run, Settings},
};
use nsbu_solver::{
    checkpoint::CheckpointError,
    domain::{Domain, Layout, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    spectral::FftBackend,
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

fn pre_cache_settings() -> Settings {
    let mut result = settings(Method::CoxMatthews);
    result.configuration.limits.endpoint = 128;
    result.configuration.limits.maximum_attempts = 1;
    result.configuration.tolerances = Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    };
    result
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

#[test]
fn direct_layout_resources_and_pre_cache_archive_remain_compatible() {
    let plan = Plan::from_rest(pre_cache_settings(), 1 << 26).unwrap();
    assert_eq!(
        std::mem::size_of::<nsbu_benchmarks::runtime_force::RunForce>(),
        592
    );
    assert_eq!(std::mem::size_of::<Plan>(), 512);
    assert_eq!(std::mem::size_of::<Run>(), 5840);
    assert_eq!(
        std::mem::size_of::<nsbu_benchmarks::v2_run::ReconstructedPlan>(),
        480
    );
    assert_eq!(
        std::mem::size_of::<nsbu_benchmarks::v2_run::ReconstructedRun>(),
        7136
    );
    assert_eq!(plan.resources().total(), 252_184);
    let bytes = include_bytes!("fixtures/v2-archive-pre-cache.bin");
    let imported = archive::read(bytes, plan, bytes.len(), 1 << 26).unwrap();
    assert_eq!(imported.state().clock().elapsed(), 128);
    assert_eq!(imported.work().len(), 1);
}

#[test]
fn cached_profile_refuses_every_version_one_archive_operation() {
    let plan = Plan::from_rest_cached(pre_cache_settings(), 1 << 26).unwrap();
    let run = Run::from_rest(plan).unwrap();
    assert_eq!(
        archive::encoded_len(&run),
        Err(CheckpointError::InvalidEncoding)
    );
    assert_eq!(
        archive::maximum_encoded_len(plan),
        Err(CheckpointError::InvalidEncoding)
    );
    assert_eq!(
        archive::read_reservation(plan, 0),
        Err(CheckpointError::InvalidEncoding)
    );
    assert_eq!(
        archive::write(&run, &mut []),
        Err(CheckpointError::InvalidEncoding)
    );
    assert!(matches!(
        archive::read(&[], plan, 0, 0),
        Err(CheckpointError::InvalidEncoding)
    ));
}

#[test]
fn accelerated_arithmetic_plan_is_not_relabelled_as_version_one_archive() {
    let mut accelerated = pre_cache_settings();
    accelerated.domain = Domain::new([64; 3], [1.0; 3], 1.0).unwrap();
    accelerated.force.samples = Layout::new([96; 3]).unwrap();
    let plan =
        Plan::from_rest_with_fft(accelerated, FftBackend::RustFft6_4_1AvxFma, 34_359_738_368)
            .unwrap();
    assert_eq!(plan.fft_backend(), FftBackend::RustFft6_4_1AvxFma);
    assert_eq!(
        archive::maximum_encoded_len(plan),
        Err(CheckpointError::InvalidEncoding)
    );
    assert_eq!(
        archive::read_reservation(plan, 0),
        Err(CheckpointError::InvalidEncoding)
    );
}

#[test]
fn direct_read_reservation_refuses_excess_records() {
    let plan = Plan::from_rest(settings(Method::CoxMatthews), 1 << 26).unwrap();
    let records = plan.settings().configuration.limits.maximum_attempts + 1;
    assert_eq!(
        archive::read_reservation(plan, records),
        Err(CheckpointError::ResourceLimit)
    );
}
