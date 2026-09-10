//! Off-stage residuals must be collected while each branch's actual accepted history still exists.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        residuals::{ResidualFamily, ResidualFamilyPlan},
        ProbeFamily, ProbePlan,
    },
    FamilyError, FamilyPlan,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use sha2::{Digest, Sha256};
fn clocks() -> [TickClock; 7] {
    [0, 7, 31, 63, 95, 127, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap())
}
fn owner_plan<'a>(
    accepted: &'a [TickClock],
    times: &'a [TickClock],
    tolerance: f64,
) -> ProbePlan<'a> {
    let cap = smooth_family_support::CAP;
    let family = FamilyPlan::new(
        smooth_family_support::settings(tolerance),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        cap,
    )
    .unwrap();
    ProbePlan::new(
        family,
        TestedTimes::new(times, times.len()).unwrap(),
        times.len(),
        cap,
    )
    .unwrap()
}
fn digest(family: &ProbeFamily<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    for index in 0..6 {
        let fields = family.fields(index).unwrap();
        let state = family.branch(index).unwrap().state();
        hash.update(state.clock().elapsed().to_le_bytes());
        for axis in 0..3 {
            for values in [
                fields.value[axis],
                fields.derivative[axis],
                state.component(axis).unwrap(),
            ] {
                for value in values {
                    hash.update(value.re.to_bits().to_le_bytes());
                    hash.update(value.im.to_bits().to_le_bytes());
                }
            }
        }
    }
    hash.finalize().into()
}

#[test]
fn early_and_late_residuals_retain_complete_fields_and_strict_actual_temporal_geometry() {
    let accepted = smooth_family_support::clocks();
    let times = clocks();
    let subset = &times[1..6];
    let plan = owner_plan(&accepted, &times, 1e-2);
    let admission = ResidualFamilyPlan::new(plan, subset, 5, smooth_family_support::CAP).unwrap();
    assert_eq!(admission.tested_times(), subset);
    assert_eq!(admission.probe_plan().tested_times().as_slice(), &times);
    assert_eq!(admission.bounds().admission_geometry_checks, 30);
    let mut family = ProbeFamily::new(plan).unwrap();
    let mut residuals = ResidualFamily::new(admission).unwrap();
    let mut early = None;
    let mut norm_difference_gap = 0.0_f64;
    while let Some(probe) = family.advance().unwrap() {
        if residuals.next_time() != Some(probe.clock()) {
            continue;
        }
        let before = digest(&family);
        let measured = residuals.measure(&family).unwrap();
        assert_eq!(measured.clock(), probe.clock());
        assert_eq!(measured.reconstruction().origins(), probe.origins());
        assert_eq!(digest(&family), before);
        let levels = measured.temporal_geometry().levels();
        assert!(levels[1].refines(levels[0]) && levels[2].refines(levels[1]));
        for (index, branch) in measured.branches().iter().enumerate() {
            assert_eq!(
                branch.geometry().nodes(),
                probe.origins()[index].accepted_nodes
            );
            assert_eq!(branch.geometry().time(), probe.clock());
            assert_eq!(branch.domain(), family.fields(index).unwrap().domain);
            assert!(branch.norms().l2 > 0.0);
            assert!(branch.norms().h1.is_finite());
        }
        for comparison in measured.comparisons() {
            assert!(comparison.full.l2.is_finite());
            let split = comparison.common.l2.hypot(comparison.newly_resolved.l2);
            assert!((split - comparison.full.l2).abs() < 1e-20);
        }
        for (slot, (a, b)) in [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)]
            .into_iter()
            .enumerate()
        {
            let wrong =
                (measured.branches()[a].norms().l2 - measured.branches()[b].norms().l2).abs();
            norm_difference_gap =
                norm_difference_gap.max((wrong - measured.comparisons()[slot].full.l2).abs());
        }
        if probe.clock().elapsed() == 7 {
            early = Some(measured);
        }
    }
    let early = early.unwrap();
    assert_eq!(early.clock().elapsed(), 7);
    assert!(
        norm_difference_gap > 1e-12,
        "actual residual fields are not scalar norm differences: {norm_difference_gap}"
    );
    assert_eq!(residuals.charged_work(), admission.bounds().work);
    assert!(residuals.child_work().iter().all(|work| work.probes == 5));
    assert_eq!(residuals.remaining(), 0);
    assert_eq!(residuals.next_time(), None);
    assert!(!residuals.is_terminated());
    assert!(
        family
            .branch(2)
            .unwrap()
            .observer()
            .last_accepted_clocks()
            .unwrap()[0]
            .elapsed()
            > 7
    );
    let charged = residuals.charged_work();
    assert!(residuals.measure(&family).is_err());
    assert_eq!(charged, residuals.charged_work());
}

