//! Exact-v2 residuals consume only complete genuine off-stage reconstruction probes.
mod v2_family_oracle;
mod v2_family_support;

use nsbu_benchmarks::provider::V2Force;
use nsbu_benchmarks::v2_experiment::{
    probes::{
        residuals::{ResidualFamily, ResidualFamilyPlan},
        ProbeFamily, ProbePlan,
    },
    FamilyError, FamilyPlan,
};
use nsbu_solver::{
    diagnostics::conservative::ConservativeWorkspace,
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    verification::times::TestedTimes,
    Complex64,
};
use v2_family_oracle::{assert_oracle, modal_oracle};
use v2_family_support::{clocks as accepted_clocks, settings, CAP};

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn probes() -> [TickClock; 4] {
    [0, 7, 95, 128].map(clock)
}
fn probe_plan(tolerance: f64) -> ProbePlan<'static> {
    let accepted = Box::leak(Box::new(accepted_clocks()));
    let probes = Box::leak(Box::new(probes()));
    let family = FamilyPlan::new(
        settings(tolerance),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    ProbePlan::new(
        family,
        TestedTimes::new(probes, probes.len()).unwrap(),
        probes.len(),
        CAP,
    )
    .unwrap()
}

#[test]
fn early_and_late_nonstage_residuals_publish_complete_doubled_band_comparisons() {
    let plan = probe_plan(1e-5);
    let times = Box::leak(Box::new([clock(7), clock(95)]));
    let admitted = ResidualFamilyPlan::new(plan, times, 2, CAP).unwrap();
    assert_eq!(admitted.bounds().admission_geometry_checks, 12);
    let mut probes = ProbeFamily::new(plan).unwrap();
    let mut residuals = ResidualFamily::new(admitted).unwrap();
    let pairs = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];
    let mut reports = 0;
    while let Some(probe) = probes.advance().unwrap() {
        if residuals.next_time() != Some(probe.clock()) {
            continue;
        }
        let report = residuals.measure(&probes).unwrap();
        reports += 1;
        assert_eq!(report.clock(), probe.clock());
        assert_eq!(report.reconstruction().identity(), probe.identity());
        let levels = report.temporal_geometry().levels();
        assert!(levels[1].refines(levels[0]) && levels[2].refines(levels[1]));
        for (index, branch) in report.branches().iter().enumerate() {
            assert_eq!(
                branch.geometry().nodes(),
                probe.origins()[index].accepted_nodes
            );
            assert_eq!(branch.geometry().time(), probe.clock());
            assert_eq!(branch.domain(), probes.fields(index).unwrap().domain);
            assert_eq!(
                branch.force_sample_layout(),
                settings(1e-5).force.double_grid().unwrap().samples
            );
            assert!(branch.norms().l2.is_finite() && branch.norms().h1.is_finite());
        }
        for (slot, &(a, b)) in pairs.iter().enumerate() {
            let left = residuals.fields(a).unwrap();
            let right = residuals.fields(b).unwrap();
            let left_domain =
                ConservativeWorkspace::diagnostic_domain(probes.fields(a).unwrap().domain).unwrap();
            let right_domain =
                ConservativeWorkspace::diagnostic_domain(probes.fields(b).unwrap().domain).unwrap();
            assert_eq!(left.clock, probe.clock());
            assert_eq!(left.domain, left_domain);
            assert_eq!(right.domain, right_domain);
            assert_oracle(
                report.comparisons()[slot],
                modal_oracle(
                    left_domain,
                    right_domain,
                    left.coefficients,
                    right.coefficients,
                ),
            );
        }
        if probe.clock().elapsed() == 95 {
            assert_direct_residual_mode(
                &probes,
                &residuals,
                report.branches()[0].force_sample_layout(),
            );
        }
    }
    assert_eq!(reports, 2);
    assert_eq!(residuals.charged_work(), admitted.bounds().work);
    assert!(residuals.child_work().iter().all(|work| work.probes == 2));
    assert_eq!(residuals.remaining(), 0);
    assert!(!residuals.is_terminated());
    assert!(residuals.measure(&probes).is_err());
    assert!(residuals.is_terminated());
    assert!(residuals.fields(0).is_none());
}

