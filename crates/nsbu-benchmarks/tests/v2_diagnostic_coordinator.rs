//! Bounded unqualified coordination across accepted and genuine off-stage clocks.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    binding::NodeBindingStatus,
    diagnostic::{
        AcceptedSchedule, DiagnosticDriver, DiagnosticError, DiagnosticPlan, DiagnosticSettings,
        DiagnosticStatus, MissingChannel, ResidualSchedule,
    },
    physical::{PhysicalFamilyPlan, PhysicalFamilyWorkspace, PhysicalRefinementSample},
    pressure::{PressureFamilyPlan, PressureFamilyWorkspace, PressureRefinementSample},
    probes::{
        physical::{ProbePhysicalPlan, ProbePhysicalSample, ProbePhysicalWorkspace},
        ProbePlan,
    },
    reference::{
        regional::{RegionalTrackingPlan, RegionalTrackingSample, RegionalTrackingWorkspace},
        ReferenceTrackingPlan,
    },
    FamilyPlan,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
};
use v2_family_support::{clocks, settings, CAP};

fn ticks(values: &[u128]) -> Vec<TickClock> {
    values
        .iter()
        .map(|&elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
        .collect()
}

fn policy() -> DiagnosticSettings {
    DiagnosticSettings {
        physical_samples: Layout::new([12; 3]).unwrap(),
        pressure_samples: Layout::new([24; 3]).unwrap(),
        reference_samples: Layout::new([12; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
    }
}

fn plans<'a>(
    accepted: &'a [TickClock],
    manifest: &'a [TickClock],
    residual: &'a [TickClock],
) -> DiagnosticPlan<'a> {
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    DiagnosticPlan::new(family, probes, residual, policy(), CAP).unwrap()
}

fn assert_physical(left: PhysicalRefinementSample, right: PhysicalRefinementSample) {
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.identity(), right.identity());
    assert_eq!(left.sample_layout(), right.sample_layout());
    assert_eq!(left.relative_floors(), right.relative_floors());
    for (a, b) in left.quantities().iter().zip(right.quantities()) {
        assert_eq!(a.quantity, b.quantity);
        assert_eq!(a.space, b.space);
        assert_eq!(a.time, b.time);
        assert_eq!(a.method, b.method);
    }
}

fn assert_pressure(left: PressureRefinementSample, right: PressureRefinementSample) {
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.identity(), right.identity());
    assert_eq!(left.source_domain(), right.source_domain());
    assert_eq!(left.sample_layout(), right.sample_layout());
    assert_eq!(left.force_layout(), right.force_layout());
    assert_eq!(left.relative_floors(), right.relative_floors());
    for (a, b) in left.quantities().iter().zip(right.quantities()) {
        assert_eq!(a.quantity, b.quantity);
        assert_eq!(a.space, b.space);
        assert_eq!(a.time, b.time);
        assert_eq!(a.method, b.method);
    }
}

fn assert_regional(left: RegionalTrackingSample, right: RegionalTrackingSample) {
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.identity(), right.identity());
    assert_eq!(left.sample_layout(), right.sample_layout());
    assert_eq!(left.relative_floors(), right.relative_floors());
    for (a, b) in left.branches().iter().zip(right.branches()) {
        assert_eq!(a.branch, b.branch);
        for (x, y) in a.quantities.iter().zip(b.quantities) {
            assert_eq!(x.quantity, y.quantity);
            assert_eq!(x.global, y.global);
            assert_eq!(x.regional.components, y.regional.components);
            assert_eq!(x.regional.clock, y.regional.clock);
            assert_eq!(x.regional.dimensions, y.regional.dimensions);
            assert_eq!(x.regional.grid_complete, y.regional.grid_complete);
            assert_eq!(x.regional.global, y.regional.global);
            assert_eq!(x.regional.regions, y.regional.regions);
            assert_eq!(x.regional.root_work_charged, y.regional.root_work_charged);
        }
    }
}

fn assert_probe_physical(left: ProbePhysicalSample, right: ProbePhysicalSample) {
    assert_eq!(
        (left.clock(), left.identity()),
        (right.clock(), right.identity())
    );
    assert_eq!(left.origins(), right.origins());
    assert_eq!(left.source_domains(), right.source_domains());
    assert_eq!(left.sample_layout(), right.sample_layout());
    assert_eq!(left.relative_floors(), right.relative_floors());
    for (a, b) in left.quantities().iter().zip(right.quantities()) {
        assert_eq!((a.quantity, a.pairs), (b.quantity, b.pairs));
        for pair in 0..5 {
            let x = a.extrema(pair).unwrap();
            let y = b.extrema(pair).unwrap();
            assert_eq!(x.error.measured(), y.error.measured());
            assert_eq!(x.relative_error.measured(), y.relative_error.measured());
            assert_eq!(x.reference.measured(), y.reference.measured());
        }
    }
}

