//! Outer identity frame binds accepted reconstruction, raw history and physical state.
mod owned_reconstruction_support;
use nsbu_benchmarks::{
    smooth_observer::reconstruction::archive as nodes,
    smooth_run::{archive, reconstructed_archive as full, Origin, ReconstructedRun},
};
use nsbu_solver::{checkpoint::CheckpointError, integrators::method::Method};
use owned_reconstruction_support::{compare, run, CAP};
use sha2::{Digest, Sha256};

fn encode(run: &ReconstructedRun) -> Vec<u8> {
    let mut bytes = vec![0; full::encoded_len(run).unwrap()];
    full::write(run, &mut bytes).unwrap();
    bytes
}
#[test]
fn all_ring_phases_preserve_next_attempt_and_interpolant_after_external_import() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        for steps in 0..4 {
            let mut original = run(method, 1e-2, 5);
            for _ in 0..steps {
                original.step().unwrap();
            }
            let bytes = encode(&original);
            let imported = full::read(&bytes, original.state().plan(), bytes.len(), CAP).unwrap();
            assert_eq!(imported.origin(), Origin::ExternalUnverified);
            assert_eq!(imported.configuration().method, method);
            assert_eq!(imported.history().controller().committed(), steps);
            assert_eq!(imported.state().clock(), original.state().clock());
            let mut restored = imported.continue_unverified(CAP).unwrap();
            compare(&original, &restored);
            assert_eq!(original.step(), restored.step());
            compare(&original, &restored);
            assert_eq!(restored.origin(), Origin::ExternalUnverified);
            let snapshot = restored.snapshot(CAP).unwrap();
            let restored = ReconstructedRun::restore(
                snapshot,
                restored.history().controller().configuration(),
                CAP,
            )
            .unwrap();
            assert_eq!(restored.origin(), Origin::ExternalUnverified);
            let bytes = encode(&restored);
            let imported = full::read(&bytes, restored.state().plan(), bytes.len(), CAP).unwrap();
            assert_eq!(imported.origin(), Origin::ExternalUnverified);
        }
    }
}
#[test]
fn rejected_and_exhausted_runs_keep_the_same_terminal_state_and_work() {
    for (tolerance, samples, steps) in [(1e-30, 5, 1), (1e-2, 3, 3)] {
        let mut original = run(Method::CoxMatthews, tolerance, samples);
        for _ in 0..steps {
            original.step().unwrap();
        }
        let bytes = encode(&original);
        let imported = full::read(&bytes, original.state().plan(), bytes.len(), CAP).unwrap();
        let restored = imported.continue_unverified(CAP).unwrap();
        compare(&original, &restored);
    }
}
fn rehash(bytes: &mut [u8]) {
    let p = bytes.len() - 32;
    let hash = Sha256::digest(&bytes[..p]);
    bytes[p..].copy_from_slice(&hash);
}
fn lengths(bytes: &[u8]) -> (usize, usize) {
    (
        u128::from_le_bytes(bytes[10..26].try_into().unwrap()) as usize,
        u128::from_le_bytes(bytes[26..42].try_into().unwrap()) as usize,
    )
}
#[test]
fn common_frame_cannot_drop_reconstruction_and_rehashed_node_clocks_must_match_history() {
    let mut owner = run(Method::CoxMatthews, 1e-2, 5);
    for _ in 0..3 {
        owner.step().unwrap();
    }
    let mut bytes = encode(&owner);
    let (core_len, node_len) = lengths(&bytes);
    let core = &bytes[42..42 + core_len];
    assert!(archive::read(core, owner.state().plan(), core.len(), CAP).is_err());
    let start = 42 + core_len;
    let stride = 84 + 96 * owner.state().plan().domain().layout().half_len();
    for (index, tick) in [(0, 512u128), (1, 1792u128)] {
        let clock = start + 107 + index * stride;
        bytes[clock + 20..clock + 36].copy_from_slice(&tick.to_le_bytes());
        bytes[clock + 36..clock + 52].copy_from_slice(&((1u128 << 20) - tick).to_le_bytes());
    }
    // Standalone nodes remain equally spaced and agree with the last physical state.
    assert!(nodes::read(
        &bytes[start..start + node_len],
        owner.state().plan(),
        5,
        owner.state(),
        node_len,
        CAP
    )
    .is_ok());
    rehash(&mut bytes);
    assert!(full::read(&bytes, owner.state().plan(), bytes.len(), CAP).is_err());
}
#[test]
fn outer_integrity_caps_and_output_rollback_are_enforced() {
    let owner = run(Method::HochbruckOstermann, 1e-2, 5);
    let bytes = encode(&owner);
    assert!(
        full::maximum_encoded_len(
            owner.state().plan(),
            owner.history().controller().configuration()
        )
        .unwrap()
            >= bytes.len()
    );
    let mut short = vec![73; bytes.len() - 1];
    assert_eq!(
        full::write(&owner, &mut short),
        Err(CheckpointError::ResourceLimit)
    );
    assert!(short.iter().all(|byte| *byte == 73));
    for n in [0, 42, bytes.len() - 1] {
        assert!(full::read(&bytes[..n], owner.state().plan(), bytes.len(), CAP).is_err());
    }
    assert!(full::read(&bytes, owner.state().plan(), bytes.len() - 1, CAP).is_err());
    assert!(full::read(&bytes, owner.state().plan(), bytes.len(), 1).is_err());
    let mut bad = bytes.clone();
    bad[42] ^= 1;
    assert!(matches!(
        full::read(&bad, owner.state().plan(), bad.len(), CAP),
        Err(CheckpointError::HashMismatch)
    ));
    rehash(&mut bad);
    assert!(full::read(&bad, owner.state().plan(), bad.len(), CAP).is_err());
}