#[test]
fn admission_refuses_nodes_missing_times_order_caps_and_unrepresentable_offstage_geometry() {
    let accepted = smooth_family_support::clocks();
    let times = clocks();
    let plan = owner_plan(&accepted, &times, 1e-2);
    let subset = &times[1..6];
    let cap = smooth_family_support::CAP;
    let admitted = ResidualFamilyPlan::new(plan, subset, 5, cap).unwrap();
    assert!(
        ResidualFamilyPlan::new(plan, subset, 5, admitted.bounds().joint_storage_bytes - 1)
            .is_err()
    );
    assert!(ResidualFamilyPlan::new(plan, subset, 4, cap).is_err());
    assert!(ResidualFamilyPlan::new(plan, subset, usize::MAX, usize::MAX).is_err());
    for bad in [
        &[][..],
        &times[..1],
        &times[6..],
        &[times[2], times[1]],
        &[times[1], times[1]],
    ] {
        assert!(ResidualFamilyPlan::new(plan, bad, 5, cap).is_err());
    }
    let missing = [TickClock::restore(-16, 512, 9, 503).unwrap()];
    assert!(ResidualFamilyPlan::new(plan, &missing, 5, cap).is_err());
    let mut settings = smooth_family_support::settings(1e-2);
    settings.steps = [16, 8, 4];
    let family = FamilyPlan::new(settings, TestedTimes::new(&accepted, 3).unwrap(), cap).unwrap();
    let fine = ProbePlan::new(family, TestedTimes::new(&times, 7).unwrap(), 7, cap).unwrap();
    // With H=4 every integral tick is a stage time; an off-stage claim needs a finer clock.
    assert!(ResidualFamilyPlan::new(fine, subset, 5, cap).is_err());
}

#[test]
fn missing_skipped_changed_and_rejected_owners_cannot_publish_residuals() {
    let accepted = smooth_family_support::clocks();
    let times = clocks();
    let plan = owner_plan(&accepted, &times, 1e-2);
    let admission =
        ResidualFamilyPlan::new(plan, &times[1..2], 4, smooth_family_support::CAP).unwrap();
    let mut family = ProbeFamily::new(plan).unwrap();
    let mut residuals = ResidualFamily::new(admission).unwrap();
    assert!(residuals.measure(&family).is_err());
    let mut other = ProbeFamily::new(owner_plan(&accepted, &times, 2e-2)).unwrap();
    other.advance().unwrap();
    other.advance().unwrap();
    assert!(residuals.measure(&other).is_err());
    for _ in 0..3 {
        family.advance().unwrap();
    }
    assert!(residuals.measure(&family).is_err());
    let mut rejected = ProbeFamily::new(owner_plan(&accepted, &times, 1e-30)).unwrap();
    assert!(rejected.advance().is_err());
    assert!(matches!(
        residuals.measure(&rejected),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(residuals.charged_work(), admission.bounds().work);
    assert!(residuals.child_work().iter().all(|work| work.probes == 0));
    assert!(!residuals.is_terminated());
    assert_eq!(residuals.next_time(), Some(times[1]));
}
