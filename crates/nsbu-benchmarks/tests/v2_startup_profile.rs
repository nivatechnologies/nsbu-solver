//! Fixed-array startup profile admission shared by example and CLI.
use nsbu_benchmarks::v2_experiment::diagnostic::StartupProfile;

const CAP: usize = 256 * 1024 * 1024;

#[test]
fn owned_clocks_admit_exactly_the_fixed_complete_partition() {
    let profile = StartupProfile::new().unwrap();
    assert_eq!(
        profile.accepted_times().map(|clock| clock.elapsed()),
        [0, 64, 128]
    );
    assert_eq!(
        profile.manifest().map(|clock| clock.elapsed()),
        [0, 7, 63, 64, 95, 127, 128]
    );
    assert_eq!(
        profile.residual_times().map(|clock| clock.elapsed()),
        [7, 63, 95, 127]
    );
    let plan = profile.plan(CAP).unwrap();
    assert_eq!(plan.bounds().work.attempts, 7);
    assert_eq!(plan.residual_times(), profile.residual_times());
    assert!(profile.plan(plan.bounds().joint_storage_bytes - 1).is_err());
}
