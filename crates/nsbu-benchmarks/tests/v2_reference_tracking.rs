//! Actual-state analytical tracking, independent sampling and failure-contract checks.
mod v2_family_support;
mod v2_reference_oracle;

use nsbu_benchmarks::{
    fields::reference,
    time::BenchmarkTime,
    v2_experiment::{
        reference::{
            ReferenceTrackingError, ReferenceTrackingPlan, ReferenceTrackingWorkspace, QUANTITIES,
        },
        FamilyError, FamilyPlan, V2Family,
    },
    v2_run::Origin,
};
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];

fn setup<'a>(
    clocks: &'a [TickClock; 3],
    attempts: usize,
) -> (FamilyPlan<'a>, ReferenceTrackingPlan<'a>) {
    let family = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        TestedTimes::new(clocks, clocks.len()).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let tracking = ReferenceTrackingPlan::new(
        family,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        attempts,
        v2_family_support::CAP,
    )
    .unwrap();
    // Retain the existing test-only headroom, including the independently allocated
    // direct oracle and its phase::ALLOWANCE_BYTES basis scratch.
    assert!(tracking.bounds().joint_storage_bytes + 16 * 1024 * 1024 <= v2_family_support::CAP);
    (family, tracking)
}

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
                    .map(|value| (value.re.to_bits(), value.im.to_bits()))
            })
        })
        .collect()
}

#[test]
fn six_private_actual_states_track_all_reference_tensors_without_mutation() {
    let clocks = v2_family_support::clocks();
    let (family_plan, tracking_plan) = setup(&clocks, 4);
    assert_eq!(tracking_plan.bounds().work.scalar_transforms, 4 * 270);
    assert_eq!(tracking_plan.bounds().work.reference_evaluations, 4 * 1728);
    assert_eq!(tracking_plan.relative_floors(), FLOORS);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut tracking = ReferenceTrackingWorkspace::new(tracking_plan).unwrap();
    assert!(matches!(
        tracking.measure(&family),
        Err(ReferenceTrackingError::Family(FamilyError::InvalidFamily))
    ));
    for (frame, expected_clock) in clocks.into_iter().enumerate() {
        family.advance().unwrap().unwrap();
        let before = digest(&family);
        let sample = tracking.measure(&family).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(sample.clock(), expected_clock);
        assert_eq!(sample.identity(), family_plan.identity());
        assert_eq!(sample.sample_layout(), Layout::new([12; 3]).unwrap());
        assert_eq!(sample.relative_floors(), FLOORS);
        for (branch, report) in sample.branches().iter().enumerate() {
            assert_eq!(report.branch, branch);
            assert_eq!(
                family.branch(branch).unwrap().origin(),
                Origin::InternalFromRest
            );
            for (quantity, finding) in report.quantities.iter().enumerate() {
                assert_eq!(finding.quantity, QUANTITIES[quantity]);
                assert_shape(finding.error, QUANTITIES[quantity], FLOORS[quantity]);
                if frame == 0 {
                    assert_eq!(finding.error.rms_error, 0.0);
                    assert_eq!(finding.error.reference_peak, 0.0);
                }
            }
        }
        if frame == 2 {
            compare_independent_oracle(&family, &sample);
            let finest = sample.branches()[2]
                .quantities
                .map(|value| value.error.rms_error);
            println!("exact-v2 tracking endpoint branch2 RMS={finest:?}");
            assert!(finest.into_iter().all(|value| value > 0.0));
            assert!(finest[2] > 1e-4);
        }
    }
}

fn assert_shape(error: LocalError, quantity: PhysicalQuantity, floor: f64) {
    assert_eq!(error.components, quantity.components());
    assert_eq!(error.samples, 1728);
    assert_eq!(error.relative_floor, floor);
    assert!(error.rms_error.is_finite());
    assert!(error.peak_error >= error.rms_error);
    assert!(error.peak_relative_error.is_finite());
    assert!(error.reference_peak.is_finite());
}

fn compare_independent_oracle(
    family: &V2Family<'_>,
    sample: &nsbu_benchmarks::v2_experiment::reference::ReferenceTrackingSample,
) {
    let time = BenchmarkTime::new(sample.clock()).unwrap();
    for branch in 0..6 {
        v2_reference_oracle::assert_legacy_equivalence(
            family.branch(branch).unwrap().state(),
            sample.sample_layout(),
        );
        let expected = v2_reference_oracle::tracking(
            family.branch(branch).unwrap().state(),
            sample.sample_layout(),
            time,
            FLOORS,
        );
        for (actual, expected) in sample.branches()[branch].quantities.iter().zip(expected) {
            for (measured, oracle) in [
                (actual.error.rms_error, expected.rms),
                (actual.error.peak_error, expected.peak),
                (actual.error.peak_relative_error, expected.relative_peak),
                (actual.error.reference_peak, expected.reference_peak),
            ] {
                assert!((measured - oracle).abs() < 2e-10 * (1.0 + oracle));
            }
        }
    }
}

