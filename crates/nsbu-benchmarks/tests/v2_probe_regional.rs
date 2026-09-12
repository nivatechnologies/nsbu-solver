//! Sampled regional analytical tracking of actual reconstructed probe fields.
mod v2_family_support;

use nsbu_benchmarks::{
    regions::{SampledError, SpatialRegion},
    v2_experiment::{
        probes::{
            reference::{ProbeReferencePlan, ProbeReferenceWorkspace},
            regional::{
                ProbeRegionalError, ProbeRegionalPlan, ProbeRegionalStatus, ProbeRegionalWorkspace,
            },
            ProbeFamily, ProbePlan,
        },
        reference::QUANTITIES,
        FamilyError, FamilyPlan,
    },
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};
use v2_family_support::{clocks as accepted_clocks, settings, CAP};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];
fn clocks() -> [TickClock; 7] {
    [0, 7, 63, 64, 95, 127, 128].map(|n| TickClock::restore(-20, 8192, n, 8192 - n).unwrap())
}
fn plans<'a>(
    accepted: &'a [TickClock; 3],
    times: &'a [TickClock; 7],
    attempts: usize,
) -> (ProbePlan<'a>, ProbeReferencePlan<'a>, ProbeRegionalPlan<'a>) {
    let family =
        FamilyPlan::new(settings(1e-5), TestedTimes::new(accepted, 3).unwrap(), CAP).unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(times, 7).unwrap(), 7, CAP).unwrap();
    let reference =
        ProbeReferencePlan::new(probes, Layout::new([12; 3]).unwrap(), FLOORS, attempts, CAP)
            .unwrap();
    let regional = ProbeRegionalPlan::new(reference, 128, CAP).unwrap();
    (probes, reference, regional)
}
fn digest(family: &ProbeFamily<'_>) -> Vec<(u64, u64)> {
    (0..6)
        .flat_map(|branch| {
            family
                .fields(branch)
                .unwrap()
                .value
                .into_iter()
                .flatten()
                .map(|v| (v.re.to_bits(), v.im.to_bits()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn all_probe_clocks_match_independent_global_producer_and_preserve_fields() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, reference_plan, regional_plan) = plans(&accepted, &times, 7);
    assert_eq!(
        regional_plan.bounds().regional_work.classifications,
        7 * 24 * 1728
    );
    assert_eq!(
        regional_plan.bounds().tracking_work.scalar_transforms,
        7 * 270
    );
    let mut family = ProbeFamily::new(probe_plan).unwrap();
    let mut reference = ProbeReferenceWorkspace::new(reference_plan).unwrap();
    let mut regional = ProbeRegionalWorkspace::new(regional_plan).unwrap();
    for (frame, clock) in times.into_iter().enumerate() {
        let raw = family.advance().unwrap().unwrap();
        let before = digest(&family);
        let expected = reference.measure(&family, raw).unwrap();
        let report = regional.measure(&family, raw, expected).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(report.clock(), clock);
        assert_eq!(report.identity(), probe_plan.identity());
        assert_eq!(report.origins(), raw.origins());
        assert_eq!(report.status(), ProbeRegionalStatus::DiagnosticOnly);
        for branch in 0..6 {
            for quantity in 0..4 {
                let finding = report.branches()[branch].quantities[quantity];
                assert_eq!(finding.quantity, QUANTITIES[quantity]);
                assert_eq!(
                    finding.global,
                    expected.branches()[branch].quantities[quantity].error
                );
                assert_eq!(
                    finding.regional.global,
                    SampledError::Measured(finding.global)
                );
                assert_eq!(finding.regional.clock, clock);
                assert_eq!(finding.regional.dimensions, [12; 3]);
                assert_eq!(finding.regional.root_work_charged, 1728 * 128);
                assert_eq!(
                    finding.regional.regions.map(|v| v.0),
                    [
                        SpatialRegion::Core,
                        SpatialRegion::Annulus,
                        SpatialRegion::InteriorOutsideNominal,
                        SpatialRegion::Collar,
                        SpatialRegion::Exterior
                    ]
                );
                let samples: usize = finding
                    .regional
                    .regions
                    .iter()
                    .map(|v| match v.1 {
                        SampledError::Measured(e) => e.samples,
                        SampledError::NoSamples => 0,
                    })
                    .sum();
                assert_eq!(samples, 1728);
                if frame == 4 {
                    assert!(finding.global.rms_error > 0.0);
                }
            }
        }
    }
    assert_eq!(
        regional.tracking_work(),
        regional_plan.bounds().tracking_work
    );
    assert_eq!(
        regional.regional_work(),
        regional_plan.bounds().regional_work
    );
}

#[test]
fn stale_publication_is_terminal_and_retains_last_complete_report() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, reference_plan, regional_plan) = plans(&accepted, &times, 8);
    let mut family = ProbeFamily::new(probe_plan).unwrap();
    let mut reference = ProbeReferenceWorkspace::new(reference_plan).unwrap();
    let mut regional = ProbeRegionalWorkspace::new(regional_plan).unwrap();
    let raw0 = family.advance().unwrap().unwrap();
    let ref0 = reference.measure(&family, raw0).unwrap();
    regional.measure(&family, raw0, ref0).unwrap();
    let retained = regional.current().unwrap();
    family.advance().unwrap();
    assert!(matches!(
        regional.measure(&family, raw0, ref0),
        Err(ProbeRegionalError::Tracking(_))
    ));
    assert_eq!(regional.current().unwrap().clock(), retained.clock());
    assert_eq!(regional.tracking_work().attempts, 2);
    assert!(matches!(
        regional.measure(&family, raw0, ref0),
        Err(ProbeRegionalError::Tracking(
            nsbu_benchmarks::v2_experiment::reference::ReferenceTrackingError::Terminated
        ))
    ));
    assert_eq!(regional.tracking_work().attempts, 2);
}

#[test]
fn admission_refuses_bad_roots_caps_attempts_and_overflow() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (_, reference, good) = plans(&accepted, &times, 7);
    for roots in [0, 1, 127, 129, usize::MAX] {
        assert!(ProbeRegionalPlan::new(reference, roots, CAP).is_err());
    }
    assert!(ProbeRegionalPlan::new(reference, 128, good.bounds().joint_storage_bytes - 1).is_err());
    let probes = reference.probe_plan();
    assert!(ProbeReferencePlan::new(
        probes,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        usize::MAX,
        usize::MAX
    )
    .is_err());
    assert!(
        ProbeReferencePlan::new(probes, Layout::new([12; 3]).unwrap(), FLOORS, 6, CAP).is_err()
    );
    let fresh = ProbeFamily::new(probes).unwrap();
    let mut regional = ProbeRegionalWorkspace::new(good).unwrap();
    let fake_family =
        FamilyPlan::new(settings(2e-5), TestedTimes::new(&accepted, 3).unwrap(), CAP).unwrap();
    let fake_plan =
        ProbePlan::new(fake_family, TestedTimes::new(&times, 7).unwrap(), 7, CAP).unwrap();
    let mut foreign = ProbeFamily::new(fake_plan).unwrap();
    let raw = foreign.advance().unwrap().unwrap();
    let mut foreign_ref = ProbeReferenceWorkspace::new(
        ProbeReferencePlan::new(fake_plan, Layout::new([12; 3]).unwrap(), FLOORS, 7, CAP).unwrap(),
    )
    .unwrap();
    let report = foreign_ref.measure(&foreign, raw).unwrap();
    assert!(matches!(
        regional.measure(&fresh, raw, report),
        Err(ProbeRegionalError::Tracking(
            nsbu_benchmarks::v2_experiment::reference::ReferenceTrackingError::Family(
                FamilyError::InvalidFamily
            )
        ))
    ));
    assert!(regional.current().is_none());
    let _ = SolverError::InvalidPayload;
}
