//! Streaming exact-v2 reconstructed probes from six actual independent owners.
mod v2_family_oracle;
mod v2_family_support;

use nsbu_benchmarks::{
    provider::V2Force,
    v2_experiment::{
        probes::{ProbeFamily, ProbePlan},
        FamilyError, FamilyPlan,
    },
    v2_run::{ReconstructedPlan, ReconstructedRun},
};
use nsbu_solver::{
    diagnostics::conservative::ConservativeWorkspace,
    domain::{Domain, TickClock},
    integrators::forcing::PrescribedForce,
    spectral::{modal, transfer},
    verification::times::TestedTimes,
    Complex64,
};
use v2_family_oracle::{assert_oracle, modal_oracle};
use v2_family_support::{clocks as accepted_clocks, settings, CAP};

fn probes() -> [TickClock; 8] {
    [0, 7, 31, 63, 64, 95, 127, 128]
        .map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}

fn plan(tolerance: f64) -> ProbePlan<'static> {
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
fn early_and_late_probes_publish_exact_origins_and_independent_comparisons() {
    let plan = plan(1e-5);
    assert_eq!(plan.bounds().admission_geometry_checks, 48);
    assert_eq!(plan.tested_times().as_slice(), &probes());
    assert_ne!(plan.identity(), plan.family_plan().identity());
    let mut family = ProbeFamily::new(plan).unwrap();
    assert!(family.fields(0).is_none());
    let pairs = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];
    let mut late = None;

    for expected_clock in probes() {
        assert_eq!(family.next_time(), Some(expected_clock));
        let sample = family.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), expected_clock);
        assert_eq!(sample.identity(), plan.identity());
        for index in 0..6 {
            let branch = family.branch(index).unwrap();
            let fields = family.fields(index).unwrap();
            assert_eq!(fields.clock, expected_clock);
            assert_eq!(fields.origin, sample.origins()[index]);
            assert_eq!(fields.domain, branch.state().plan().domain());
            assert_eq!(
                fields.origin.accepted_nodes,
                branch.observer().last_accepted_clocks().unwrap()
            );
            assert_eq!(fields.origin.state_clock, branch.state().clock());
            assert!(fields.origin.accepted_nodes[0].elapsed() <= expected_clock.elapsed());
            assert!(fields.origin.accepted_nodes[2].elapsed() >= expected_clock.elapsed());
            assert!(fields
                .derivative
                .iter()
                .flat_map(|values| values.iter())
                .all(|z| z.re.is_finite() && z.im.is_finite()));
            if fields.origin.state_clock == expected_clock {
                for axis in 0..3 {
                    assert_eq!(fields.value[axis], branch.state().component(axis).unwrap());
                }
            }
            if index == 0 && expected_clock.elapsed() == 95 {
                late = Some((
                    fields.value.map(<[Complex64]>::to_vec),
                    fields.derivative.map(<[Complex64]>::to_vec),
                ));
            }
        }
        for (slot, &(left, right)) in pairs.iter().enumerate() {
            let a = family.fields(left).unwrap();
            let b = family.fields(right).unwrap();
            assert_oracle(
                sample.values()[slot],
                modal_oracle(a.domain, b.domain, a.value, b.value),
            );
            assert_oracle(
                sample.derivatives()[slot],
                modal_oracle(a.domain, b.domain, a.derivative, b.derivative),
            );
        }
    }
    assert_eq!(family.charged_work(), plan.bounds().work);
    assert!(family.advance().unwrap().is_none());
    assert!(family.branch(6).is_none());
    assert!(family.fields(6).is_none());
    let (value, derivative) = late.unwrap();
    assert_independent_late_probe(plan, value, derivative);
    compare_with_standalone_owners(&family, plan);
}

