//! Complete derivative refinements follow the actual six-branch family and bounded clock schedule.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    physical::{PhysicalFamilyPlan, PhysicalFamilyWorkspace, QUANTITIES},
    FamilyPlan, SmoothFamily,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use sha2::{Digest, Sha256};

fn digest(family: &SmoothFamily<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    for index in 0..6 {
        for axis in 0..3 {
            for value in family
                .branch(index)
                .unwrap()
                .state()
                .component(axis)
                .unwrap()
            {
                hash.update(value.re.to_bits().to_le_bytes());
                hash.update(value.im.to_bits().to_le_bytes());
            }
        }
    }
    hash.finalize().into()
}

#[test]
fn all_quantities_and_refinement_pairs_come_from_actual_synchronized_rest_branches() {
    let clocks = smooth_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let family_plan = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        times,
        smooth_family_support::CAP,
    )
    .unwrap();
    let samples = Layout::new([24; 3]).unwrap();
    let floors = [1e-8, 1e-7, 1e-6, 1e-7];
    let plan = PhysicalFamilyPlan::new(family_plan, samples, floors, 6, smooth_family_support::CAP)
        .unwrap();
    assert_eq!(plan.sample_layout(), samples);
    assert_eq!(plan.relative_floors(), floors);
    assert_eq!(plan.bounds().work.scalar_transforms, 6 * 450);
    let mut family = SmoothFamily::new(family_plan).unwrap();
    let mut physical = PhysicalFamilyWorkspace::new(plan).unwrap();
    assert!(physical.measure(&family).is_err());
    assert_eq!(physical.next_time(), Some(clocks[0]));
    for expected in clocks {
        let fourier = family.advance().unwrap().unwrap();
        let before = digest(&family);
        let result = physical.measure(&family).unwrap();
        assert_eq!(result.clock(), expected);
        assert_eq!(result.sample_layout(), samples);
        assert_eq!(digest(&family), before);
        for (index, item) in result.quantities().iter().enumerate() {
            assert_eq!(item.quantity, QUANTITIES[index]);
            for error in item.space.into_iter().chain(item.time).chain([item.method]) {
                assert_eq!(error.components, item.quantity.components());
                assert_eq!(error.samples, samples.real_len());
                assert_eq!(error.relative_floor, floors[index]);
                assert!(error.peak_error >= error.rms_error);
            }
            if expected.elapsed() == 0 {
                assert_eq!(item.method.rms_error, 0.0);
            } else {
                assert!(item.method.rms_error > 0.0);
                assert!(item.time[1].rms_error < 0.2 * item.time[0].rms_error);
                println!("tick={} {:?}: spatial_RMS={:?}; temporal_RMS={:?}; method_RMS={}; accepted_pde_windows=0",
                    expected.elapsed(), item.quantity, item.space.map(|v| v.rms_error), item.time.map(|v| v.rms_error), item.method.rms_error);
            }
        }
        assert!((result.quantities()[0].method.rms_error - fourier.method().full.l2).abs() < 1e-15);
        if expected.elapsed() == 0 {
            assert!(physical.measure(&family).is_err());
            assert_eq!(physical.next_time(), Some(clocks[1]));
        }
    }
    assert_eq!(physical.next_time(), None);
    assert!(physical.measure(&family).is_err());
    assert_eq!(physical.remaining(), 0);
    assert_eq!(physical.charged_work(), plan.bounds().work);
    assert!(physical.measure(&family).is_err());
    assert_eq!(physical.charged_work(), plan.bounds().work);
}

#[test]
fn changed_policy_words_manifests_and_stopped_families_cannot_reuse_measurements() {
    let clocks = smooth_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let settings = smooth_family_support::settings(1e-2);
    let plan = FamilyPlan::new(settings, times, smooth_family_support::CAP).unwrap();
    let physical_plan = PhysicalFamilyPlan::new(
        plan,
        Layout::new([12; 3]).unwrap(),
        [1.0; 4],
        4,
        smooth_family_support::CAP,
    )
    .unwrap();
    let mut physical = PhysicalFamilyWorkspace::new(physical_plan).unwrap();
    let mut changed = settings;
    changed.tolerances.relative = [-0.0; 2];
    let mut other =
        SmoothFamily::new(FamilyPlan::new(changed, times, smooth_family_support::CAP).unwrap())
            .unwrap();
    other.advance().unwrap();
    assert!(physical.measure(&other).is_err());
    let shortened = [clocks[0], clocks[2]];
    let mut other = SmoothFamily::new(
        FamilyPlan::new(
            settings,
            TestedTimes::new(&shortened, 2).unwrap(),
            smooth_family_support::CAP,
        )
        .unwrap(),
    )
    .unwrap();
    other.advance().unwrap();
    assert!(physical.measure(&other).is_err());
    let mut failed = SmoothFamily::new(
        FamilyPlan::new(
            smooth_family_support::settings(1e-30),
            times,
            smooth_family_support::CAP,
        )
        .unwrap(),
    )
    .unwrap();
    failed.advance().unwrap();
    assert!(failed.advance().is_err());
    assert!(matches!(
        physical.measure(&failed),
        Err(nsbu_benchmarks::smooth_experiment::FamilyError::Terminated)
    ));
    assert_eq!(physical.next_time(), Some(clocks[0]));
    assert_eq!(physical.remaining(), 1);
    let mut original = SmoothFamily::new(plan).unwrap();
    original.advance().unwrap();
    assert_eq!(physical.measure(&original).unwrap().clock(), clocks[0]);
    assert_eq!(physical.remaining(), 0);
    assert_eq!(physical.charged_work(), physical_plan.bounds().work);
}

#[test]
fn joint_capacity_floors_sampling_and_finite_attempt_products_are_admitted() {
    let clocks = smooth_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let family = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        times,
        smooth_family_support::CAP,
    )
    .unwrap();
    let samples = Layout::new([24; 3]).unwrap();
    let plan =
        PhysicalFamilyPlan::new(family, samples, [1.0; 4], 3, smooth_family_support::CAP).unwrap();
    assert_eq!(
        plan.bounds().joint_storage_bytes,
        family.bounds().storage_bytes + plan.bounds().storage_bytes
    );
    assert!(PhysicalFamilyPlan::new(
        family,
        samples,
        [1.0; 4],
        3,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(
        PhysicalFamilyPlan::new(family, samples, [1.0; 4], 2, smooth_family_support::CAP).is_err()
    );
    assert!(PhysicalFamilyPlan::new(
        family,
        samples,
        [1.0; 4],
        usize::MAX,
        smooth_family_support::CAP
    )
    .is_err());
    assert!(PhysicalFamilyPlan::new(
        family,
        samples,
        [1.0, 1.0, 0.0, 1.0],
        3,
        smooth_family_support::CAP
    )
    .is_err());
    assert!(PhysicalFamilyPlan::new(
        family,
        Layout::new([8; 3]).unwrap(),
        [1.0; 4],
        3,
        smooth_family_support::CAP
    )
    .is_err());
}