fn assert_direct_residual_mode(
    probes: &ProbeFamily<'_>,
    residuals: &ResidualFamily<'_>,
    force_samples: nsbu_solver::domain::Layout,
) {
    let fields = probes.fields(0).unwrap();
    let source = fields.domain;
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).unwrap();
    let limits = V2Force::preflight(diagnostic, force_samples).unwrap();
    let mut force = V2Force::new(diagnostic, force_samples, limits.storage_bytes).unwrap();
    let mut forcing: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()]);
    force
        .evaluate(
            fields.clock,
            limits,
            forcing.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let actual = residuals.fields(0).unwrap();
    let mut margins = [0.0_f64; 5];
    for index in 0..diagnostic.layout().half_len() {
        let position = diagnostic.layout().position(index).unwrap();
        let target = diagnostic.layout().mode(position).unwrap();
        if target == [0; 3] || diagnostic.layout().is_nyquist(position).unwrap() {
            continue;
        }
        let measured = direct_mode(
            source,
            diagnostic,
            &fields,
            &forcing,
            actual.coefficients,
            target,
        );
        for (maximum, value) in margins.iter_mut().zip(measured) {
            *maximum = maximum.max(value);
        }
    }
    let [signal, error, omit_force_gap, flip_force_gap, omit_nonlinear_gap] = margins;
    eprintln!(
        "late residual max signal={signal:e} oracle_error={error:e} omit_force_gap={omit_force_gap:e} flip_force_gap={flip_force_gap:e} omit_nonlinear_gap={omit_nonlinear_gap:e}"
    );
    assert!(signal > 1e-8);
    assert!(error < 2e-11);
    assert!(omit_force_gap > 1e-8);
    assert!(flip_force_gap > 1e-8);
    // This low-amplitude exact-v2 trajectory is force dominated even at the
    // late probe.  The nonlinear contribution is nevertheless resolved well
    // above the direct-oracle discrepancy, so an implementation that omits it
    // fails this deliberately tighter operator-control bound.
    assert!(omit_nonlinear_gap > 1e-19);
    assert!(omit_nonlinear_gap > 20.0 * error);
}

fn direct_mode(
    source: Domain,
    diagnostic: Domain,
    fields: &nsbu_benchmarks::v2_experiment::probes::ProbeFields<'_>,
    forcing: &[Vec<Complex64>; 3],
    actual: [&[Complex64]; 3],
    target: [isize; 3],
) -> [f64; 5] {
    let wave = target.map(|mode| std::f64::consts::TAU * mode as f64);
    let mut convection = [Complex64::new(0.0, 0.0); 3];
    let dimensions = source.layout().dimensions();
    for x in (-(dimensions[0] as isize / 2) + 1)..dimensions[0] as isize / 2 {
        for y in (-(dimensions[1] as isize / 2) + 1)..dimensions[1] as isize / 2 {
            for z in (-(dimensions[2] as isize / 2) + 1)..dimensions[2] as isize / 2 {
                let left = [x, y, z];
                let right = std::array::from_fn(|axis| target[axis] - left[axis]);
                for (axis, value) in convection.iter_mut().enumerate() {
                    for (column, &wavenumber) in wave.iter().enumerate() {
                        *value += Complex64::i()
                            * wavenumber
                            * coefficient(source, fields.value[column], left)
                            * coefficient(source, fields.value[axis], right);
                    }
                }
            }
        }
    }
    let force: [Complex64; 3] =
        std::array::from_fn(|axis| coefficient(diagnostic, &forcing[axis], target));
    let raw: [Complex64; 3] = std::array::from_fn(|axis| convection[axis] - force[axis]);
    let squared = wave.iter().map(|value| value * value).sum::<f64>();
    let projected = project(wave, raw, squared);
    let omitted_force = project(wave, convection, squared);
    let flipped_force = project(
        wave,
        std::array::from_fn(|axis| convection[axis] + force[axis]),
        squared,
    );
    let omitted_nonlinearity = project(wave, std::array::from_fn(|axis| -force[axis]), squared);
    let mut signal = 0.0_f64;
    let mut error = 0.0_f64;
    let mut omit_force_gap = 0.0_f64;
    let mut flip_force_gap = 0.0_f64;
    let mut omit_nonlinear_gap = 0.0_f64;
    for (axis, &projected) in projected.iter().enumerate() {
        let common = coefficient(source, fields.derivative[axis], target)
            + source.viscosity() * squared * coefficient(source, fields.value[axis], target);
        let expected = projected + common;
        let found = coefficient(diagnostic, actual[axis], target);
        signal = signal.max(expected.l1_norm());
        error = error.max((found - expected).l1_norm());
        omit_force_gap = omit_force_gap.max((found - (omitted_force[axis] + common)).l1_norm());
        flip_force_gap = flip_force_gap.max((found - (flipped_force[axis] + common)).l1_norm());
        omit_nonlinear_gap =
            omit_nonlinear_gap.max((found - (omitted_nonlinearity[axis] + common)).l1_norm());
    }
    [
        signal,
        error,
        omit_force_gap,
        flip_force_gap,
        omit_nonlinear_gap,
    ]
}

