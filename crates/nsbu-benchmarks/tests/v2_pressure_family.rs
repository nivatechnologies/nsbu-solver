//! Actual exact-v2 pressure differences, full-band independent sums and bounded reporting.
mod v2_family_support;
mod v2_pressure_oracle;
use nsbu_benchmarks::v2_experiment::{
    pressure::{PressureFamilyPlan, PressureFamilyWorkspace, PressureRefinementSample, QUANTITIES},
    FamilyError, FamilyPlan, V2Family,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};
use sha2::{Digest, Sha256};

const FLOORS: [f64; 2] = [1e-8, 1e-7];

fn digest(family: &V2Family<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    for index in 0..6 {
        let branch = family.branch(index).unwrap();
        hash.update(branch.state().clock().elapsed().to_le_bytes());
        hash.update(branch.state().clock().remaining().to_le_bytes());
        hash.update(branch.state().accepted_steps().to_le_bytes());
        for axis in 0..3 {
            for z in branch.state().component(axis).unwrap() {
                hash.update(z.re.to_bits().to_le_bytes());
                hash.update(z.im.to_bits().to_le_bytes());
            }
        }
    }
    hash.finalize().into()
}

fn setup(clocks: &[TickClock; 3], attempts: usize) -> (FamilyPlan<'_>, PressureFamilyPlan<'_>) {
    let family = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        TestedTimes::new(clocks, 3).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let plan = PressureFamilyPlan::new(
        family,
        Layout::new([24; 3]).unwrap(),
        FLOORS,
        attempts,
        v2_family_support::CAP,
    )
    .unwrap();
    // Include separately owned full-complex oracle arrays and test metadata.
    assert!(plan.bounds().joint_storage_bytes + 16 * 1024 * 1024 <= v2_family_support::CAP);
    (family, plan)
}

fn compare_pairs(family: &V2Family<'_>, report: &PressureRefinementSample) {
    for (pair, (left, right)) in [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)]
        .into_iter()
        .enumerate()
    {
        let expected = v2_pressure_oracle::pair_norms(
            family.branch(left).unwrap().state(),
            family.branch(right).unwrap().state(),
            report.sample_layout(),
        );
        for (index, (quantity, oracle)) in report.quantities().iter().zip(expected).enumerate() {
            let finding = [
                quantity.space[0],
                quantity.space[1],
                quantity.time[0],
                quantity.time[1],
                quantity.method,
            ][pair];
            // Full pressures contain the same independently sampled force pressure.
            // Their binary64 subtraction has an absolute error scale set by that pressure,
            // even when the nonlinear trajectory difference is much smaller.
            let allowance = 5e-12 * (finding.reference_peak + FLOORS[index]);
            for (actual, expected) in [
                (finding.rms_error, oracle.rms),
                (finding.peak_error, oracle.peak),
            ] {
                assert!((actual - expected).abs() < allowance,
                    "pair={pair} quantity={index}: {actual:e} != {expected:e}; allowance={allowance:e}");
                if pair < 2 {
                    // Direct relative agreement must resolve the nonzero spatial signal.
                    assert!(
                        expected > 0.0 && (actual - expected).abs() < 1e-5 * expected,
                        "spatial pair={pair} quantity={index}: {actual:e} != {expected:e}"
                    );
                }
            }
            if pair < 2 {
                assert!(finding.rms_error > 0.0);
            }
            println!("pressure pair={pair} quantity={index} measured_rms={:e} oracle_rms={:e} measured_peak={:e} oracle_peak={:e} arithmetic_allowance={allowance:e}",
                finding.rms_error, oracle.rms, finding.peak_error, oracle.peak);
        }
    }
}