fn assert_independent_late_probe(
    plan: ProbePlan<'_>,
    actual: [Vec<Complex64>; 3],
    actual_derivative: [Vec<Complex64>; 3],
) {
    let settings = plan.family_plan().branch_plan(0).unwrap().settings();
    let mut run =
        ReconstructedRun::from_rest(ReconstructedPlan::from_rest(settings, CAP).unwrap()).unwrap();
    let mut retained = std::array::from_fn::<_, 3, _>(|_| None);
    while run.state().clock().elapsed() < 96 {
        run.step().unwrap();
        let elapsed = run.state().clock().elapsed();
        if [64, 80, 96].contains(&elapsed) {
            retained[((elapsed - 64) / 16) as usize] = Some(std::array::from_fn(|axis| {
                run.state().component(axis).unwrap().to_vec()
            }));
        }
    }
    let values = retained.map(Option::unwrap);
    let clocks =
        [64, 80, 96].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let derivatives: [[Vec<Complex64>; 3]; 3] =
        std::array::from_fn(|index| direct_rhs(settings.domain, clocks[index], &values[index]));
    let weights = independent_weights(31.0 / 16.0, false);
    let derivative_weights = independent_weights(31.0 / 16.0, true);
    let wrong_weights = independent_weights(7.0 / 16.0, false);
    let wrong_derivative_weights = independent_weights(7.0 / 16.0, true);
    let dt = 16.0 * 2.0_f64.powi(-20);
    let mut signal = 0.0_f64;
    let mut derivative_signal = 0.0_f64;
    let mut value_error = 0.0_f64;
    let mut derivative_error = 0.0_f64;
    let mut wrong_weight_gap = 0.0_f64;
    let mut omitted_rhs_gap = 0.0_f64;
    let mut wrong_derivative_gap = 0.0_f64;
    let mut omitted_rhs_derivative_gap = 0.0_f64;
    for axis in 0..3 {
        for index in 0..actual[axis].len() {
            let expected = (0..3).fold(Complex64::new(0.0, 0.0), |sum, node| {
                sum + weights[node] * values[node][axis][index]
                    + dt * weights[node + 3] * derivatives[node][axis][index]
            });
            value_error = value_error.max((actual[axis][index] - expected).l1_norm());
            let expected_derivative = (0..3).fold(Complex64::new(0.0, 0.0), |sum, node| {
                sum + derivative_weights[node] / dt * values[node][axis][index]
                    + derivative_weights[node + 3] * derivatives[node][axis][index]
            });
            derivative_error = derivative_error
                .max((actual_derivative[axis][index] - expected_derivative).l1_norm());
            let wrong = (0..3).fold(Complex64::new(0.0, 0.0), |sum, node| {
                sum + wrong_weights[node] * values[node][axis][index]
                    + dt * wrong_weights[node + 3] * derivatives[node][axis][index]
            });
            let omitted_rhs = (0..3).fold(Complex64::new(0.0, 0.0), |sum, node| {
                sum + weights[node] * values[node][axis][index]
            });
            let wrong_derivative = (0..3).fold(Complex64::new(0.0, 0.0), |sum, node| {
                sum + wrong_derivative_weights[node] / dt * values[node][axis][index]
                    + wrong_derivative_weights[node + 3] * derivatives[node][axis][index]
            });
            let omitted_rhs_derivative = (0..3).fold(Complex64::new(0.0, 0.0), |sum, node| {
                sum + derivative_weights[node] / dt * values[node][axis][index]
            });
            signal = signal.max(expected.l1_norm());
            derivative_signal = derivative_signal.max(expected_derivative.l1_norm());
            wrong_weight_gap = wrong_weight_gap.max((expected - wrong).l1_norm());
            omitted_rhs_gap = omitted_rhs_gap.max((expected - omitted_rhs).l1_norm());
            wrong_derivative_gap =
                wrong_derivative_gap.max((expected_derivative - wrong_derivative).l1_norm());
            omitted_rhs_derivative_gap = omitted_rhs_derivative_gap
                .max((expected_derivative - omitted_rhs_derivative).l1_norm());
        }
    }
    eprintln!(
        "late Hermite signal={signal:e} derivative_signal={derivative_signal:e} value_error={value_error:e} derivative_error={derivative_error:e} wrong_weight_gap={wrong_weight_gap:e} omitted_rhs_gap={omitted_rhs_gap:e} wrong_derivative_gap={wrong_derivative_gap:e} omitted_rhs_derivative_gap={omitted_rhs_derivative_gap:e}"
    );
    assert!(value_error < 5e-12);
    assert!(derivative_error < 5e-6);
    assert!(derivative_signal > 1e-5);
    assert!(wrong_weight_gap > 1e-10);
    assert!(omitted_rhs_gap > 2e-11);
    assert!(wrong_derivative_gap > 1e-5);
    assert!(omitted_rhs_derivative_gap > 1e-5);
}