fn project(wave: [f64; 3], value: [Complex64; 3], squared: f64) -> [Complex64; 3] {
    let dot = wave
        .iter()
        .zip(value)
        .map(|(a, b)| a * b)
        .sum::<Complex64>();
    std::array::from_fn(|axis| value[axis] - wave[axis] * dot / squared)
}

fn coefficient(domain: Domain, values: &[Complex64], mode: [isize; 3]) -> Complex64 {
    let Ok((index, conjugate)) = domain.layout().locate(mode) else {
        return Complex64::new(0.0, 0.0);
    };
    if conjugate {
        values[index].conj()
    } else {
        values[index]
    }
}

#[test]
fn invalid_bindings_caps_and_rejected_branches_publish_nothing_and_terminate() {
    let plan = probe_plan(1e-5);
    let all = probes();
    let valid = [all[1]];
    let admitted = ResidualFamilyPlan::new(plan, &valid, 1, CAP).unwrap();
    assert!(ResidualFamilyPlan::new(plan, &[], 1, CAP).is_err());
    assert!(ResidualFamilyPlan::new(plan, &valid, 0, CAP).is_err());
    assert!(
        ResidualFamilyPlan::new(plan, &valid, 1, admitted.bounds().joint_storage_bytes - 1)
            .is_err()
    );
    for invalid in [[all[0]], [all[3]], [clock(9)]] {
        assert!(ResidualFamilyPlan::new(plan, &invalid, 1, CAP).is_err());
    }
    let reversed = [all[2], all[1]];
    assert!(ResidualFamilyPlan::new(plan, &reversed, 2, CAP).is_err());

    let accepted = Box::leak(Box::new(accepted_clocks()));
    let mut undersampled = settings(1e-5);
    undersampled.force.samples = Layout::new([8; 3]).unwrap();
    assert!(FamilyPlan::new(
        undersampled,
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .is_err());

    let mut probes = ProbeFamily::new(plan).unwrap();
    let mut residuals = ResidualFamily::new(admitted).unwrap();
    assert!(residuals.measure(&probes).is_err());
    assert!(residuals.is_terminated());
    assert!(residuals.fields(0).is_none());
    let charged = residuals.charged_work();
    assert!(matches!(
        residuals.measure(&probes),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(residuals.charged_work(), charged);

    let rejected_plan = probe_plan(1e-40);
    let rejected_admission = ResidualFamilyPlan::new(rejected_plan, &valid, 1, CAP).unwrap();
    let mut rejected = ProbeFamily::new(rejected_plan).unwrap();
    assert!(rejected.advance().is_err());
    let mut consumer = ResidualFamily::new(rejected_admission).unwrap();
    assert!(consumer.measure(&rejected).is_err());
    assert!(consumer.fields(0).is_none());
    assert!(consumer.is_terminated());

    // A legal but foreign current report cannot be rebound to this residual admission.
    probes.advance().unwrap();
    probes.advance().unwrap();
    let late = [all[2]];
    let mut mistimed =
        ResidualFamily::new(ResidualFamilyPlan::new(plan, &late, 1, CAP).unwrap()).unwrap();
    assert!(mistimed.measure(&probes).is_err());
    assert!(mistimed.is_terminated());
    assert!(mistimed.fields(0).is_none());

    let other_plan = probe_plan(2e-5);
    let other_admission = ResidualFamilyPlan::new(other_plan, &valid, 1, CAP).unwrap();
    let mut other_consumer = ResidualFamily::new(other_admission).unwrap();
    assert!(other_consumer.measure(&probes).is_err());
    assert!(other_consumer.fields(0).is_none());
}
