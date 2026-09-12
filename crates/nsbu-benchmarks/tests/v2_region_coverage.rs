//! Actual accepted-state nominal-coverage binding and refusal controls.
mod v2_family_support;

use nsbu_benchmarks::{
    regions::CoverageStatus,
    v2_experiment::{
        coverage::{CoverageFamilyPlan, CoverageFamilyWorkspace},
        reference::{
            regional::{RegionalTrackingPlan, RegionalTrackingWorkspace},
            ReferenceTrackingPlan,
        },
        FamilyPlan, V2Family,
    },
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use v2_family_support::{clocks, settings, CAP};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];

fn plans<'a>(
    times: &'a [nsbu_solver::domain::TickClock; 3],
) -> (
    FamilyPlan<'a>,
    RegionalTrackingPlan<'a>,
    CoverageFamilyPlan<'a>,
) {
    let family = FamilyPlan::new(settings(1e-5), TestedTimes::new(times, 3).unwrap(), CAP).unwrap();
    let tracking =
        ReferenceTrackingPlan::new(family, Layout::new([12; 3]).unwrap(), FLOORS, 3, CAP).unwrap();
    let regional = RegionalTrackingPlan::new(tracking, 128, CAP).unwrap();
    let coverage = CoverageFamilyPlan::new(family, [256, 512, 1024], 3, CAP).unwrap();
    (family, regional, coverage)
}

#[test]
fn actual_accepted_reports_bind_raw_nominal_coverage_without_changing_state() {
    let times = clocks();
    let (family_plan, regional_plan, coverage_plan) = plans(&times);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut regional = RegionalTrackingWorkspace::new(regional_plan).unwrap();
    let mut coverage = CoverageFamilyWorkspace::new(coverage_plan);
    family.advance().unwrap();
    let report = regional.measure(&family).unwrap();
    let before = family.branch(2).unwrap().state().component(0).unwrap()[0];
    let sample = coverage.measure(&family, report).unwrap();
    assert_eq!(sample.clock(), times[0]);
    assert_eq!(sample.family_identity(), family_plan.identity());
    assert_eq!(sample.tracking_identity(), report.identity());
    assert_eq!(sample.sampling().layout, Layout::new([12; 3]).unwrap());
    assert_eq!(sample.sampling().points, 1728);
    assert!(sample.sampling().sampled_core.iter().all(Option::is_some));
    assert!(sample
        .sampling()
        .sampled_annulus
        .iter()
        .all(Option::is_some));
    assert!(sample
        .annulus()
        .into_iter()
        .all(|value| value.status == CoverageStatus::Nonempty));
    assert!(sample
        .core()
        .into_iter()
        .all(|value| value.status == CoverageStatus::Nonempty));
    assert_eq!(
        family.branch(2).unwrap().state().component(0).unwrap()[0],
        before
    );
    assert_eq!(coverage.last_report().unwrap().clock(), times[0]);
    assert_eq!(coverage.charged_work().attempts, 1);
    assert!(coverage.measure(&family, report).is_err());
    assert_eq!(coverage.last_report().unwrap().clock(), times[0]);
    assert_eq!(coverage.charged_work().attempts, 2);

    let foreign =
        FamilyPlan::new(settings(2e-5), TestedTimes::new(&times, 3).unwrap(), CAP).unwrap();
    let mut foreign_coverage =
        CoverageFamilyWorkspace::new(CoverageFamilyPlan::new(foreign, [4, 8, 16], 3, CAP).unwrap());
    assert!(foreign_coverage.measure(&family, report).is_err());
    assert!(foreign_coverage.last_report().is_none());
    assert_eq!(foreign_coverage.charged_work().attempts, 1);
}

#[test]
fn admission_requires_nested_panels_and_joint_cap() {
    let times = clocks();
    let family =
        FamilyPlan::new(settings(1e-5), TestedTimes::new(&times, 3).unwrap(), CAP).unwrap();
    assert!(CoverageFamilyPlan::new(family, [4, 6, 12], 3, CAP).is_err());
    assert!(CoverageFamilyPlan::new(family, [4, 8, 16], 2, CAP).is_err());
    let plan = CoverageFamilyPlan::new(family, [256, 512, 1024], 3, CAP).unwrap();
    assert!(
        CoverageFamilyPlan::new(family, [4, 8, 16], 3, plan.bounds().joint_storage_bytes - 1)
            .is_err()
    );
    assert_eq!(
        plan.bounds().work.geometry_evaluations,
        3 * 2 * ((3 * 256 + 2) + (3 * 512 + 2) + (3 * 1024 + 2))
    );
}