fn independent_weights(x: f64, derivative: bool) -> [f64; 6] {
    let mut matrix = [[0.0; 6]; 6];
    for (slot, node) in [0.0_f64, 1.0, 2.0].into_iter().enumerate() {
        matrix[slot] = std::array::from_fn(|power| node.powi(power as i32));
        matrix[slot + 3] = std::array::from_fn(|power| {
            if power == 0 {
                0.0
            } else {
                power as f64 * node.powi(power as i32 - 1)
            }
        });
    }
    std::array::from_fn(|basis| {
        let mut augmented = std::array::from_fn::<_, 6, _>(|row| {
            let mut values = [0.0; 7];
            values[..6].copy_from_slice(&matrix[row]);
            values[6] = f64::from(row == basis);
            values
        });
        eliminate(&mut augmented);
        (usize::from(derivative)..6)
            .map(|power| {
                let basis = if derivative {
                    power as f64 * x.powi(power as i32 - 1)
                } else {
                    x.powi(power as i32)
                };
                augmented[power][6] * basis
            })
            .sum()
    })
}

fn eliminate(augmented: &mut [[f64; 7]; 6]) {
    for pivot in 0..6 {
        let scale = augmented[pivot][pivot];
        for value in augmented[pivot].iter_mut().skip(pivot) {
            *value /= scale;
        }
        let pivot_row = augmented[pivot];
        for (row, values) in augmented.iter_mut().enumerate() {
            if row != pivot {
                let scale = values[pivot];
                for (column, value) in values.iter_mut().enumerate().skip(pivot) {
                    *value -= scale * pivot_row[column];
                }
            }
        }
    }
}

fn direct_rhs(
    domain: Domain,
    clock: TickClock,
    velocity: &[Vec<Complex64>; 3],
) -> [Vec<Complex64>; 3] {
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let limits = V2Force::preflight(diagnostic, diagnostic.layout()).unwrap();
    let mut provider = V2Force::new(diagnostic, diagnostic.layout(), limits.storage_bytes).unwrap();
    let mut force: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()]);
    provider
        .evaluate(clock, limits, force.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    let mut products =
        ConservativeWorkspace::new(domain, ConservativeWorkspace::reservation(domain).unwrap())
            .unwrap();
    let mut conservative: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()]);
    let mut pressure = vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()];
    products
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            force.each_ref().map(Vec::as_slice),
            conservative.each_mut().map(Vec::as_mut_slice),
            &mut pressure,
        )
        .unwrap();
    std::array::from_fn(|axis| {
        let mut result = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
        transfer(
            diagnostic.layout(),
            domain.layout(),
            &conservative[axis],
            &mut result,
        )
        .unwrap();
        for (index, value) in result.iter_mut().enumerate() {
            let position = domain.layout().position(index).unwrap();
            if domain.layout().is_nyquist(position).unwrap() {
                *value = Complex64::new(0.0, 0.0);
            } else {
                let wave =
                    modal::wavevector(domain, domain.layout().mode(position).unwrap()).unwrap();
                *value = -*value
                    - domain.viscosity()
                        * wave
                            .iter()
                            .map(|component| component * component)
                            .sum::<f64>()
                        * velocity[axis][index];
            }
        }
        result
    })
}