#[test]
fn publishes_crossed_consumers_in_one_unqualified_manifest() {
    let accepted = clocks();
    let manifest = ticks(&[0, 7, 63, 64, 95, 127, 128]);
    let residual = ticks(&[7, 63, 95, 127]);
    let plan = plans(&accepted, &manifest, &residual);
    assert_eq!(plan.bounds().work.attempts, 7);
    assert_eq!(plan.bounds().work.accepted_events, 3);
    assert_eq!(plan.bounds().work.residual_events, 4);
    assert_eq!(plan.bounds().work.manifest_comparisons, 49);
    assert!(plan.bounds().joint_storage_bytes <= CAP);

    let family_identity = plan.family_plan().identity();
    let probe_identity = plan.probe_plan().identity();
    let standalone_policy = policy();
    let standalone_physical_plan = PhysicalFamilyPlan::new(
        plan.family_plan(),
        standalone_policy.physical_samples,
        standalone_policy.physical_floors,
        accepted.len(),
        CAP,
    )
    .unwrap();
    let standalone_probe_physical_plan = ProbePhysicalPlan::new(
        plan.probe_plan(),
        standalone_policy.physical_samples,
        standalone_policy.physical_floors,
        manifest.len(),
        CAP,
    )
    .unwrap();
    let standalone_pressure_plan = PressureFamilyPlan::new(
        plan.family_plan(),
        standalone_policy.pressure_samples,
        standalone_policy.pressure_floors,
        accepted.len(),
        CAP,
    )
    .unwrap();
    let tracking = ReferenceTrackingPlan::new(
        plan.family_plan(),
        standalone_policy.reference_samples,
        standalone_policy.reference_floors,
        accepted.len(),
        CAP,
    )
    .unwrap();
    let standalone_regional_plan = RegionalTrackingPlan::new(tracking, 128, CAP).unwrap();
    let standalone_bytes = standalone_physical_plan
        .bounds()
        .storage_bytes
        .checked_add(standalone_probe_physical_plan.bounds().storage_bytes)
        .and_then(|n| n.checked_add(standalone_pressure_plan.bounds().storage_bytes))
        .and_then(|n| {
            n.checked_add(
                standalone_regional_plan.bounds().joint_storage_bytes
                    - plan.family_plan().bounds().storage_bytes,
            )
        })
        .unwrap();
    assert!(plan.bounds().joint_storage_bytes + standalone_bytes <= CAP);
    let mut driver = DiagnosticDriver::new(plan).unwrap();
    let mut standalone_physical = PhysicalFamilyWorkspace::new(standalone_physical_plan).unwrap();
    let mut standalone_probe_physical =
        ProbePhysicalWorkspace::new(standalone_probe_physical_plan).unwrap();
    let mut standalone_pressure = PressureFamilyWorkspace::new(standalone_pressure_plan).unwrap();
    let mut standalone_regional = RegionalTrackingWorkspace::new(standalone_regional_plan).unwrap();
    let mut maximum_residual_l2 = 0.0_f64;
    for (index, &clock) in manifest.iter().enumerate() {
        assert_eq!(driver.next_time(), Some(clock));
        let event = driver.advance().unwrap().unwrap();
        assert_eq!(event.clock(), clock);
        assert_eq!(event.family_identity(), family_identity);
        assert_eq!(event.probe_identity(), probe_identity);
        assert_eq!(event.probe().clock(), clock);
        assert_eq!(event.probe().identity(), probe_identity);
        assert_eq!(event.status(), DiagnosticStatus::UnqualifiedDiagnostic);
        assert_probe_physical(
            event.reconstructed_physical(),
            standalone_probe_physical
                .measure(driver.probes(), event.probe())
                .unwrap(),
        );
        assert!(event
            .missing_channels()
            .contains(&MissingChannel::ForceResolution));
        assert!(event
            .missing_channels()
            .contains(&MissingChannel::ReferencePrecision));
        if accepted.contains(&clock) {
            assert_eq!(event.accepted().schedule(), AcceptedSchedule::Measured);
            let measured = event.accepted().sample().unwrap();
            assert!(matches!(
                event.residual().schedule(),
                ResidualSchedule::NotScheduledAtAcceptedClock
            ));
            assert!(event.residual().sample().is_none());
            assert_eq!(measured.spectral.clock(), clock);
            assert_eq!(measured.physical.clock(), clock);
            assert_eq!(measured.pressure.clock(), clock);
            assert_eq!(measured.regional_reference.clock(), clock);
            assert_eq!(measured.node_binding.clock(), clock);
            assert_physical(
                measured.physical,
                standalone_physical.measure(driver.ordinary()).unwrap(),
            );
            assert_pressure(
                measured.pressure,
                standalone_pressure.measure(driver.ordinary()).unwrap(),
            );
            assert_regional(
                measured.regional_reference,
                standalone_regional.measure(driver.ordinary()).unwrap(),
            );
            for branch in measured.node_binding.branches() {
                let NodeBindingStatus::Compared(node) = branch else {
                    panic!("common endpoint was not retained")
                };
                assert!(node.coefficients_equal);
            }
        } else {
            assert!(matches!(
                event.accepted().schedule(),
                AcceptedSchedule::NotScheduledAtResidualClock
            ));
            assert!(event.accepted().sample().is_none());
            assert_eq!(event.residual().schedule(), ResidualSchedule::Measured);
            let measured = event.residual().sample().unwrap();
            assert_eq!(measured.clock(), clock);
            assert_eq!(measured.reconstruction().identity(), probe_identity);
            maximum_residual_l2 = measured
                .branches()
                .iter()
                .map(|branch| branch.norms().l2)
                .fold(maximum_residual_l2, f64::max);
        }
        assert_eq!(driver.reports().len(), index + 1);
    }
    assert!(maximum_residual_l2 > 0.0);
    assert!(driver.advance().unwrap().is_none());
    assert_eq!(driver.charged_work(), plan.bounds().work);
    assert_eq!(driver.consumer_work().probes, plan.bounds().probes);
    assert_eq!(driver.consumer_work().physical, plan.bounds().physical);
    assert_eq!(
        driver.consumer_work().probe_physical,
        plan.bounds().probe_physical
    );
    assert_eq!(
        driver.consumer_work().probe_pressure,
        plan.bounds().probe_pressure
    );
    assert_eq!(driver.consumer_work().pressure, plan.bounds().pressure);
    assert_eq!(driver.consumer_work().reference, plan.bounds().reference);
    assert_eq!(driver.consumer_work().regional, plan.bounds().regional);
    assert_eq!(driver.consumer_work().residual, plan.bounds().residual);
    assert_eq!(driver.consumer_work().binding, plan.bounds().binding);
    assert_eq!(driver.ordinary().plan().identity(), family_identity);
    assert_eq!(driver.probes().plan().identity(), probe_identity);
    println!(
        "maximum off-stage residual L2={maximum_residual_l2:.17e} reconstructed_physical_work={:?}",
        plan.bounds().probe_physical
    );
}

