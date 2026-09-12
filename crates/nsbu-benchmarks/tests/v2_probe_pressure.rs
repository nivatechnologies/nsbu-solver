//! Actual reconstructed off-stage pressure and independent full-complex controls.
mod v2_family_support;
mod v2_probe_pressure_oracle;
use nsbu_benchmarks::runtime_force::ForceSettings;
use nsbu_benchmarks::v2_experiment::{
    probes::{
        pressure::{ProbePressurePlan, ProbePressureWorkspace},
        ProbeFamily, ProbePlan,
    },
    FamilyError, FamilyPlan,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    verification::times::TestedTimes,
    SolverError,
};
use v2_family_support::{clocks as accepted, settings, CAP};
const FLOORS: [f64; 2] = [1e-8, 1e-7];
fn times() -> [TickClock; 3] {
    [0, 95, 128].map(|e| TickClock::restore(-20, 8192, e, 8192 - e).unwrap())
}
#[test]
fn actual_offstage_pressure_matches_independent_full_complex_differences() {
    let accepted = accepted();
    let clocks = times();
    let family =
        FamilyPlan::new(settings(1e-5), TestedTimes::new(&accepted, 3).unwrap(), CAP).unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(&clocks, 3).unwrap(), 3, CAP).unwrap();
    let plan =
        ProbePressurePlan::new(probes, Layout::new([24; 3]).unwrap(), FLOORS, 3, CAP).unwrap();
    assert!(matches!(
        ProbePressurePlan::new(probes, Layout::new([12; 3]).unwrap(), FLOORS, 3, CAP),
        Err(FamilyError::Numerical(SolverError::InvalidDomain))
    ));
    assert!(ProbePressurePlan::new(probes, Layout::new([24; 3]).unwrap(), FLOORS, 2, CAP).is_err());
    assert!(ProbePressurePlan::new(
        probes,
        Layout::new([24; 3]).unwrap(),
        FLOORS,
        3,
        plan.bounds().joint_storage_bytes - 1,
    )
    .is_err());
    assert_eq!(plan.force_layout().dimensions(), [24; 3]);
    assert_eq!(plan.force_workers(), 0);
    let mut owner = ProbeFamily::new(probes).unwrap();
    let mut pressure = ProbePressureWorkspace::new(plan).unwrap();
    let mut stale = ProbePressureWorkspace::new(plan).unwrap();
    let foreign_clocks = [0, 7, 128].map(|e| TickClock::restore(-20, 8192, e, 8192 - e).unwrap());
    let foreign_family =
        FamilyPlan::new(settings(1e-5), TestedTimes::new(&accepted, 3).unwrap(), CAP).unwrap();
    let foreign_probes = ProbePlan::new(
        foreign_family,
        TestedTimes::new(&foreign_clocks, 3).unwrap(),
        3,
        CAP,
    )
    .unwrap();
    let foreign_plan = ProbePressurePlan::new(
        foreign_probes,
        Layout::new([24; 3]).unwrap(),
        FLOORS,
        3,
        CAP,
    )
    .unwrap();
    let mut foreign = ProbePressureWorkspace::new(foreign_plan).unwrap();
    let mut last = None;
    let mut first = None;
    for clock in clocks {
        let sample = owner.advance().unwrap().unwrap();
        if clock.elapsed() == 0 {
            assert!(matches!(
                foreign.measure(&owner, sample),
                Err(FamilyError::InvalidFamily)
            ));
            assert!(foreign.current().is_none());
            assert!(matches!(
                foreign.measure(&owner, sample),
                Err(FamilyError::Terminated)
            ));
        }
        if clock.elapsed() == 0 {
            first = Some(sample);
        }
        if clock.elapsed() == 95 {
            let old = first.unwrap();
            assert!(matches!(
                stale.measure(&owner, old),
                Err(FamilyError::InvalidFamily)
            ));
            assert_eq!(stale.charged_work().attempts, 1);
            assert!(stale.current().is_none());
            assert!(matches!(
                stale.measure(&owner, old),
                Err(FamilyError::Terminated)
            ));
        }
        let report = pressure.measure(&owner, sample).unwrap();
        assert_eq!(report.clock(), clock);
        assert_eq!(report.origins(), sample.origins());
        for (pair, (a, b)) in [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)]
            .into_iter()
            .enumerate()
        {
            let l = owner.fields(a).unwrap();
            let r = owner.fields(b).unwrap();
            let expected = v2_probe_pressure_oracle::pair(
                l.domain,
                l.value,
                r.domain,
                r.value,
                report.sample_layout(),
            );
            for (i, q) in report.quantities().iter().enumerate() {
                let actual = q.pairs[pair];
                let allowance = 5e-12 * (actual.reference_peak + FLOORS[i]);
                assert!((actual.rms_error - expected[i].rms).abs() < allowance);
                assert!((actual.peak_error - expected[i].peak).abs() < allowance);
            }
        }
        if clock.elapsed() == 95 {
            check_absolute_force_control(&owner, report);
        }
        last = Some(report)
    }
    assert_eq!(pressure.charged_work(), plan.bounds().work);
    let retained = last.unwrap();
    assert!(matches!(
        pressure.measure(&owner, retained.reconstruction()),
        Err(FamilyError::Numerical(SolverError::ProviderBudgetExceeded))
    ));
    assert_eq!(pressure.current().unwrap().clock(), retained.clock());
}

