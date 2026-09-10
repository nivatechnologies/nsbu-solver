//! Actual from-rest execution is required before unverified canonical checkpoint bytes can match.
mod owned_reconstruction_support;
use nsbu_benchmarks::smooth_run::{
    reconstructed_archive as archive,
    replay::{ReplayError, ReplayPlan},
    Origin, ReconstructedRun,
};
use nsbu_solver::{domain::ResourcePlan, integrators::method::Method};
use owned_reconstruction_support::{compare, run, CAP};
use sha2::{Digest, Sha256};
fn encoded(run: &ReconstructedRun) -> Vec<u8> {
    let mut bytes = vec![0; archive::encoded_len(run).unwrap()];
    archive::write(run, &mut bytes).unwrap();
    bytes
}
fn import(bytes: &[u8], plan: ResourcePlan) -> ReconstructedRun {
    archive::read(bytes, plan, bytes.len(), CAP)
        .unwrap()
        .continue_unverified(CAP)
        .unwrap()
}
#[test]
fn every_ring_phase_replays_all_canonical_bytes_and_preserves_the_original_origin() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut original = run(method, 1e-2, 5);
        for attempts in 0..=4 {
            let bytes = encoded(&original);
            let imported = import(&bytes, original.state().plan());
            let imported_bytes = encoded(&imported);
            let plan = ReplayPlan::new(&imported, attempts, CAP).unwrap();
            assert_eq!(plan.bounds().attempts, attempts);
            assert_eq!(plan.bounds().archive_bytes, bytes.len());
            let result = plan.execute().unwrap();
            assert_eq!(
                result.report().archive_sha256,
                <[u8; 32]>::from(Sha256::digest(&imported_bytes))
            );
            assert_eq!(result.report().attempts, attempts);
            assert_eq!(result.report().compared_bytes, bytes.len());
            assert_eq!(result.run().origin(), Origin::InternalFromRest);
            compare(result.run(), &original);
            let mut replayed = result.into_run();
            assert_eq!(encoded(&replayed), bytes);
            assert_ne!(
                replayed.state().component(0).unwrap().as_ptr(),
                imported.state().component(0).unwrap().as_ptr()
            );
            assert_eq!(imported.origin(), Origin::ExternalUnverified);
            assert!(encoded(&imported) == imported_bytes);
            if attempts < 4 {
                assert_eq!(original.step().unwrap(), replayed.step().unwrap());
                compare(&original, &replayed);
            }
        }
    }
}
#[test]
fn terminal_rejections_and_diagnostic_refusals_are_reproduced_with_spent_work() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        for (tolerance, samples) in [(1e-40, 5), (1e-2, 1)] {
            let mut original = run(method, tolerance, samples);
            original.step().unwrap();
            let bytes = encoded(&original);
            let imported = import(&bytes, original.state().plan());
            let replayed = ReplayPlan::new(&imported, 1, CAP)
                .unwrap()
                .execute()
                .unwrap();
            compare(replayed.run(), &original);
            assert_eq!(encoded(replayed.run()), bytes);
        }
    }
}
#[test]
fn a_structurally_valid_rehashed_derivative_is_rejected_by_independent_evolution() {
    let mut original = run(Method::CoxMatthews, 1e-2, 5);
    original.step().unwrap();
    original.step().unwrap();
    let mut bytes = encoded(&original);
    let core = usize::try_from(u128::from_le_bytes(bytes[10..26].try_into().unwrap())).unwrap();
    let first_derivative =
        42 + core + 107 + 84 + 3 * original.state().plan().domain().layout().half_len() * 16;
    bytes[first_derivative..first_derivative + 8].copy_from_slice(&1.0f64.to_le_bytes());
    let end = bytes.len() - 32;
    let hash = Sha256::digest(&bytes[..end]);
    bytes[end..].copy_from_slice(&hash);
    let imported = import(&bytes, original.state().plan());
    let imported_bytes = encoded(&imported);
    assert!(matches!(
        ReplayPlan::new(&imported, 2, CAP).unwrap().execute(),
        Err(ReplayError::DifferentReplay)
    ));
    assert!(encoded(&imported) == imported_bytes);
    assert_eq!(imported.origin(), Origin::ExternalUnverified);
}
#[test]
fn replay_admission_requires_complete_attempt_and_simultaneous_storage_allowances() {
    let mut original = run(Method::CoxMatthews, 1e-2, 5);
    original.step().unwrap();
    assert!(matches!(
        ReplayPlan::new(&original, 0, CAP),
        Err(ReplayError::AttemptLimit)
    ));
    assert!(ReplayPlan::new(&original, 1, 1).is_err());
    let admitted = ReplayPlan::new(&original, 1, CAP).unwrap().bounds();
    assert!(admitted.storage_bytes > 2 * original.state().plan().total());
    assert!(ReplayPlan::new(&original, 1, admitted.storage_bytes - 1).is_err());
    assert!(ReplayPlan::new(&original, 1, admitted.storage_bytes).is_ok());
}
