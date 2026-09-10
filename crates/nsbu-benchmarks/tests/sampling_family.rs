//! Actual trajectory sampling and analytic peak controls keep RMS agreement separate from maxima.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    sampling::{SamplingPlan, SamplingWorkspace},
    FamilyPlan, SmoothFamily,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use sha2::{Digest, Sha256};
const FLOORS: [f64; 6] = [1e-8, 1e-7, 1e-6, 1e-7, 1e-8, 1e-7];

fn digest(family: &SmoothFamily<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    for branch in 0..6 {
        for axis in 0..3 {
            for value in family
                .branch(branch)
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
fn complete_sampling_refinements_retain_all_pairs_states_and_original_statistics() {
    let clocks = smooth_family_support::clocks();
    let family_plan = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        TestedTimes::new(&clocks, 3).unwrap(),
        smooth_family_support::CAP,
    )
    .unwrap();
    let layouts = [24, 32, 48].map(|n| Layout::new([n; 3]).unwrap());
    let plan =
        SamplingPlan::new(family_plan, layouts, FLOORS, 5, smooth_family_support::CAP).unwrap();
    assert_eq!(plan.sample_layouts(), layouts);
    assert_eq!(plan.relative_floors(), FLOORS);
    assert_eq!(plan.bounds().work.scalar_transforms, 5 * 1740);
    let mut family = SmoothFamily::new(family_plan).unwrap();
    let mut consumer = SamplingWorkspace::new(plan).unwrap();
    assert!(consumer.measure(&family).is_err());
    assert!(!consumer.is_terminated());
    assert_eq!(consumer.next_time(), Some(clocks[0]));
    for clock in clocks {
        family.advance().unwrap().unwrap();
        let before = digest(&family);
        let sample = consumer.measure(&family).unwrap();
        assert_eq!(sample.clock(), clock);
        assert_eq!(sample.sample_layouts(), layouts);
        assert_eq!(digest(&family), before);
        for (index, quantity) in sample.quantities().iter().enumerate() {
            assert_eq!(quantity.levels().len(), 3);
            for pair in 0..5 {
                let measured = quantity.pair(pair).unwrap();
                for (level, error) in measured.into_iter().enumerate() {
                    assert_eq!(error.samples, layouts[level].real_len());
                    assert_eq!(error.relative_floor, FLOORS[index]);
                    assert_eq!(error.components, quantity.quantity().components());
                    assert!(error.peak_error >= error.rms_error);
                    assert!(error.peak_relative_error.is_finite());
                }
                for change in quantity.changes(pair).unwrap() {
                    assert!(change.rms_error.is_finite() && change.rms_error >= 0.0);
                    assert!(change.peak_error.is_finite() && change.peak_error >= 0.0);
                    assert!(
                        change.peak_relative_error.is_finite() && change.reference_peak.is_finite()
                    );
                }
            }
            assert!(quantity.pair(5).is_err());
            assert!(quantity.changes(usize::MAX).is_err());
            let measured = quantity.pair(4).unwrap();
            if clock.elapsed() == 0 {
                assert_eq!(measured.map(|v| v.rms_error), [0.0; 3]);
            } else {
                assert!(measured[2].rms_error > 0.0);
                println!(
                    "tick={} {:?} CM/HO rms={:?} peak={:?} sampling_changes={:?}",
                    clock.elapsed(),
                    quantity.quantity(),
                    measured.map(|v| v.rms_error),
                    measured.map(|v| v.peak_error),
                    quantity.changes(4).unwrap()
                );
            }
        }
    }
    assert_eq!(consumer.next_time(), None);
    assert!(consumer.measure(&family).is_err());
    assert_eq!(consumer.charged_work(), plan.bounds().work);
    assert_eq!(consumer.remaining(), 0);
    assert!(consumer.measure(&family).is_err());
    assert_eq!(consumer.charged_work(), plan.bounds().work);
}

#[test]
fn joint_caps_strict_grid_refinement_floors_and_finite_work_are_required() {
    let clocks = smooth_family_support::clocks();
    let family = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        TestedTimes::new(&clocks, 3).unwrap(),
        smooth_family_support::CAP,
    )
    .unwrap();
    let layouts = [24, 32, 48].map(|n| Layout::new([n; 3]).unwrap());
    let plan = SamplingPlan::new(family, layouts, FLOORS, 3, smooth_family_support::CAP).unwrap();
    assert_eq!(
        plan.bounds().joint_storage_bytes,
        family.bounds().storage_bytes + plan.bounds().storage_bytes
    );
    assert!(SamplingPlan::new(
        family,
        layouts,
        FLOORS,
        3,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(SamplingPlan::new(family, layouts, FLOORS, 2, smooth_family_support::CAP).is_err());
    assert!(SamplingPlan::new(family, layouts, FLOORS, usize::MAX, usize::MAX).is_err());
    for invalid in [
        [layouts[0], layouts[0], layouts[2]],
        [layouts[2], layouts[1], layouts[0]],
        [Layout::new([12; 3]).unwrap(), layouts[1], layouts[2]],
        [layouts[0], Layout::new([32, 24, 32]).unwrap(), layouts[2]],
    ] {
        assert!(SamplingPlan::new(family, invalid, FLOORS, 3, smooth_family_support::CAP).is_err());
    }
    let mut floors = FLOORS;
    floors[4] = 0.0;
    assert!(SamplingPlan::new(family, layouts, floors, 3, smooth_family_support::CAP).is_err());
}

#[test]
fn exact_rms_on_three_grids_cannot_hide_a_missed_physical_peak() {
    use nsbu_solver::{
        diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
        domain::Domain,
        Complex64,
    };
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let mut coefficients = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let phase = std::f64::consts::FRAC_PI_8;
    coefficients[layout.index([1, 0, 0]).unwrap()] =
        Complex64::new(0.5 * phase.cos(), -0.5 * phase.sin());
    coefficients[layout.index([3, 0, 0]).unwrap()] =
        Complex64::new(0.5 * phase.cos(), 0.5 * phase.sin());
    let zero = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for (n, peak) in [(4, phase.cos()), (12, (phase / 3.0).cos()), (16, 1.0)] {
        let samples = Layout::new([n; 3]).unwrap();
        let mut work =
            PhysicalComparisonWorkspace::new(domain, domain, samples, 1024 * 1024).unwrap();
        let error = work
            .compare(
                PhysicalField::Scalar(&coefficients),
                PhysicalField::Scalar(&zero),
                PhysicalQuantity::Scalar,
                1.0,
            )
            .unwrap()
            .global();
        assert!((error.rms_error - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
        assert!((error.peak_error - peak).abs() < 1e-14);
    }
}