fn compare_with_standalone_owners(family: &ProbeFamily<'_>, plan: ProbePlan<'_>) {
    for index in 0..6 {
        let settings = plan.family_plan().branch_plan(index).unwrap().settings();
        let mut direct =
            ReconstructedRun::from_rest(ReconstructedPlan::from_rest(settings, CAP).unwrap())
                .unwrap();
        let expected_clock = family.branch(index).unwrap().state().clock();
        while direct.state().clock().elapsed() < expected_clock.elapsed() {
            direct.step().unwrap();
        }
        let actual = family.branch(index).unwrap();
        assert_eq!(direct.state().clock(), actual.state().clock());
        assert_eq!(
            direct.history().records().len(),
            actual.history().records().len()
        );
        for (left, right) in direct
            .history()
            .records()
            .iter()
            .zip(actual.history().records())
        {
            assert_eq!(
                (left.start, left.outcome, left.sample),
                (right.start, right.outcome, right.sample)
            );
        }
        assert_eq!(direct.work(), actual.work());
        for axis in 0..3 {
            assert_words(
                direct.state().component(axis).unwrap(),
                actual.state().component(axis).unwrap(),
            );
        }
    }
}

fn assert_words(left: &[Complex64], right: &[Complex64]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(
            [a.re.to_bits(), a.im.to_bits()],
            [b.re.to_bits(), b.im.to_bits()]
        );
    }
}

#[test]
fn admission_and_terminal_failure_never_publish_partial_fields() {
    let accepted = accepted_clocks();
    let probe_clocks = probes();
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let times = TestedTimes::new(&probe_clocks, probe_clocks.len()).unwrap();
    let admitted = ProbePlan::new(family, times, probe_clocks.len(), CAP).unwrap();
    assert!(ProbePlan::new(family, times, 7, CAP).is_err());
    assert!(ProbePlan::new(family, times, 8, admitted.bounds().joint_storage_bytes - 1).is_err());
    let too_late = [
        probe_clocks[0],
        TickClock::restore(-20, 8192, 129, 8063).unwrap(),
    ];
    assert!(ProbePlan::new(family, TestedTimes::new(&too_late, 2).unwrap(), 2, CAP).is_err());
    let changed = probe_clocks
        .map(|clock| TickClock::restore(-19, 8192, clock.elapsed(), clock.remaining()).unwrap());
    assert!(ProbePlan::new(
        family,
        TestedTimes::new(&changed, changed.len()).unwrap(),
        8,
        CAP
    )
    .is_err());

    let mut failed = ProbeFamily::new(plan(1e-40)).unwrap();
    assert!(failed.advance().is_err());
    assert!(failed.is_terminated());
    assert!(failed.fields(0).is_none());
    assert_eq!(failed.charged_work().attempts, 1);
    let clock = failed.branch(0).unwrap().state().clock();
    let work = failed.branch(0).unwrap().work().to_vec();
    let words = (0..3)
        .flat_map(|axis| failed.branch(0).unwrap().state().component(axis).unwrap())
        .map(|value| [value.re.to_bits(), value.im.to_bits()])
        .collect::<Vec<_>>();
    let charged = failed.charged_work();
    assert!(matches!(failed.advance(), Err(FamilyError::Terminated)));
    assert_eq!(failed.charged_work(), charged);
    assert_eq!(failed.branch(0).unwrap().state().clock(), clock);
    assert_eq!(failed.branch(0).unwrap().work(), work);
    assert_eq!(
        (0..3)
            .flat_map(|axis| failed.branch(0).unwrap().state().component(axis).unwrap())
            .map(|value| [value.re.to_bits(), value.im.to_bits()])
            .collect::<Vec<_>>(),
        words
    );
}
