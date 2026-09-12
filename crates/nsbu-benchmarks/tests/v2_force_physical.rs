//! Actual sampled physical force-grid differences, identity, and terminal controls.
mod v2_force_family_support;
mod v2_physical_oracle;

use nsbu_benchmarks::v2_force_experiment::{
    physical::{
        ForcePhysicalError, ForcePhysicalPlan, ForcePhysicalStatus, ForcePhysicalWorkspace,
        FORCE_PHYSICAL_QUANTITIES,
    },
    ForceFamily, ForceFamilyPlan,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes, SolverError};
use v2_force_family_support::{clocks, settings, CAP};

const FLOORS: [f64; 4] = [1e-8, 1e-7, 1e-6, 1e-7];

fn plans<'a>(
    times: &'a [nsbu_solver::domain::TickClock; 3],
) -> (ForceFamilyPlan<'a>, ForcePhysicalPlan<'a>) {
    let family = ForceFamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(times, times.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let physical = ForcePhysicalPlan::new(
        family,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        times.len(),
        CAP,
    )
    .unwrap();
    (family, physical)
}

fn digest(family: &ForceFamily<'_>) -> Vec<(u64, u64)> {
    (0..3)
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
fn actual_force_states_report_complete_physical_pairs_without_mutation() {
    let times = clocks();
    let (family_plan, physical_plan) = plans(&times);
    assert_eq!(physical_plan.bounds().work.scalar_transforms, 3 * 180);
    assert_eq!(physical_plan.sample_layout().dimensions(), [12; 3]);
    assert_eq!(physical_plan.relative_floors(), FLOORS);
    assert_eq!(
        physical_plan.family_plan().identity(),
        family_plan.identity()
    );
    let mut family = ForceFamily::from_rest(family_plan).unwrap();
    let mut physical = ForcePhysicalWorkspace::new(physical_plan).unwrap();
    let mut last_raw = None;
    for (frame, clock) in times.into_iter().enumerate() {
        let raw = family.advance().unwrap().unwrap();
        last_raw = Some(raw);
        let before = digest(&family);
        let report = physical.measure(&family, raw).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(report.clock(), clock);
        assert_eq!(report.identity(), family_plan.identity());
        assert_eq!(report.settings().force_grids, [4, 8, 16]);
        assert_eq!(report.case_sha256(), nsbu_benchmarks::CASE_SHA256);
        assert_eq!(report.status(), ForcePhysicalStatus::DiagnosticOnly);
        for (quantity, finding) in report.quantities().iter().enumerate() {
            assert_eq!(finding.quantity, FORCE_PHYSICAL_QUANTITIES[quantity]);
            for (pair, error) in finding.pairs.into_iter().enumerate() {
                assert_eq!(error.components, finding.quantity.components());
                assert_eq!(error.samples, report.sample_layout().real_len());
                assert_eq!(error.relative_floor, FLOORS[quantity]);
                assert!(error.rms_error.is_finite() && error.peak_error >= error.rms_error);
                let extrema = finding.extrema(pair).unwrap();
                let (_, linear, index, peak) = extrema.error.measured().unwrap();
                assert_eq!(peak, error.peak_error);
                if frame == 0 {
                    assert_eq!((error.rms_error, error.peak_error), (0.0, 0.0));
                    assert_eq!((linear, index), (0, [0; 3]));
                    assert_eq!(
                        extrema.relative_error.measured().unwrap(),
                        (report.sample_layout(), 0, [0; 3], 0.0)
                    );
                    assert_eq!(
                        extrema.reference.measured().unwrap(),
                        (report.sample_layout(), 0, [0; 3], 0.0)
                    );
                }
            }
        }
        if frame > 0 {
            compare_oracle(&family, &report);
        }
    }
    assert_eq!(physical.charged_work(), physical_plan.bounds().work);
    let current = physical.current().unwrap();
    assert!(family.advance().unwrap().is_none());
    assert!(matches!(
        physical.measure(&family, last_raw.unwrap()),
        Err(ForcePhysicalError::Numerical(
            SolverError::ProviderBudgetExceeded
        ))
    ));
    assert_eq!(physical.current().unwrap().clock(), current.clock());
    assert!(matches!(
        physical.measure(&family, last_raw.unwrap()),
        Err(ForcePhysicalError::Terminated)
    ));
}

fn compare_oracle(
    family: &ForceFamily<'_>,
    report: &nsbu_benchmarks::v2_force_experiment::physical::ForcePhysicalSample,
) {
    for (pair, (left, right)) in [(0, 1), (1, 2)].into_iter().enumerate() {
        let expected = v2_physical_oracle::pair_errors(
            family.branch(left).unwrap().state(),
            family.branch(right).unwrap().state(),
            report.sample_layout(),
            FLOORS,
        );
        for (quantity, (finding, oracle)) in report.quantities().iter().zip(expected).enumerate() {
            let actual = finding.pairs[pair];
            let allowance = 2e-11 * (FLOORS[quantity] + oracle.rms.max(oracle.peak));
            assert!((actual.rms_error - oracle.rms).abs() < allowance);
            assert!((actual.peak_error - oracle.peak).abs() < allowance);
            assert!(
                (actual.reference_peak - oracle.reference_peak).abs()
                    < 2e-11 * (FLOORS[quantity] + oracle.reference_peak)
            );
            assert!(
                (actual.peak_relative_error - oracle.relative_peak).abs()
                    < 2e-11 * (1.0 + oracle.relative_peak)
            );
            assert!(actual.rms_error > 0.0 && actual.peak_error > 0.0);
            let extrema = finding.extrema(pair).unwrap();
            let error_peak = extrema.error.measured().unwrap();
            let relative_peak = extrema.relative_error.measured().unwrap();
            let reference_peak = extrema.reference.measured().unwrap();
            let error_point = v2_physical_oracle::point_magnitudes(
                family.branch(left).unwrap().state(),
                family.branch(right).unwrap().state(),
                report.sample_layout(),
                error_peak.1,
            );
            let relative_point = v2_physical_oracle::point_magnitudes(
                family.branch(left).unwrap().state(),
                family.branch(right).unwrap().state(),
                report.sample_layout(),
                relative_peak.1,
            );
            let reference_point = v2_physical_oracle::point_magnitudes(
                family.branch(left).unwrap().state(),
                family.branch(right).unwrap().state(),
                report.sample_layout(),
                reference_peak.1,
            );
            check_peak(
                error_peak,
                report.sample_layout(),
                oracle.peak_linear,
                oracle.peak,
                error_point.0[quantity],
                allowance,
            );
            check_peak(
                relative_peak,
                report.sample_layout(),
                oracle.relative_linear,
                oracle.relative_peak,
                relative_point.0[quantity] / relative_point.1[quantity].max(FLOORS[quantity]),
                2e-11 * (1.0 + oracle.relative_peak),
            );
            check_peak(
                reference_peak,
                report.sample_layout(),
                oracle.reference_linear,
                oracle.reference_peak,
                reference_point.1[quantity],
                2e-11 * (FLOORS[quantity] + oracle.reference_peak),
            );
            println!(
                "pair={pair} quantity={:?} rms={:.17e} peak={:.17e}",
                finding.quantity, actual.rms_error, actual.peak_error
            );
        }
    }
}

fn check_peak(
    actual: (Layout, usize, [usize; 3], f64),
    layout: Layout,
    linear: usize,
    maximum: f64,
    point_value: f64,
    allowance: f64,
) {
    let [_, ny, nz] = layout.dimensions();
    let expected_index = [linear / (ny * nz), (linear / nz) % ny, linear % nz];
    assert_eq!(actual.0, layout);
    assert_eq!(
        actual.2,
        [actual.1 / (ny * nz), (actual.1 / nz) % ny, actual.1 % nz,]
    );
    assert!((actual.3 - point_value).abs() < allowance);
    assert!((actual.3 - maximum).abs() < allowance);
    if actual.1 == linear {
        assert_eq!(actual.2, expected_index);
    }
}

#[test]
fn refusals_charge_once_terminate_and_retain_prior_report() {
    let times = clocks();
    let (family_plan, physical_plan) = plans(&times);
    assert!(
        ForcePhysicalPlan::new(family_plan, Layout::new([12; 3]).unwrap(), FLOORS, 2, CAP).is_err()
    );
    assert!(ForcePhysicalPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        [0.0, 1.0, 1.0, 1.0],
        3,
        CAP
    )
    .is_err());
    assert!(ForcePhysicalPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        FLOORS,
        3,
        physical_plan.bounds().joint_storage_bytes - 1
    )
    .is_err());

    let mut source = ForceFamily::from_rest(family_plan).unwrap();
    let raw = source.advance().unwrap().unwrap();
    let missing = ForceFamily::from_rest(family_plan).unwrap();
    let mut missing_measurement = ForcePhysicalWorkspace::new(physical_plan).unwrap();
    assert!(matches!(
        missing_measurement.measure(&missing, raw),
        Err(ForcePhysicalError::InvalidFamily)
    ));
    assert!(missing_measurement.current().is_none());
    assert_eq!(missing_measurement.charged_work().attempts, 1);

    let mut family = ForceFamily::from_rest(family_plan).unwrap();
    let raw0 = family.advance().unwrap().unwrap();
    let mut physical = ForcePhysicalWorkspace::new(physical_plan).unwrap();
    physical.measure(&family, raw0).unwrap();
    let prior = physical.current().unwrap();
    let _raw1 = family.advance().unwrap().unwrap();
    assert!(matches!(
        physical.measure(&family, raw0),
        Err(ForcePhysicalError::InvalidFamily)
    ));
    assert_eq!(physical.current().unwrap().clock(), prior.clock());
    let charged = physical.charged_work();
    assert!(matches!(
        physical.measure(&family, raw0),
        Err(ForcePhysicalError::Terminated)
    ));
    assert_eq!(physical.charged_work(), charged);

    let foreign_plan = ForceFamilyPlan::new(
        settings(2e-5),
        TestedTimes::new(&times, times.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let mut foreign = ForceFamily::from_rest(foreign_plan).unwrap();
    let foreign_raw = foreign.advance().unwrap().unwrap();
    let mut physical = ForcePhysicalWorkspace::new(physical_plan).unwrap();
    assert!(matches!(
        physical.measure(&foreign, foreign_raw),
        Err(ForcePhysicalError::InvalidFamily)
    ));
    assert!(physical.current().is_none());
    assert_eq!(physical.charged_work().attempts, 1);
}

#[test]
fn failed_family_is_rejected_without_replacing_rest_report() {
    let times = clocks();
    let family_plan = ForceFamilyPlan::new(
        settings(1e-40),
        TestedTimes::new(&times, times.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let physical_plan =
        ForcePhysicalPlan::new(family_plan, Layout::new([12; 3]).unwrap(), FLOORS, 3, CAP).unwrap();
    let mut family = ForceFamily::from_rest(family_plan).unwrap();
    let rest = family.advance().unwrap().unwrap();
    let mut physical = ForcePhysicalWorkspace::new(physical_plan).unwrap();
    physical.measure(&family, rest).unwrap();
    assert!(family.advance().is_err());
    assert!(matches!(
        physical.measure(&family, rest),
        Err(ForcePhysicalError::InvalidFamily)
    ));
    assert_eq!(physical.current().unwrap().clock().elapsed(), 0);
    assert!(matches!(
        physical.measure(&family, rest),
        Err(ForcePhysicalError::Terminated)
    ));
}