#[test]
fn actual_rest_states_supply_complete_pressure_and_gradient_pairs() {
    let clocks = v2_family_support::clocks();
    let (family_plan, plan) = setup(&clocks, 5);
    assert_eq!(plan.sample_layout().dimensions(), [24; 3]);
    assert_eq!(plan.relative_floors(), FLOORS);
    assert_eq!(plan.bounds().work.scalar_transforms, 5 * 133);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut pressure = PressureFamilyWorkspace::new(plan).unwrap();
    assert!(matches!(
        pressure.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(pressure.charged_work().attempts, 1);
    for (frame, clock) in clocks.into_iter().enumerate() {
        family.advance().unwrap().unwrap();
        let before = digest(&family);
        let report = pressure.measure(&family).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(report.clock(), clock);
        assert_eq!(report.identity(), family_plan.identity());
        assert_eq!(report.source_domain().layout().dimensions(), [12; 3]);
        assert_eq!(report.sample_layout(), plan.sample_layout());
        assert_eq!(report.force_layout().dimensions(), [24; 3]);
        assert_eq!(report.relative_floors(), FLOORS);
        for (index, item) in report.quantities().iter().enumerate() {
            assert_eq!(item.quantity, QUANTITIES[index]);
            for finding in item.space.into_iter().chain(item.time).chain([item.method]) {
                assert_eq!(finding.components, item.quantity.components());
                assert_eq!(finding.samples, 24usize.pow(3));
                assert_eq!(finding.relative_floor, FLOORS[index]);
                assert!(finding.rms_error.is_finite() && finding.peak_error >= finding.rms_error);
                assert!(
                    finding.reference_peak.is_finite() && finding.peak_relative_error.is_finite()
                );
                if frame == 0 {
                    assert_eq!(
                        (
                            finding.rms_error,
                            finding.peak_error,
                            finding.reference_peak
                        ),
                        (0.0, 0.0, 0.0)
                    );
                }
            }
        }
        if frame == 2 {
            compare_pairs(&family, &report);
        }
    }
    assert_eq!(pressure.next_time(), None);
    assert!(matches!(
        pressure.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(pressure.charged_work(), plan.bounds().work);
    assert!(matches!(
        pressure.measure(&family),
        Err(FamilyError::Numerical(SolverError::ProviderBudgetExceeded))
    ));
    assert_eq!(pressure.charged_work(), plan.bounds().work);
}

#[test]
fn unrelated_stale_and_terminated_families_spend_attempts_without_state_changes() {
    let clocks = v2_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let (family_plan, plan) = setup(&clocks, 6);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut pressure = PressureFamilyWorkspace::new(plan).unwrap();
    family.advance().unwrap();
    let changed = FamilyPlan::new(
        v2_family_support::settings(2e-5),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let live_bytes = plan.bounds().joint_storage_bytes + changed.bounds().storage_bytes;
    assert!(live_bytes <= v2_family_support::CAP);
    let mut other = V2Family::new(changed).unwrap();
    other.advance().unwrap();
    let other_before = digest(&other);
    assert!(matches!(
        pressure.measure(&other),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(digest(&other), other_before);
    pressure.measure(&family).unwrap();
    let before = digest(&family);
    assert!(matches!(
        pressure.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(digest(&family), before);
    assert_eq!(pressure.charged_work().attempts, 3);
    assert_eq!(pressure.next_time(), Some(clocks[1]));

    let low = FamilyPlan::new(
        v2_family_support::settings(1e-40),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let failed_plan =
        PressureFamilyPlan::new(low, plan.sample_layout(), FLOORS, 3, v2_family_support::CAP)
            .unwrap();
    assert!(live_bytes + failed_plan.bounds().joint_storage_bytes <= v2_family_support::CAP);
    let mut failed = V2Family::new(low).unwrap();
    let mut observer = PressureFamilyWorkspace::new(failed_plan).unwrap();
    failed.advance().unwrap();
    observer.measure(&failed).unwrap();
    assert!(failed.advance().is_err());
    let before = digest(&failed);
    assert!(matches!(
        observer.measure(&failed),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(digest(&failed), before);
    assert_eq!(observer.next_time(), Some(clocks[1]));
    assert_eq!(observer.charged_work().attempts, 2);
}

#[test]
fn complete_pressure_admission_refuses_bad_samples_floors_counts_caps_and_overflow() {
    let clocks = v2_family_support::clocks();
    let (family, plan) = setup(&clocks, 3);
    let samples = plan.sample_layout();
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            PressureFamilyPlan::new(family, samples, [1.0, bad], 3, v2_family_support::CAP)
                .is_err()
        );
    }
    for shape in [[12; 3], [24, 24, 12], [26; 3]] {
        assert!(PressureFamilyPlan::new(
            family,
            Layout::new(shape).unwrap(),
            FLOORS,
            3,
            v2_family_support::CAP
        )
        .is_err());
    }
    assert!(PressureFamilyPlan::new(family, samples, FLOORS, 2, v2_family_support::CAP).is_err());
    assert!(PressureFamilyPlan::new(
        family,
        samples,
        FLOORS,
        3,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(
        PressureFamilyPlan::new(family, samples, FLOORS, usize::MAX, v2_family_support::CAP)
            .is_err()
    );
}