fn check_absolute_force_control(
    owner: &ProbeFamily<'_>,
    report: nsbu_benchmarks::v2_experiment::probes::pressure::ProbePressureSample,
) {
    let source = report.source_domain();
    let force_domain = Domain::new(
        report.force_layout().dimensions(),
        source.lengths(),
        source.viscosity(),
    )
    .unwrap();
    let settings = ForceSettings {
        samples: report.force_layout(),
        workers: report.force_workers(),
    };
    let limits = settings.limits(force_domain).unwrap();
    let mut provider = settings.build(force_domain, limits.storage_bytes).unwrap();
    let mut force: [Vec<nsbu_solver::Complex64>; 3] = std::array::from_fn(|_| {
        vec![nsbu_solver::Complex64::new(0.0, 0.0); force_domain.layout().half_len()]
    });
    provider
        .evaluate(
            report.clock(),
            limits,
            force.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let left = owner.fields(1).unwrap();
    let right = owner.fields(2).unwrap();
    let l = v2_probe_pressure_oracle::absolute(
        left.domain,
        left.value,
        force_domain,
        force.each_ref().map(Vec::as_slice),
        report.sample_layout(),
    );
    let r = v2_probe_pressure_oracle::absolute(
        right.domain,
        right.value,
        force_domain,
        force.each_ref().map(Vec::as_slice),
        report.sample_layout(),
    );
    for (index, floor) in FLOORS.into_iter().enumerate() {
        let magnitude = |p: &v2_probe_pressure_oracle::AbsolutePressure, q: usize| {
            if index == 0 {
                p.scalar[q].abs()
            } else {
                (0..3).fold(0.0_f64, |v, a| v.hypot(p.gradient[a][q]))
            }
        };
        let reference = (0..r.scalar.len())
            .map(|q| magnitude(&r, q))
            .fold(0.0, f64::max);
        let relative = (0..r.scalar.len())
            .map(|q| (magnitude(&l, q) - magnitude(&r, q)).abs() / magnitude(&r, q).max(floor))
            .fold(0.0, f64::max);
        let finding = report.quantities()[index].pairs[1];
        assert!((finding.reference_peak - reference).abs() < 2e-11 * (reference + floor));
        assert!((finding.peak_relative_error - relative).abs() < 2e-11 * (1.0 + relative));
        let zero: [Vec<nsbu_solver::Complex64>; 3] = std::array::from_fn(|_| {
            vec![nsbu_solver::Complex64::new(0.0, 0.0); force_domain.layout().half_len()]
        });
        let omitted = v2_probe_pressure_oracle::absolute(
            right.domain,
            right.value,
            force_domain,
            zero.each_ref().map(Vec::as_slice),
            report.sample_layout(),
        );
        let omitted_reference = (0..omitted.scalar.len())
            .map(|q| magnitude(&omitted, q))
            .fold(0.0, f64::max);
        assert!((reference - omitted_reference).abs() > 1e-12 * (reference + floor));
    }
}