#[test]
fn rejects_incomplete_crossed_manifests_foreign_owners_caps_and_bad_regions() {
    let accepted = clocks();
    let manifest = ticks(&[0, 7, 63, 64, 95, 127, 128]);
    let residual = ticks(&[7, 63, 95, 127]);
    let admitted = plans(&accepted, &manifest, &residual);
    assert!(DiagnosticPlan::new(
        admitted.family_plan(),
        admitted.probe_plan(),
        &residual,
        policy(),
        admitted.bounds().joint_storage_bytes - 1,
    )
    .is_err());

    let missing = ticks(&[0, 7, 63, 95, 127, 128]);
    let family = admitted.family_plan();
    let missing_probe = ProbePlan::new(
        family,
        TestedTimes::new(&missing, missing.len()).unwrap(),
        missing.len(),
        CAP,
    )
    .unwrap();
    assert!(DiagnosticPlan::new(family, missing_probe, &residual, policy(), CAP).is_err());

    let incomplete = ticks(&[7, 63, 95]);
    assert!(
        DiagnosticPlan::new(family, admitted.probe_plan(), &incomplete, policy(), CAP).is_err()
    );
    let overlapping = ticks(&[0, 7, 63, 95, 127]);
    assert!(
        DiagnosticPlan::new(family, admitted.probe_plan(), &overlapping, policy(), CAP).is_err()
    );

    let altered = FamilyPlan::new(
        settings(2e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    assert!(DiagnosticPlan::new(altered, admitted.probe_plan(), &residual, policy(), CAP).is_err());
    let mut bad_policy = policy();
    bad_policy.regional_root_budget = 127;
    assert!(
        DiagnosticPlan::new(family, admitted.probe_plan(), &residual, bad_policy, CAP).is_err()
    );
}

#[test]
fn failed_child_terminates_without_partial_publication_or_retry_charge() {
    let accepted = clocks();
    let manifest = ticks(&[0, 7, 63, 64, 95, 127, 128]);
    let residual = ticks(&[7, 63, 95, 127]);
    let family = FamilyPlan::new(
        settings(1e-40),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    let plan = DiagnosticPlan::new(family, probes, &residual, policy(), CAP).unwrap();
    let mut driver = DiagnosticDriver::new(plan).unwrap();
    assert!(driver.advance().is_err());
    assert!(driver.reports().is_empty());
    let charged = driver.charged_work();
    let consumers = driver.consumer_work();
    assert_eq!(charged.attempts, 1);
    assert!(matches!(driver.advance(), Err(DiagnosticError::Terminated)));
    assert_eq!(driver.charged_work(), charged);
    assert_eq!(driver.consumer_work(), consumers);
    assert_eq!(driver.next_time(), Some(manifest[0]));
    assert_eq!(driver.ordinary().plan().identity(), family.identity());
}
