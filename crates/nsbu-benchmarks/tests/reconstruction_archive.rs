//! External reconstruction bytes retain bits and budgets without conferring trusted origin.
mod owned_reconstruction_support;
use nsbu_benchmarks::smooth_observer::reconstruction::{archive, ReconstructionObserver};
use nsbu_solver::{
    checkpoint::CheckpointError, domain::TickClock, integrators::method::Method, Complex64,
};
use owned_reconstruction_support::run;

#[test]
fn every_accepted_ring_phase_roundtrips_for_both_methods() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        for steps in 0..4 {
            let mut owner = run(method, 1e-2, 5);
            for _ in 0..steps {
                owner.step().unwrap();
            }
            let mut bytes = vec![0; archive::encoded_len(owner.observer()).unwrap()];
            assert_eq!(
                archive::write(owner.observer(), &mut bytes).unwrap(),
                bytes.len()
            );
            let imported = archive::read(
                &bytes,
                owner.state().plan(),
                5,
                owner.state(),
                bytes.len(),
                usize::MAX,
            )
            .unwrap();
            let restored = imported
                .restore_unverified(owner.state().plan(), 5, owner.state())
                .unwrap();
            assert_eq!(restored.consumption(), owner.observer().consumption());
            assert_eq!(restored.modal_visits(), owner.observer().modal_visits());
            assert_eq!(
                restored.last_accepted_clocks(),
                owner.observer().last_accepted_clocks()
            );
            compare_interpolant(
                owner.observer(),
                &restored,
                owner.state().plan().domain().layout().half_len(),
            );
            let mut encoded_again = vec![0; bytes.len()];
            archive::write(&restored, &mut encoded_again).unwrap();
            assert_eq!(encoded_again, bytes);
        }
    }
}
fn compare_interpolant(a: &ReconstructionObserver, b: &ReconstructionObserver, half: usize) {
    let Some(clocks) = a.last_accepted_clocks() else {
        return;
    };
    let tick = clocks[1].elapsed() + 1;
    let probe = TickClock::restore(
        clocks[0].exponent(),
        clocks[0].target(),
        tick,
        clocks[0].target() - tick,
    )
    .unwrap();
    let mut av = vec![Complex64::new(0.0, 0.0); half];
    let mut bv = av.clone();
    let mut ad = av.clone();
    let mut bd = av.clone();
    for axis in 0..3 {
        a.reconstruct(probe, axis, &mut av, &mut ad).unwrap();
        b.reconstruct(probe, axis, &mut bv, &mut bd).unwrap();
        for (a, b) in av.iter().chain(&ad).zip(bv.iter().chain(&bd)) {
            assert_eq!(
                [a.re.to_bits(), a.im.to_bits()],
                [b.re.to_bits(), b.im.to_bits()]
            );
        }
    }
}
#[test]
fn shape_clock_state_and_counter_corruption_are_refused() {
    let mut owner = run(Method::CoxMatthews, 1e-2, 5);
    owner.step().unwrap();
    owner.step().unwrap();
    let mut bytes = vec![0; archive::encoded_len(owner.observer()).unwrap()];
    archive::write(owner.observer(), &mut bytes).unwrap();
    let read = |bytes: &[u8]| {
        archive::read(
            bytes,
            owner.state().plan(),
            5,
            owner.state(),
            bytes.len(),
            usize::MAX,
        )
    };
    for length in [0, 8, 106, bytes.len() - 1] {
        assert!(read(&bytes[..length]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(read(&extra).is_err());
    let half = owner.state().plan().domain().layout().half_len();
    for (offset, value) in [(0, 0_u8), (8, 2), (26, 0), (26, 4), (27, 0), (91, 0)] {
        let mut changed = bytes.clone();
        changed[offset] = value;
        assert!(read(&changed).is_err(), "offset {offset}");
    }
    // First clock must be the rest node. The final field must match physical bits exactly.
    for offset in [107 + 20, 107 + 2 * (84 + 96 * half) + 84] {
        let mut changed = bytes.clone();
        changed[offset..offset + 8].copy_from_slice(&1.0_f64.to_bits().to_le_bytes());
        assert!(read(&changed).is_err());
    }
    let mut changed = bytes.clone();
    let derivative = 107 + 84 + 48 * half;
    changed[derivative..derivative + 8].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
    assert!(read(&changed).is_err());
    assert!(archive::read(
        &bytes,
        owner.state().plan(),
        4,
        owner.state(),
        bytes.len(),
        usize::MAX
    )
    .is_err());
}
#[test]
fn complete_caps_and_output_rollback_are_enforced() {
    let owner = run(Method::HochbruckOstermann, 1e-2, 5);
    let required = archive::encoded_len(owner.observer()).unwrap();
    let mut short = vec![123; required - 1];
    assert_eq!(
        archive::write(owner.observer(), &mut short),
        Err(CheckpointError::ResourceLimit)
    );
    assert!(short.iter().all(|byte| *byte == 123));
    let mut bytes = vec![0; required];
    archive::write(owner.observer(), &mut bytes).unwrap();
    assert!(archive::read(
        &bytes,
        owner.state().plan(),
        5,
        owner.state(),
        required - 1,
        usize::MAX
    )
    .is_err());
    let storage =
        ReconstructionObserver::snapshot_reservation(owner.state().plan().domain()).unwrap();
    assert!(archive::read(
        &bytes,
        owner.state().plan(),
        5,
        owner.state(),
        required,
        storage - 1
    )
    .is_err());
    assert!(archive::read(
        &bytes,
        owner.state().plan(),
        5,
        owner.state(),
        required,
        storage
    )
    .is_ok());
}

#[test]
fn earlier_node_identity_and_actual_rest_values_are_checked() {
    let mut owner = run(Method::CoxMatthews, 1e-2, 5);
    owner.step().unwrap();
    owner.step().unwrap();
    let mut bytes = vec![0; archive::encoded_len(owner.observer()).unwrap()];
    archive::write(owner.observer(), &mut bytes).unwrap();
    let read = |bytes: &[u8]| {
        archive::read(
            bytes,
            owner.state().plan(),
            5,
            owner.state(),
            bytes.len(),
            usize::MAX,
        )
    };
    for offset in [107 + 52, 107 + 68] {
        let mut changed = bytes.clone();
        changed[offset..offset + 16].copy_from_slice(&1_u128.to_le_bytes());
        assert!(read(&changed).is_err());
    }
    let mut wrong_rest = bytes.clone();
    wrong_rest[107 + 84..107 + 92].copy_from_slice(&1.0_f64.to_bits().to_le_bytes());
    assert!(read(&wrong_rest).is_err());
    let mut wrong_family = bytes.clone();
    let changed_target = (1_u128 << 20) + 1;
    for offset in [107 + 4, 107 + 36] {
        wrong_family[offset..offset + 16].copy_from_slice(&changed_target.to_le_bytes());
    }
    assert!(read(&wrong_family).is_err());
}
