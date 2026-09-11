//! Nested exact-v2 physical sampling retains complete statistics and peak locations.
#[path = "v2_sampling/actual.rs"]
mod actual;
mod v2_family_support;
mod v2_sampling_oracle;

use nsbu_benchmarks::v2_experiment::{
    physical::{PhysicalFamilyPlan, SampleMaximum, QUANTITIES},
    sampling::{SamplingPlan, SamplingWorkspace},
    FamilyError, FamilyPlan, V2Family,
};
use nsbu_solver::{
    diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField},
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};
use sha2::{Digest, Sha256};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];

fn layouts() -> [Layout; 3] {
    [12, 24, 48].map(|n| Layout::new([n; 3]).unwrap())
}

fn setup(clocks: &[TickClock; 3], attempts: usize) -> (FamilyPlan<'_>, SamplingPlan<'_>) {
    let family = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        TestedTimes::new(clocks, 3).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let sampling =
        SamplingPlan::new(family, layouts(), FLOORS, attempts, v2_family_support::CAP).unwrap();
    (family, sampling)
}

fn digest(family: &V2Family<'_>) -> [u8; 32] {
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
fn selected_signed_dft_values_match_sample_arrays_on_every_layout() {
    let clocks = v2_family_support::clocks();
    let (family_plan, _) = setup(&clocks, 3);
    let mut family = V2Family::new(family_plan).unwrap();
    for _ in clocks {
        family.advance().unwrap();
    }
    let left = family.branch(2).unwrap().state();
    let right = family.branch(5).unwrap().state();
    assert!(v2_sampling_oracle::retained_band_excludes(
        left,
        [
            left.plan().domain().layout().dimensions()[0] as isize / 2,
            0,
            0
        ]
    ));
    for samples in layouts() {
        let n = samples.dimensions()[0];
        let mut workspace = PhysicalComparisonWorkspace::new(
            left.plan().domain(),
            right.plan().domain(),
            samples,
            v2_family_support::CAP,
        )
        .unwrap();
        for (quantity_index, quantity) in QUANTITIES.into_iter().enumerate() {
            let report = workspace
                .compare_domains(
                    [left.plan().domain(), right.plan().domain()],
                    PhysicalField::Vector([
                        left.component(0).unwrap(),
                        left.component(1).unwrap(),
                        left.component(2).unwrap(),
                    ]),
                    PhysicalField::Vector([
                        right.component(0).unwrap(),
                        right.component(1).unwrap(),
                        right.component(2).unwrap(),
                    ]),
                    quantity,
                    FLOORS[quantity_index],
                )
                .unwrap();
            for index in [[0, 0, 1], [1, 1, 1], [n / 2 - 1, 0, 0]] {
                let linear = (index[0] * n + index[1]) * n + index[2];
                let (errors, references) =
                    v2_sampling_oracle::magnitudes(left, right, samples, index);
                let expected_error = errors[quantity_index];
                let expected_reference = references[quantity_index];
                let allowance = 3e-11 * (1.0 + expected_error.abs() + expected_reference.abs());
                assert!((report.error_magnitudes()[linear] - expected_error).abs() <= allowance);
                assert!(
                    (report.reference_magnitudes()[linear] - expected_reference).abs() <= allowance
                );
            }
        }
    }
}

#[test]
fn admission_identity_clock_and_attempt_refusals_are_bounded() {
    let clocks = v2_family_support::clocks();
    let (family_plan, plan) = setup(&clocks, 4);
    let good = layouts();
    assert!(SamplingPlan::new(
        family_plan,
        good,
        FLOORS,
        3,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(SamplingPlan::new(family_plan, good, FLOORS, 2, v2_family_support::CAP).is_err());
    assert!(PhysicalFamilyPlan::new(family_plan, good[0], FLOORS, 3, 0).is_err());
    assert!(SamplingPlan::new(family_plan, good, FLOORS, usize::MAX, usize::MAX).is_err());
    for invalid in [
        [good[0], good[0], good[2]],
        [good[2], good[1], good[0]],
        [good[0], Layout::new([20; 3]).unwrap(), good[2]],
        [Layout::new([12, 24, 12]).unwrap(), good[1], good[2]],
    ] {
        assert!(
            SamplingPlan::new(family_plan, invalid, FLOORS, 3, v2_family_support::CAP).is_err()
        );
    }
    let mut floors = FLOORS;
    floors[2] = 0.0;
    assert!(SamplingPlan::new(family_plan, good, floors, 3, v2_family_support::CAP).is_err());

    let mut family = V2Family::new(family_plan).unwrap();
    let mut sampling = SamplingWorkspace::new(plan).unwrap();
    assert!(matches!(
        sampling.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    family.advance().unwrap();
    assert_eq!(sampling.measure(&family).unwrap().clock(), clocks[0]);
    assert!(matches!(
        sampling.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(sampling.remaining(), 1);

    let altered_plan = FamilyPlan::new(
        v2_family_support::settings(2e-5),
        TestedTimes::new(&clocks, 3).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let mut altered = V2Family::new(altered_plan).unwrap();
    altered.advance().unwrap();
    altered.advance().unwrap();
    assert!(matches!(
        sampling.measure(&altered),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(sampling.remaining(), 0);
    assert!(matches!(
        sampling.measure(&family),
        Err(FamilyError::Numerical(SolverError::ProviderBudgetExceeded))
    ));
    assert!(!sampling.is_terminated());
}
