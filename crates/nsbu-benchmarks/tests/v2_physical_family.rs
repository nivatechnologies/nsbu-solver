//! Physical exact-v2 measurements retain complete quantity/pair policy and actual state.
mod v2_family_support;
mod v2_physical_oracle;

use nsbu_benchmarks::v2_experiment::{
    physical::{PhysicalFamilyPlan, PhysicalFamilyWorkspace, PhysicalRefinementSample, QUANTITIES},
    FamilyError, FamilyPlan, V2Family,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};
const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];

fn digest(family: &V2Family<'_>) -> Vec<(u64, u64)> {
    (0..6)
        .flat_map(|branch| {
            (0..3).flat_map(move |axis| {
                family
                    .branch(branch)
                    .unwrap()
                    .state()
                    .component(axis)
                    .unwrap()
                    .iter()
                    .map(|z| (z.re.to_bits(), z.im.to_bits()))
            })
        })
        .collect()
}

fn setup(clocks: &[TickClock; 3], attempts: usize) -> (FamilyPlan<'_>, PhysicalFamilyPlan<'_>) {
    let family = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        TestedTimes::new(clocks, 3).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let physical = PhysicalFamilyPlan::new(
        family,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        attempts,
        v2_family_support::CAP,
    )
    .unwrap();
    (family, physical)
}

fn compare_full_physical_grid(family: &V2Family<'_>, report: &PhysicalRefinementSample) {
    for (pair, (left, right)) in [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)]
        .into_iter()
        .enumerate()
    {
        let expected = v2_physical_oracle::pair_norms(
            family.branch(left).unwrap().state(),
            family.branch(right).unwrap().state(),
            report.sample_layout(),
        );
        for (quantity, (item, expected)) in report.quantities().iter().zip(expected).enumerate() {
            let actual = [
                item.space[0],
                item.space[1],
                item.time[0],
                item.time[1],
                item.method,
            ][pair];
            for (measured, oracle) in [
                (actual.rms_error, expected.rms),
                (actual.peak_error, expected.peak),
            ] {
                let allowance = 2e-11 * (FLOORS[quantity] + oracle);
                assert!(
                    (measured - oracle).abs() < allowance,
                    "pair {pair} {:?}: {measured:e} != {oracle:e}; allowance {allowance:e}",
                    item.quantity
                );
                assert!(
                    measured > 0.0,
                    "all final physical pair differences are nonzero"
                );
            }
        }
    }
}

#[test]
fn actual_rest_frames_measure_every_quantity_and_pair() {
    let clocks = v2_family_support::clocks();
    let (family_plan, physical_plan) = setup(&clocks, 5);
    assert_eq!(physical_plan.bounds().work.scalar_transforms, 5 * 450);
    assert_eq!(physical_plan.relative_floors(), FLOORS);
    let samples = physical_plan.sample_layout();
    let mut family = V2Family::new(family_plan).unwrap();
    let mut physical = PhysicalFamilyWorkspace::new(physical_plan).unwrap();
    assert!(matches!(
        physical.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(physical.charged_work().attempts, 1);
    for (frame, expected) in clocks.into_iter().enumerate() {
        family.advance().unwrap().unwrap();
        let before = digest(&family);
        let report = physical.measure(&family).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(report.clock(), expected);
        assert_eq!(report.sample_layout(), samples);
        assert_eq!(report.identity(), family_plan.identity());
        assert_eq!(report.relative_floors(), FLOORS);
        for (index, item) in report.quantities().iter().enumerate() {
            assert_eq!(item.quantity, QUANTITIES[index]);
            for error in item.space.into_iter().chain(item.time).chain([item.method]) {
                assert_eq!(error.components, item.quantity.components());
                assert_eq!(error.samples, samples.real_len());
                assert_eq!(error.relative_floor, FLOORS[index]);
                assert!(error.rms_error.is_finite() && error.peak_error >= error.rms_error);
                if frame == 0 {
                    assert_eq!((error.rms_error, error.peak_error), (0.0, 0.0));
                }
            }
        }
        if frame == 2 {
            compare_full_physical_grid(&family, &report);
        }
    }
    assert_eq!(physical.next_time(), None);
    assert!(matches!(
        physical.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    let charged = physical.charged_work();
    assert_eq!(charged, physical_plan.bounds().work);
    assert!(matches!(
        physical.measure(&family),
        Err(FamilyError::Numerical(SolverError::ProviderBudgetExceeded))
    ));
    assert_eq!(physical.charged_work(), charged);
}

#[test]
fn stale_wrong_identity_and_terminated_calls_spend_attempts_without_mutating_states() {
    let clocks = v2_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let (family_plan, physical_plan) = setup(&clocks, 6);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut physical = PhysicalFamilyWorkspace::new(physical_plan).unwrap();
    family.advance().unwrap();
    let altered = FamilyPlan::new(
        v2_family_support::settings(2e-5),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let mut other = V2Family::new(altered).unwrap();
    other.advance().unwrap();
    // Both families are at the correct schedule slot and clock; only identity differs.
    let other_before = digest(&other);
    assert!(matches!(
        physical.measure(&other),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(digest(&other), other_before);
    assert_eq!(physical.charged_work().attempts, 1);
    assert_eq!(physical.measure(&family).unwrap().clock(), clocks[0]);
    let state_before = digest(&family);
    assert!(matches!(
        physical.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(digest(&family), state_before);
    assert_eq!(physical.charged_work().attempts, 3);
    assert_eq!(physical.next_time(), Some(clocks[1]));

    let low = FamilyPlan::new(
        v2_family_support::settings(1e-40),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let low_physical = PhysicalFamilyPlan::new(
        low,
        physical_plan.sample_layout(),
        FLOORS,
        3,
        v2_family_support::CAP,
    )
    .unwrap();
    let mut failed = V2Family::new(low).unwrap();
    let mut observer = PhysicalFamilyWorkspace::new(low_physical).unwrap();
    failed.advance().unwrap();
    observer.measure(&failed).unwrap();
    assert!(failed.advance().is_err());
    let before = digest(&failed);
    assert!(matches!(
        observer.measure(&failed),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(digest(&failed), before);
    assert_eq!(observer.charged_work().attempts, 2);
    assert_eq!(observer.next_time(), Some(clocks[1]));
}

#[test]
fn physical_admission_rejects_invalid_floors_layout_count_caps_and_overflow() {
    let clocks = v2_family_support::clocks();
    let (family, _) = setup(&clocks, 3);
    let samples = Layout::new([12; 3]).unwrap();
    let good =
        PhysicalFamilyPlan::new(family, samples, [1.0; 4], 3, v2_family_support::CAP).unwrap();
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(PhysicalFamilyPlan::new(
            family,
            samples,
            [1.0, 1.0, bad, 1.0],
            3,
            v2_family_support::CAP
        )
        .is_err());
    }
    assert!(PhysicalFamilyPlan::new(
        family,
        Layout::new([8; 3]).unwrap(),
        [1.0; 4],
        3,
        v2_family_support::CAP
    )
    .is_err());
    assert!(PhysicalFamilyPlan::new(family, samples, [1.0; 4], 2, v2_family_support::CAP).is_err());
    assert!(PhysicalFamilyPlan::new(
        family,
        samples,
        [1.0; 4],
        3,
        good.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(PhysicalFamilyPlan::new(
        family,
        samples,
        [1.0; 4],
        usize::MAX,
        v2_family_support::CAP
    )
    .is_err());
    assert!(Layout::new([usize::MAX; 3]).is_err());
}
