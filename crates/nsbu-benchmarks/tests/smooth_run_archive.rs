//! External smooth-run payloads remain unqualified while retaining diagnostic continuation.
use nsbu_benchmarks::smooth_run::{
    archive::{encoded_len, read, write},
    Origin, SmoothRun,
};
use nsbu_solver::{
    checkpoint::CheckpointError,
    domain::{Domain, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};

fn configuration(method: Method) -> Configuration {
    Configuration {
        method,
        limits: RunLimits {
            endpoint: 8,
            step_ticks: 4,
            maximum_attempts: 2,
        },
        tolerances: Tolerances {
            absolute: [1.0; 2],
            relative: [0.0; 2],
        },
    }
}

fn run(method: Method) -> SmoothRun {
    SmoothRun::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-12, 100).unwrap(),
        configuration(method),
        2,
        0.3,
        8 * 1024 * 1024,
    )
    .unwrap()
}

fn bytes(run: &SmoothRun) -> Vec<u8> {
    let size = encoded_len(run).unwrap();
    let mut output = vec![0x55; size + 1];
    assert_eq!(
        write(run, &mut output[..size - 1]),
        Err(CheckpointError::ResourceLimit)
    );
    assert!(output.iter().all(|value| *value == 0x55));
    assert_eq!(write(run, &mut output), Ok(size));
    assert_eq!(output[size], 0x55);
    output.truncate(size);
    output
}

#[test]
fn checksum_import_is_explicitly_external_and_continues_with_fresh_scratch() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut original = run(method);
        assert!(matches!(original.step().unwrap(), Outcome::Committed(_)));
        assert_eq!(original.origin(), Origin::InternalFromRest);
        let encoded = bytes(&original);
        let imported = read(&encoded, original.state().plan(), encoded.len(), usize::MAX).unwrap();
        assert_eq!(imported.origin(), Origin::ExternalUnverified);
        assert_eq!(imported.state().clock(), original.state().clock());
        assert_eq!(
            imported.history().records().len(),
            original.history().records().len()
        );
        let mut resumed = imported.continue_unverified(8 * 1024 * 1024).unwrap();
        assert_eq!(resumed.origin(), Origin::ExternalUnverified);
        let snapshot = resumed.snapshot(usize::MAX).unwrap();
        let restored =
            SmoothRun::restore(snapshot, configuration(method), 8 * 1024 * 1024).unwrap();
        assert_eq!(restored.origin(), Origin::ExternalUnverified);
        assert_eq!(resumed.step(), original.step());
        assert_eq!(resumed.origin(), Origin::ExternalUnverified);
        assert_eq!(resumed.work(), original.work());
        assert_eq!(resumed.observer_work(), original.observer_work());
        for axis in 0..3 {
            for (left, right) in original
                .state()
                .component(axis)
                .unwrap()
                .iter()
                .zip(resumed.state().component(axis).unwrap())
            {
                assert_eq!(
                    [left.re.to_bits(), left.im.to_bits()],
                    [right.re.to_bits(), right.im.to_bits()]
                );
            }
        }
    }
}

#[test]
fn bounded_checksum_and_profile_frames_refuse_corruption_before_import() {
    let original = run(Method::CoxMatthews);
    let encoded = bytes(&original);
    assert!(matches!(
        read(
            &encoded,
            original.state().plan(),
            encoded.len() - 1,
            usize::MAX
        ),
        Err(CheckpointError::ResourceLimit)
    ));
    assert!(matches!(
        read(
            &encoded,
            original.state().plan(),
            encoded.len(),
            original.state().plan().total(),
        ),
        Err(CheckpointError::ResourceLimit)
    ));
    for index in [0, 10, 63, encoded.len() - 1] {
        let mut corrupted = encoded.clone();
        corrupted[index] ^= 1;
        assert!(matches!(
            read(
                &corrupted,
                original.state().plan(),
                corrupted.len(),
                usize::MAX
            ),
            Err(CheckpointError::HashMismatch)
        ));
    }
    for length in [0, 8, 263, encoded.len() - 1] {
        assert!(read(
            &encoded[..length],
            original.state().plan(),
            encoded.len(),
            usize::MAX
        )
        .is_err());
    }
}

fn seal(bytes: &mut [u8]) {
    use sha2::{Digest, Sha256};
    let body = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[..body]);
    bytes[body..].copy_from_slice(&digest);
}

#[test]
fn checksum_cannot_hide_epoch_drift_or_unrecorded_completed_rhs_work() {
    let mut original = run(Method::CoxMatthews);
    original.step().unwrap();
    let mut encoded = bytes(&original);
    // Version-one owner header then physical state-epoch offset.
    encoded[264 + 302..264 + 318].copy_from_slice(&42_u128.to_le_bytes());
    seal(&mut encoded);
    assert!(read(&encoded, original.state().plan(), encoded.len(), usize::MAX).is_err());

    let mut refused = SmoothRun::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-12, 100).unwrap(),
        configuration(Method::CoxMatthews),
        1,
        0.3,
        8 * 1024 * 1024,
    )
    .unwrap();
    refused.step().unwrap();
    assert!(matches!(
        refused.step().unwrap(),
        Outcome::Refused {
            indicators: Some(_),
            ..
        }
    ));
    let mut encoded = bytes(&refused);
    read(&encoded, refused.state().plan(), encoded.len(), usize::MAX).unwrap();
    // The final 48-byte work record precedes the 32-byte checksum.
    let record = encoded.len() - 80;
    encoded[record..record + 48].fill(0);
    seal(&mut encoded);
    assert!(read(&encoded, refused.state().plan(), encoded.len(), usize::MAX).is_err());
}
