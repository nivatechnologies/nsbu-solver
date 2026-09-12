//! Actual reconstructed physical comparisons and independent signed-Fourier controls.
mod v2_family_support;
mod v2_physical_oracle;

use nsbu_benchmarks::v2_experiment::{
    probes::{
        physical::{
            ProbePhysicalPlan, ProbePhysicalSample, ProbePhysicalStatus, ProbePhysicalWorkspace,
            PROBE_PHYSICAL_QUANTITIES,
        },
        ProbeFamily, ProbePlan,
    },
    FamilyError, FamilyPlan,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
    SolverError,
};
use v2_family_support::{clocks as accepted_clocks, settings, CAP};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];
const PAIRS: [(usize, usize); 5] = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];

#[derive(Clone, Copy)]
struct Views<'a> {
    domain: nsbu_solver::domain::Domain,
    values: [&'a [nsbu_solver::Complex64]; 3],
}

fn clocks() -> [TickClock; 3] {
    [0, 7, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}

fn plans<'a>(
    accepted: &'a [TickClock; 3],
    probes: &'a [TickClock; 3],
) -> (ProbePlan<'a>, ProbePhysicalPlan<'a>) {
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
    let physical = ProbePhysicalPlan::new(
        probes,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        probes.tested_times().as_slice().len(),
        CAP,
    )
    .unwrap();
    (probes, physical)
}

fn digest(family: &ProbeFamily<'_>) -> Vec<(u64, u64)> {
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
                    .map(|z| (z.re.to_bits(), z.im.to_bits()))
            })
        })
        .collect()
}

