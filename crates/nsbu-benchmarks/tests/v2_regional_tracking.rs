//! Regional analytical tracking over the six accepted exact-v2 actual states.
mod v2_family_support;

use nsbu_benchmarks::{
    regions::SpatialRegion,
    v2_experiment::{
        reference::{
            regional::{RegionalTrackingError, RegionalTrackingPlan, RegionalTrackingWorkspace},
            ReferenceTrackingPlan, ReferenceTrackingWorkspace, QUANTITIES,
        },
        FamilyError, FamilyPlan, V2Family,
    },
};
use nsbu_solver::{
    diagnostics::local::SampledError,
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];

fn plans<'a>(
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
fn regional_findings_match_the_existing_global_path_on_identical_actual_states() {
    let clocks = v2_family_support::clocks();
    let (family_plan, tracking_plan) = plans(&clocks, 3);
    let mut direct_family = V2Family::new(family_plan).unwrap();
    let mut direct = ReferenceTrackingWorkspace::new(tracking_plan).unwrap();
    let mut expected = Vec::new();
    for clock in clocks {
        direct_family.advance().unwrap();
        let report = direct.measure(&direct_family).unwrap();
        assert_eq!(report.clock(), clock);
        expected.push(report);
    }
    drop(direct);
    drop(direct_family);

    let regional_plan =
        RegionalTrackingPlan::new(tracking_plan, 128, v2_family_support::CAP).unwrap();
    assert_eq!(
        regional_plan.bounds().regional_work.classifications,
        3 * 24 * 1728
    );
    assert_eq!(
        regional_plan.bounds().regional_work.root_iterations,
        3 * 24 * 1728 * 128
    );
    let mut family = V2Family::new(family_plan).unwrap();
    let mut regional = RegionalTrackingWorkspace::new(regional_plan).unwrap();
    for (frame, clock) in clocks.into_iter().enumerate() {
        family.advance().unwrap();
        let before = digest(&family);
        let sample = regional.measure(&family).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(sample.clock(), clock);
        assert_eq!(sample.identity(), family_plan.identity());
        assert_eq!(sample.sample_layout(), Layout::new([12; 3]).unwrap());
        assert_eq!(sample.relative_floors(), FLOORS);
        for (branch, findings) in sample.branches().iter().enumerate() {
            assert_eq!(findings.branch, branch);
            for (quantity, finding) in findings.quantities.iter().enumerate() {
                assert_eq!(finding.quantity, QUANTITIES[quantity]);
                assert_eq!(
                    finding.global,
                    expected[frame].branches()[branch].quantities[quantity].error
                );
                assert_eq!(
                    finding.regional.global,
                    SampledError::Measured(finding.global)
                );
                assert_eq!(finding.regional.clock, clock);
                assert_eq!(finding.regional.dimensions, [12; 3]);
                assert!(finding.regional.grid_complete);
                assert_eq!(
                    finding.regional.components,
                    QUANTITIES[quantity].components()
                );
                assert_eq!(finding.regional.root_work_charged, 1728 * 128);
                assert_eq!(
                    finding.regional.regions.map(|entry| entry.0),
                    [
                        SpatialRegion::Core,
                        SpatialRegion::Annulus,
                        SpatialRegion::InteriorOutsideNominal,
                        SpatialRegion::Collar,
                        SpatialRegion::Exterior,
                    ]
                );
                let count: usize = finding
                    .regional
                    .regions
                    .iter()
                    .map(|(_, error)| match error {
                        SampledError::Measured(value) => value.samples,
                        SampledError::NoSamples => 0,
                    })
                    .sum();
                assert_eq!(count, 1728);
                if frame == 2 {
                    assert!(finding.regional.regions.iter().any(|(_, error)| matches!(
                        error,
                        SampledError::Measured(value) if value.rms_error > 0.0
                    )));
                }
            }
        }
        if frame == 2 {
            let finding = sample.branches()[2].quantities[2];
            let summary: Vec<_> = finding
                .regional
                .regions
                .iter()
                .map(|(region, value)| {
                    (
                        *region,
                        match value {
                            SampledError::Measured(error) => Some((error.samples, error.rms_error)),
                            SampledError::NoSamples => None,
                        },
                    )
                })
                .collect();
            println!("regional endpoint branch2 Hessian={summary:?}");
        }
    }
    assert_eq!(
        regional.regional_work(),
        regional_plan.bounds().regional_work
    );
    assert_eq!(
        regional.tracking_work(),
        regional_plan.bounds().tracking_work
    );
}

