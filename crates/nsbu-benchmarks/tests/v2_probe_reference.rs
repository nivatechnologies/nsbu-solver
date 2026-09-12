//! Analytical tracking of actual reconstructed probe velocity fields.
mod v2_family_support;
mod v2_reference_oracle;

use nsbu_benchmarks::{
    time::BenchmarkTime,
    v2_experiment::{
        probes::{
            reference::{ProbeReferencePlan, ProbeReferenceStatus, ProbeReferenceWorkspace},
            ProbeFamily, ProbePlan, ProbeSample,
        },
        reference::{ReferenceTrackingError, QUANTITIES},
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

fn clocks() -> [TickClock; 3] {
    [0, 95, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}

fn plans<'a>(
    accepted: &'a [TickClock; 3],
    probes: &'a [TickClock; 3],
    attempts: usize,
) -> (ProbePlan<'a>, ProbeReferencePlan<'a>) {
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(probes, probes.len()).unwrap(),
        probes.len(),
        CAP,
    )
    .unwrap();
    let reference =
        ProbeReferencePlan::new(probes, Layout::new([12; 3]).unwrap(), FLOORS, attempts, CAP)
            .unwrap();
    (probes, reference)
}

fn digest(family: &ProbeFamily<'_>) -> Vec<(u64, u64)> {
    (0..6)
        .flat_map(|branch| {
            let fields = family.fields(branch).unwrap();
            fields
                .value
                .into_iter()
                .chain(fields.derivative)
                .flatten()
                .map(|value| (value.re.to_bits(), value.im.to_bits()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn late_offstage_fields_match_independent_analytical_oracle_without_mutation() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, reference_plan) = plans(&accepted, &times, 3);
    assert_eq!(
        reference_plan.probe_plan().identity(),
        probe_plan.identity()
    );
    assert_eq!(reference_plan.sample_layout().dimensions(), [12; 3]);
    assert_eq!(reference_plan.relative_floors(), FLOORS);
    assert_eq!(reference_plan.bounds().work.scalar_transforms, 3 * 270);
    assert_eq!(reference_plan.bounds().work.reference_evaluations, 3 * 1728);
    assert_eq!(reference_plan.bounds().work.binding_checks, 3 * 128);

    let mut family = ProbeFamily::new(probe_plan).unwrap();
    let mut tracking = ProbeReferenceWorkspace::new(reference_plan).unwrap();
    for (frame, clock) in times.into_iter().enumerate() {
        let raw = family.advance().unwrap().unwrap();
        let before = digest(&family);
        let report = tracking.measure(&family, raw).unwrap();
        assert_eq!(digest(&family), before);
        assert_report(&report, raw, clock, probe_plan, &family);
        if frame == 0 {
            assert!(report
                .branches()
                .iter()
                .flat_map(|branch| branch.quantities)
                .all(|finding| finding.error.rms_error == 0.0));
        } else {
            assert!(report
                .branches()
                .iter()
                .flat_map(|branch| branch.quantities)
                .all(|finding| finding.error.rms_error > 0.0));
        }
        if frame == 1 {
            compare_oracle(&family, &report);
        }
    }
    assert_eq!(tracking.charged_work(), reference_plan.bounds().work);
}

fn assert_report(
    report: &nsbu_benchmarks::v2_experiment::probes::reference::ProbeReferenceSample,
    raw: ProbeSample,
    clock: TickClock,
    plan: ProbePlan<'_>,
    family: &ProbeFamily<'_>,
) {
    assert_eq!(report.clock(), clock);
    assert_eq!(report.identity(), plan.identity());
    assert_eq!(report.reconstruction().clock(), raw.clock());
    assert_eq!(report.origins(), raw.origins());
    assert_eq!(report.relative_floors(), FLOORS);
    assert_eq!(report.case_sha256(), nsbu_benchmarks::CASE_SHA256);
    assert_eq!(report.status(), ProbeReferenceStatus::DiagnosticOnly);
    assert_eq!(
        report.source_domains(),
        std::array::from_fn(|index| family.fields(index).unwrap().domain)
    );
    for (branch, finding) in report.branches().iter().enumerate() {
        assert_eq!(finding.branch, branch);
        for (quantity, value) in finding.quantities.iter().enumerate() {
            assert_eq!(value.quantity, QUANTITIES[quantity]);
            assert_eq!(value.error.components, QUANTITIES[quantity].components());
            assert_eq!(value.error.samples, 1728);
            assert_eq!(value.error.relative_floor, FLOORS[quantity]);
            assert!(value.error.peak_error >= value.error.rms_error);
        }
    }
}

fn compare_oracle(
    family: &ProbeFamily<'_>,
    report: &nsbu_benchmarks::v2_experiment::probes::reference::ProbeReferenceSample,
) {
    let time = BenchmarkTime::new(report.clock()).unwrap();
    for branch in 0..6 {
        let fields = family.fields(branch).unwrap();
        let expected = v2_reference_oracle::tracking_view(
            fields.domain,
            fields.value,
            report.sample_layout(),
            time,
            FLOORS,
        );
        for (actual, expected) in report.branches()[branch].quantities.iter().zip(expected) {
            compare_error(actual.error, expected);
        }
    }
}

fn compare_error(
    actual: nsbu_solver::diagnostics::local::LocalError,
    expected: v2_reference_oracle::Expected,
) {
    for (measured, oracle) in [
        (actual.rms_error, expected.rms),
        (actual.peak_error, expected.peak),
        (actual.peak_relative_error, expected.relative_peak),
        (actual.reference_peak, expected.reference_peak),
    ] {
        assert!((measured - oracle).abs() < 2e-10 * (1.0 + oracle));
    }
}

#[test]
fn stale_foreign_missing_and_exhausted_publications_are_terminal() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, plan) = plans(&accepted, &times, 4);
    let mut source = ProbeFamily::new(probe_plan).unwrap();
    let raw0 = source.advance().unwrap().unwrap();

    let fresh = ProbeFamily::new(probe_plan).unwrap();
    let mut missing = ProbeReferenceWorkspace::new(plan).unwrap();
    assert!(matches!(
        missing.measure(&fresh, raw0),
        Err(ReferenceTrackingError::Family(FamilyError::InvalidFamily))
    ));
    assert!(missing.current().is_none());
    assert!(matches!(
        missing.measure(&fresh, raw0),
        Err(ReferenceTrackingError::Terminated)
    ));

    let mut tracking = ProbeReferenceWorkspace::new(plan).unwrap();
    tracking.measure(&source, raw0).unwrap();
    let retained = tracking.current().unwrap();
    source.advance().unwrap();
    assert!(tracking.measure(&source, raw0).is_err());
    assert_eq!(tracking.current().unwrap().clock(), retained.clock());

    let foreign_times =
        [0, 93, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let (foreign_plan, _) = plans(&accepted, &foreign_times, 3);
    let mut foreign = ProbeFamily::new(foreign_plan).unwrap();
    let foreign_raw = foreign.advance().unwrap().unwrap();
    let mut rejected = ProbeReferenceWorkspace::new(plan).unwrap();
    assert!(rejected.measure(&foreign, foreign_raw).is_err());

    let mut complete = ProbeFamily::new(probe_plan).unwrap();
    let mut exhausted = ProbeReferenceWorkspace::new(plan).unwrap();
    let mut latest = raw0;
    for _ in 0..3 {
        latest = complete.advance().unwrap().unwrap();
        exhausted.measure(&complete, latest).unwrap();
    }
    assert_eq!(exhausted.remaining(), 1);
    assert!(exhausted.measure(&complete, latest).is_err());
    assert_eq!(exhausted.remaining(), 0);
}

#[test]
fn admission_refuses_bad_floors_samples_attempts_caps_and_overflow() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probes, good) = plans(&accepted, &times, 3);
    let samples = Layout::new([12; 3]).unwrap();
    assert!(ProbeReferencePlan::new(probes, samples, FLOORS, 2, CAP).is_err());
    assert!(ProbeReferencePlan::new(
        probes,
        samples,
        [0.0, FLOORS[1], FLOORS[2], FLOORS[3]],
        3,
        CAP
    )
    .is_err());
    assert!(ProbeReferencePlan::new(probes, Layout::new([8; 3]).unwrap(), FLOORS, 3, CAP).is_err());
    assert!(ProbeReferencePlan::new(
        probes,
        samples,
        FLOORS,
        3,
        good.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(ProbeReferencePlan::new(probes, samples, FLOORS, usize::MAX, usize::MAX).is_err());
    assert_eq!(
        good.bounds().maximum_attempts,
        good.probe_plan().tested_times().as_slice().len()
    );
}

#[test]
fn shared_accepted_reference_regression_remains_green() {
    let columns: Vec<_> = include_str!("fixtures/derivatives.tsv")
        .lines()
        .next()
        .unwrap()
        .split('\t')
        .collect();
    let clock = TickClock::restore(-8, 2, 1, 1).unwrap();
    let value =
        nsbu_benchmarks::fields::reference::evaluate([0.0; 3], BenchmarkTime::new(clock).unwrap())
            .unwrap();
    let actual: Vec<_> = value
        .velocity
        .into_iter()
        .chain(value.gradient.into_iter().flatten())
        .chain(value.hessian.into_iter().flatten().flatten())
        .chain(value.vorticity)
        .collect();
    for (actual, expected) in actual.into_iter().zip(&columns[5..47]) {
        let expected: f64 = expected.parse().unwrap();
        assert!((actual - expected).abs() < 2e-9 * (1.0 + expected.abs()));
    }
}

#[test]
fn cap_failure_is_a_numerical_refusal() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, _) = plans(&accepted, &times, 3);
    let result = ProbeReferencePlan::new(probe_plan, Layout::new([12; 3]).unwrap(), FLOORS, 3, 0);
    assert!(matches!(
        result,
        Err(FamilyError::Numerical(SolverError::ResourceLimit))
    ));
}

#[test]
fn failed_probe_producer_cannot_publish_reference_fields() {
    let accepted = accepted_clocks();
    let times = clocks();
    let family = FamilyPlan::new(
        settings(1e-40),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&times, times.len()).unwrap(),
        times.len(),
        CAP,
    )
    .unwrap();
    let plan = ProbeReferencePlan::new(
        probes,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        times.len(),
        CAP,
    )
    .unwrap();
    let mut family = ProbeFamily::new(probes).unwrap();
    let tracking = ProbeReferenceWorkspace::new(plan).unwrap();
    let failure = family.advance().unwrap_err();
    assert!(matches!(failure, FamilyError::BranchStopped { .. }));
    assert!(family.is_terminated());
    assert!(tracking.current().is_none());
    assert_eq!(tracking.next_time(), Some(times[0]));
}