#[test]
fn offstage_and_endpoint_reports_match_complete_independent_oracle() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, physical_plan) = plans(&accepted, &times);
    assert_eq!(physical_plan.probe_plan().identity(), probe_plan.identity());
    assert_eq!(physical_plan.sample_layout().dimensions(), [12; 3]);
    assert_eq!(physical_plan.relative_floors(), FLOORS);
    assert_eq!(physical_plan.bounds().work.binding_checks, 3 * 64);
    let mut family = ProbeFamily::new(probe_plan).unwrap();
    let mut physical = ProbePhysicalWorkspace::new(physical_plan).unwrap();
    let mut raw = None;
    for (frame, clock) in times.into_iter().enumerate() {
        let sample = family.advance().unwrap().unwrap();
        raw = Some(sample);
        let before = digest(&family);
        let report = physical.measure(&family, sample).unwrap();
        assert_eq!(digest(&family), before);
        assert_report_identity(&report, sample, clock, probe_plan, &family);
        compare_oracle(&family, &report, frame == 0);
    }
    assert_eq!(physical.charged_work(), physical_plan.bounds().work);
    let retained = physical.current().unwrap();
    assert!(matches!(
        physical.measure(&family, raw.unwrap()),
        Err(FamilyError::Numerical(SolverError::ProviderBudgetExceeded))
    ));
    assert_eq!(physical.current().unwrap().clock(), retained.clock());
    let charged = physical.charged_work();
    assert!(matches!(
        physical.measure(&family, raw.unwrap()),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(physical.charged_work(), charged);
}

fn assert_report_identity(
    report: &ProbePhysicalSample,
    raw: nsbu_benchmarks::v2_experiment::probes::ProbeSample,
    clock: TickClock,
    plan: ProbePlan<'_>,
    family: &ProbeFamily<'_>,
) {
    assert_eq!(report.clock(), clock);
    assert_eq!(report.identity(), plan.identity());
    assert_eq!(report.reconstruction().clock(), raw.clock());
    assert_eq!(report.origins(), raw.origins());
    assert_eq!(report.case_sha256(), nsbu_benchmarks::CASE_SHA256);
    assert_eq!(report.status(), ProbePhysicalStatus::DiagnosticOnly);
    assert_eq!(
        report.source_domains(),
        std::array::from_fn(|i| family.fields(i).unwrap().domain)
    );
}

fn compare_oracle(family: &ProbeFamily<'_>, report: &ProbePhysicalSample, rest: bool) {
    for (pair, (left, right)) in PAIRS.into_iter().enumerate() {
        let left = family.fields(left).unwrap();
        let right = family.fields(right).unwrap();
        let views = [
            Views {
                domain: left.domain,
                values: left.value,
            },
            Views {
                domain: right.domain,
                values: right.value,
            },
        ];
        let expected = v2_physical_oracle::view_errors(
            left.domain,
            left.value,
            right.domain,
            right.value,
            report.sample_layout(),
            FLOORS,
        );
        for (quantity, (finding, oracle)) in report.quantities().iter().zip(expected).enumerate() {
            assert_eq!(finding.quantity, PROBE_PHYSICAL_QUANTITIES[quantity]);
            let actual = finding.pairs[pair];
            assert_eq!(actual.components, finding.quantity.components());
            assert_eq!(actual.samples, report.sample_layout().real_len());
            assert_eq!(actual.relative_floor, FLOORS[quantity]);
            let allowance = 2e-11 * (FLOORS[quantity] + oracle.rms.max(oracle.peak));
            close(actual.rms_error, oracle.rms, allowance);
            close(actual.peak_error, oracle.peak, allowance);
            close(
                actual.reference_peak,
                oracle.reference_peak,
                2e-11 * (FLOORS[quantity] + oracle.reference_peak),
            );
            close(
                actual.peak_relative_error,
                oracle.relative_peak,
                2e-11 * (1.0 + oracle.relative_peak),
            );
            let extrema = finding.extrema(pair).unwrap();
            check_witness(
                report,
                views,
                quantity,
                extrema.error.measured().unwrap(),
                oracle.peak,
                allowance,
                false,
            );
            check_witness(
                report,
                views,
                quantity,
                extrema.relative_error.measured().unwrap(),
                oracle.relative_peak,
                2e-11 * (1.0 + oracle.relative_peak),
                true,
            );
            check_reference(
                report,
                views,
                quantity,
                extrema.reference.measured().unwrap(),
                oracle.reference_peak,
            );
            if rest {
                assert_eq!((actual.rms_error, actual.peak_error), (0.0, 0.0));
                assert_eq!(extrema.error.measured().unwrap().1, 0);
                assert_eq!(extrema.relative_error.measured().unwrap().1, 0);
                assert_eq!(extrema.reference.measured().unwrap().1, 0);
            }
        }
    }
    if !rest {
        assert!(report
            .quantities()
            .iter()
            .flat_map(|q| q.pairs)
            .all(|e| e.rms_error > 0.0));
    }
}

fn check_witness(
    report: &ProbePhysicalSample,
    views: [Views<'_>; 2],
    quantity: usize,
    actual: (Layout, usize, [usize; 3], f64),
    maximum: f64,
    allowance: f64,
    relative: bool,
) {
    let point = v2_physical_oracle::view_point_magnitudes(
        views[0].domain,
        views[0].values,
        views[1].domain,
        views[1].values,
        report.sample_layout(),
        actual.1,
    );
    let value = if relative {
        point.0[quantity] / point.1[quantity].max(FLOORS[quantity])
    } else {
        point.0[quantity]
    };
    check_location(actual, report.sample_layout());
    close(actual.3, value, allowance);
    close(actual.3, maximum, allowance);
}

fn check_reference(
    report: &ProbePhysicalSample,
    views: [Views<'_>; 2],
    quantity: usize,
    actual: (Layout, usize, [usize; 3], f64),
    maximum: f64,
) {
    let point = v2_physical_oracle::view_point_magnitudes(
        views[0].domain,
        views[0].values,
        views[1].domain,
        views[1].values,
        report.sample_layout(),
        actual.1,
    );
    check_location(actual, report.sample_layout());
    let allowance = 2e-11 * (FLOORS[quantity] + maximum);
    close(actual.3, point.1[quantity], allowance);
    close(actual.3, maximum, allowance);
}

fn check_location(actual: (Layout, usize, [usize; 3], f64), layout: Layout) {
    let [_, ny, nz] = layout.dimensions();
    assert_eq!(actual.0, layout);
    assert_eq!(
        actual.2,
        [actual.1 / (ny * nz), (actual.1 / nz) % ny, actual.1 % nz]
    );
}

fn close(actual: f64, expected: f64, allowance: f64) {
    assert!(
        (actual - expected).abs() <= allowance,
        "actual={actual:e} expected={expected:e} allowance={allowance:e}"
    );
}

#[test]
fn admission_and_publication_refusals_are_bounded_and_terminal() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (probe_plan, plan) = plans(&accepted, &times);
    assert!(
        ProbePhysicalPlan::new(probe_plan, Layout::new([12; 3]).unwrap(), FLOORS, 2, CAP).is_err()
    );
    assert!(ProbePhysicalPlan::new(
        probe_plan,
        Layout::new([12; 3]).unwrap(),
        [0.0, 1.0, 1.0, 1.0],
        3,
        CAP
    )
    .is_err());
    assert!(ProbePhysicalPlan::new(
        probe_plan,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        3,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());

    let mut source = ProbeFamily::new(probe_plan).unwrap();
    let raw0 = source.advance().unwrap().unwrap();
    let fresh = ProbeFamily::new(probe_plan).unwrap();
    let mut missing = ProbePhysicalWorkspace::new(plan).unwrap();
    assert!(matches!(
        missing.measure(&fresh, raw0),
        Err(FamilyError::InvalidFamily)
    ));
    assert!(missing.current().is_none());
    assert_eq!(missing.charged_work().attempts, 1);
    assert!(matches!(
        missing.measure(&fresh, raw0),
        Err(FamilyError::Terminated)
    ));

    let mut physical = ProbePhysicalWorkspace::new(plan).unwrap();
    physical.measure(&source, raw0).unwrap();
    let prior = physical.current().unwrap();
    source.advance().unwrap();
    assert!(matches!(
        physical.measure(&source, raw0),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(physical.current().unwrap().clock(), prior.clock());

    let foreign_times =
        [0, 9, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let (foreign_plan, _) = plans(&accepted, &foreign_times);
    let mut foreign = ProbeFamily::new(foreign_plan).unwrap();
    let foreign_raw = foreign.advance().unwrap().unwrap();
    let mut physical = ProbePhysicalWorkspace::new(plan).unwrap();
    assert!(matches!(
        physical.measure(&foreign, foreign_raw),
        Err(FamilyError::InvalidFamily)
    ));
}

#[test]
fn failed_probe_producer_cannot_replace_complete_report() {
    let accepted = accepted_clocks();
    let times = clocks();
    let (normal_plan, physical_plan) = plans(&accepted, &times);
    let mut normal = ProbeFamily::new(normal_plan).unwrap();
    let rest = normal.advance().unwrap().unwrap();
    let mut physical = ProbePhysicalWorkspace::new(physical_plan).unwrap();
    physical.measure(&normal, rest).unwrap();

    let family = FamilyPlan::new(
        settings(1e-40),
        TestedTimes::new(&accepted, 3).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(&times, 3).unwrap(), 3, CAP).unwrap();
    let mut family = ProbeFamily::new(probes).unwrap();
    assert!(family.advance().is_err());
    assert!(family.is_terminated());
    assert!(matches!(
        physical.measure(&family, rest),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(physical.current().unwrap().clock().elapsed(), 0);
}