#[test]
fn regional_admission_and_missing_classes_are_explicit() {
    let clocks = v2_family_support::clocks();
    let (_, tracking) = plans(&clocks, 3);
    for root_budget in [0, 1, 127, 129, usize::MAX] {
        assert!(RegionalTrackingPlan::new(tracking, root_budget, v2_family_support::CAP).is_err());
    }
    let good = RegionalTrackingPlan::new(tracking, 128, v2_family_support::CAP).unwrap();
    assert!(
        RegionalTrackingPlan::new(tracking, 128, good.bounds().joint_storage_bytes - 1).is_err()
    );

    // The collector itself documents geometric absence without manufacturing zero error.
    let clock = clocks[0];
    let mut collector = nsbu_benchmarks::regions::RegionalTensorErrors::<3>::new(
        clock,
        Layout::new([12; 3]).unwrap(),
        128,
        1728,
        FLOORS[0],
    )
    .unwrap();
    collector.push_magnitudes(0.0, 0.0).unwrap();
    let report = collector.report().unwrap();
    assert!(!report.grid_complete);
    assert!(report
        .regions
        .iter()
        .any(|(_, error)| *error == SampledError::NoSamples));
    assert!(report
        .regions
        .iter()
        .any(|(_, error)| matches!(error, SampledError::Measured(_))));
}

#[test]
fn foreign_stale_failed_and_exhausted_requests_preserve_state_and_schedule() {
    let clocks = v2_family_support::clocks();
    let (family_plan, tracking) = plans(&clocks, 4);
    let plan = RegionalTrackingPlan::new(tracking, 128, v2_family_support::CAP).unwrap();
    let mut family = V2Family::new(family_plan).unwrap();
    family.advance().unwrap();
    let mut observer = RegionalTrackingWorkspace::new(plan).unwrap();

    let altered = FamilyPlan::new(
        v2_family_support::settings(2e-5),
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let mut foreign = V2Family::new(altered).unwrap();
    foreign.advance().unwrap();
    let before = digest(&foreign);
    assert!(observer.measure(&foreign).is_err());
    assert_eq!(digest(&foreign), before);
    assert_eq!(observer.next_time(), Some(clocks[0]));
    observer.measure(&family).unwrap();
    let before = digest(&family);
    assert!(observer.measure(&family).is_err());
    assert_eq!(digest(&family), before);
    assert_eq!(observer.next_time(), Some(clocks[1]));
    assert_eq!(observer.remaining(), 1);
    assert!(observer.measure(&foreign).is_err());
    assert_eq!(observer.remaining(), 0);
    assert!(matches!(
        observer.measure(&foreign),
        Err(RegionalTrackingError::Tracking(
            nsbu_benchmarks::v2_experiment::reference::ReferenceTrackingError::Numerical(
                SolverError::ProviderBudgetExceeded
            )
        ))
    ));

    let low = FamilyPlan::new(
        v2_family_support::settings(1e-40),
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let low_tracking = ReferenceTrackingPlan::new(
        low,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        3,
        v2_family_support::CAP,
    )
    .unwrap();
    let mut failed = V2Family::new(low).unwrap();
    let mut failed_observer = RegionalTrackingWorkspace::new(
        RegionalTrackingPlan::new(low_tracking, 128, v2_family_support::CAP).unwrap(),
    )
    .unwrap();
    failed.advance().unwrap();
    failed_observer.measure(&failed).unwrap();
    assert!(failed.advance().is_err());
    let before = digest(&failed);
    assert!(matches!(
        failed_observer.measure(&failed),
        Err(RegionalTrackingError::Tracking(
            nsbu_benchmarks::v2_experiment::reference::ReferenceTrackingError::Family(
                FamilyError::Terminated
            )
        ))
    ));
    assert_eq!(digest(&failed), before);
    assert_eq!(failed_observer.next_time(), Some(clocks[1]));
}