#[test]
fn stale_foreign_terminal_and_exhausted_requests_retain_state_and_schedule() {
    let clocks = v2_family_support::clocks();
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let (family_plan, tracking_plan) = setup(&clocks, 4);
    let mut family = V2Family::new(family_plan).unwrap();
    family.advance().unwrap();
    let mut tracking = ReferenceTrackingWorkspace::new(tracking_plan).unwrap();
    let altered = FamilyPlan::new(
        v2_family_support::settings(2e-5),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let mut foreign = V2Family::new(altered).unwrap();
    foreign.advance().unwrap();
    let before = digest(&foreign);
    assert!(tracking.measure(&foreign).is_err());
    assert_eq!(digest(&foreign), before);
    assert_eq!(tracking.next_time(), Some(clocks[0]));
    assert_eq!(tracking.measure(&family).unwrap().clock(), clocks[0]);
    let before = digest(&family);
    assert!(tracking.measure(&family).is_err());
    assert_eq!(digest(&family), before);
    assert_eq!(tracking.next_time(), Some(clocks[1]));

    let low = FamilyPlan::new(
        v2_family_support::settings(1e-40),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let low_plan = ReferenceTrackingPlan::new(
        low,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        3,
        v2_family_support::CAP,
    )
    .unwrap();
    let mut failed = V2Family::new(low).unwrap();
    let mut observer = ReferenceTrackingWorkspace::new(low_plan).unwrap();
    failed.advance().unwrap();
    observer.measure(&failed).unwrap();
    assert!(failed.advance().is_err());
    let before = digest(&failed);
    assert!(matches!(
        observer.measure(&failed),
        Err(ReferenceTrackingError::Family(FamilyError::Terminated))
    ));
    assert_eq!(digest(&failed), before);
    assert_eq!(observer.next_time(), Some(clocks[1]));

    assert_eq!(tracking.remaining(), 1);
    assert!(tracking.measure(&foreign).is_err());
    assert_eq!(tracking.remaining(), 0);
    assert!(matches!(
        tracking.measure(&foreign),
        Err(ReferenceTrackingError::Numerical(
            SolverError::ProviderBudgetExceeded
        ))
    ));
}

#[test]
fn admission_refuses_bad_floors_layout_attempts_caps_and_work_overflow() {
    let clocks = v2_family_support::clocks();
    let (family, _) = setup(&clocks, 3);
    let samples = Layout::new([12; 3]).unwrap();
    let good =
        ReferenceTrackingPlan::new(family, samples, FLOORS, 3, v2_family_support::CAP).unwrap();
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(ReferenceTrackingPlan::new(
            family,
            samples,
            [FLOORS[0], bad, FLOORS[2], FLOORS[3]],
            3,
            v2_family_support::CAP
        )
        .is_err());
    }
    assert!(ReferenceTrackingPlan::new(
        family,
        Layout::new([8; 3]).unwrap(),
        FLOORS,
        3,
        v2_family_support::CAP
    )
    .is_err());
    assert!(
        ReferenceTrackingPlan::new(family, samples, FLOORS, 2, v2_family_support::CAP).is_err()
    );
    assert!(ReferenceTrackingPlan::new(
        family,
        samples,
        FLOORS,
        3,
        good.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(ReferenceTrackingPlan::new(family, samples, FLOORS, usize::MAX, usize::MAX).is_err());
}

#[test]
fn analytical_tensor_path_retains_the_existing_high_precision_fixture() {
    let columns: Vec<_> = include_str!("fixtures/derivatives.tsv")
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .collect();
    let clock = TickClock::restore(-8, 2, 1, 1).unwrap();
    let value = reference::evaluate([0.0; 3], BenchmarkTime::new(clock).unwrap()).unwrap();
    let actual: Vec<_> = value
        .velocity
        .into_iter()
        .chain(value.gradient.into_iter().flatten())
        .chain(value.hessian.into_iter().flatten().flatten())
        .chain(value.vorticity)
        .collect();
    assert_eq!(actual.len(), 42);
    for (actual, expected) in actual.into_iter().zip(&columns[5..47]) {
        let expected: f64 = expected.parse().unwrap();
        assert!((actual - expected).abs() < 2e-9 * (1.0 + expected.abs()));
    }
}
