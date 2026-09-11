//! Exact-v2 reconstructed balance reports and three nested Simpson histories.
mod v2_balance_support;
mod v2_family_support;
use nsbu_benchmarks::v2_experiment::{
    probes::{
        balances::{
            quadrature::{V2BalanceQuadrature, V2QuadraturePlan},
            V2BalancePlan,
        },
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory},
    domain::TickClock,
    verification::times::TestedTimes,
};
use v2_balance_support::{assert_integral, direct_balance_with_omission, hand_simpson};
use v2_family_support::{clocks as accepted_clocks, settings, CAP};

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn fine() -> [TickClock; 9] {
    [0, 16, 32, 48, 64, 80, 96, 112, 128].map(clock)
}
fn times(values: &[TickClock]) -> TestedTimes<'_> {
    TestedTimes::new(values, values.len()).unwrap()
}
fn probe_plan(tolerance: f64) -> ProbePlan<'static> {
    let accepted = Box::leak(Box::new(accepted_clocks()));
    let probes = Box::leak(Box::new(fine()));
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
fn quadrature_plan(tolerance: f64) -> (ProbePlan<'static>, V2QuadraturePlan<'static>) {
    let probes = probe_plan(tolerance);
    let coarse = Box::leak(Box::new([0, 64, 128].map(clock)));
    let middle = Box::leak(Box::new([0, 32, 64, 96, 128].map(clock)));
    let fine = Box::leak(Box::new(fine()));
    let times = [coarse.as_slice(), middle.as_slice(), fine.as_slice()]
        .map(|set| TestedTimes::new(set, set.len()).unwrap());
    let balance = V2BalancePlan::new(probes, fine.len(), CAP).unwrap();
    let quadrature = V2QuadraturePlan::new(balance, times, 10_000, CAP).unwrap();
    (probes, quadrature)
}

#[test]
fn actual_reconstructed_balances_and_nested_simpson_match_fresh_controls() {
    let (plan, admitted) = quadrature_plan(1e-5);
    assert_eq!(admitted.bounds().maximum_half_spans, [64, 32, 16]);
    let force_samples = settings(1e-5).force.double_grid().unwrap().samples;
    assert_eq!(force_samples.dimensions(), [24; 3]);
    let mut probes = ProbeFamily::new(plan).unwrap();
    let mut quadrature = V2BalanceQuadrature::new(admitted).unwrap();
    let mut stored: Vec<[BalanceSample; 6]> = Vec::new();
    while let Some(probe) = probes.advance().unwrap() {
        assert_eq!(quadrature.charged_work().attempts, stored.len());
        let sample = quadrature.measure(&probes).unwrap();
        assert_eq!(sample.clock(), probe.clock());
        assert_eq!(sample.reconstruction().identity(), probe.identity());
        assert_eq!(sample.reconstruction().origins(), probe.origins());
        for branch in sample.branches() {
            assert!(branch.energy.is_finite() && branch.enstrophy.is_finite());
        }
        if sample.clock().elapsed() == 96 {
            for (index, &reported) in sample.branches().iter().enumerate() {
                let fields = probes.fields(index).unwrap();
                let (direct, omitted) = direct_balance_with_omission(&fields, force_samples);
                assert_eq!(direct, reported);
                let work_gap = (direct.forcing_work - omitted.forcing_work).abs();
                let vorticity_gap = (direct.vorticity_forcing - omitted.vorticity_forcing).abs();
                eprintln!(
                    "late balance branch={index} force omission work_gap={work_gap:e} vorticity_gap={vorticity_gap:e}"
                );
                assert!(work_gap.max(vorticity_gap) > 1e-12);
            }
        }
        stored.push(*sample.branches());
    }
    let report = *quadrature.report().unwrap();
    assert_eq!(report.counts(), [3, 5, 9]);
    assert_eq!(report.endpoint().clock(), clock(128));
    let fine_clocks = fine();
    for (level, stride) in [4, 2, 1].into_iter().enumerate() {
        let clocks: Vec<_> = fine_clocks.iter().step_by(stride).copied().collect();
        for branch in 0..6 {
            let samples: Vec<_> = stored
                .iter()
                .step_by(stride)
                .map(|set| set[branch])
                .collect();
            let independent = hand_simpson(&clocks, &samples);
            assert_integral(report.integrals()[level][branch], independent, 2e-18);
        }
    }
    assert_eq!(quadrature.sample_counts(), [3, 5, 9]);
    assert_eq!(
        quadrature.charged_work(),
        admitted.balance_plan().bounds().work
    );
    assert_eq!(quadrature.charged_quadrature_work(), [162, 162]);
    assert!(!quadrature.is_terminated());
    let counts = quadrature.sample_counts();
    let charged = quadrature.charged_work();
    assert!(quadrature.measure(&probes).is_err());
    assert!(quadrature.report().is_none());
    assert!(quadrature.is_terminated());
    assert_eq!(quadrature.sample_counts(), counts);
    assert_eq!(quadrature.charged_work(), charged);
}

#[test]
fn independent_simpson_polynomial_control_is_exact_to_roundoff() {
    let clocks = [0, 1, 2, 3, 4].map(clock);
    let samples = clocks.map(|time| {
        let x = time.elapsed() as f64 * 2.0_f64.powi(-20);
        BalanceSample {
            forcing_work: 3.0 * x * x + 2.0 * x + 1.0,
            vorticity_forcing: 6.0 * x * x - x + 2.0,
            ..BalanceSample::REST
        }
    });
    let mut history = BalanceHistory::new(clocks[0], samples[0], samples.len()).unwrap();
    for index in 1..clocks.len() {
        history = history.with_sample(clocks[index], samples[index]).unwrap();
    }
    let direct = hand_simpson(&clocks, &samples);
    let duration = 4.0 * 2.0_f64.powi(-20);
    let exact_energy = duration.powi(3) + duration.powi(2) + duration;
    let exact_enstrophy = 2.0 * duration.powi(3) - 0.5 * duration.powi(2) + 2.0 * duration;
    eprintln!(
        "Simpson polynomial energy_error={:e} enstrophy_error={:e}",
        (direct.energy_rhs - exact_energy).abs(),
        (direct.enstrophy_rhs - exact_enstrophy).abs()
    );
    assert!((direct.energy_rhs - exact_energy).abs() < 1e-20);
    assert!((direct.enstrophy_rhs - exact_enstrophy).abs() < 1e-20);
    assert_integral(history.integral().unwrap(), direct, 1e-20);
}

#[test]
fn invalid_admission_and_failed_bindings_publish_no_quadrature() {
    let (probe, admitted) = quadrature_plan(1e-5);
    let bounds = admitted.balance_plan().bounds();
    assert!(V2BalancePlan::new(probe, fine().len() - 1, CAP).is_err());
    assert!(V2BalancePlan::new(probe, fine().len(), bounds.joint_storage_bytes - 1).is_err());
    let sets = admitted.time_sets();
    assert!(V2QuadraturePlan::new(admitted.balance_plan(), sets, 1, CAP).is_err());
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        sets,
        10_000,
        admitted.bounds().joint_storage_bytes - 1
    )
    .is_err());
    let malformed = Box::leak(Box::new([0, 48, 64, 96, 128].map(clock)));
    let invalid_sets = [
        sets[0],
        TestedTimes::new(malformed, malformed.len()).unwrap(),
        sets[2],
    ];
    assert!(V2QuadraturePlan::new(admitted.balance_plan(), invalid_sets, 10_000, CAP).is_err());
    let missing_middle = Box::leak(Box::new([0, 16, 48, 64, 80, 96, 112, 128].map(clock)));
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [sets[0], sets[1], times(missing_middle)],
        10_000,
        CAP,
    )
    .is_err());
    let outside_owner = Box::leak(Box::new(
        [0, 8, 16, 32, 48, 64, 80, 96, 112, 128].map(clock),
    ));
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [sets[0], sets[1], times(outside_owner)],
        10_000,
        CAP,
    )
    .is_err());
    let short_coarse = Box::leak(Box::new([0, 56, 112].map(clock)));
    let short_middle = Box::leak(Box::new([0, 28, 56, 84, 112].map(clock)));
    let short_fine = Box::leak(Box::new([0, 14, 28, 42, 56, 70, 84, 98, 112].map(clock)));
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [times(short_coarse), times(short_middle), times(short_fine),],
        10_000,
        CAP,
    )
    .is_err());
    let even_coarse = Box::leak(Box::new([0, 128].map(clock)));
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [times(even_coarse), sets[1], sets[2]],
        10_000,
        CAP,
    )
    .is_err());
    let uneven_coarse = Box::leak(Box::new([0, 16, 32, 80, 128].map(clock)));
    let equal_middle = Box::leak(Box::new([0, 8, 16, 24, 32, 80, 128].map(clock)));
    let refined_fine = Box::leak(Box::new(
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 56, 80, 104, 128].map(clock),
    ));
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [
            times(uneven_coarse),
            times(equal_middle),
            times(refined_fine),
        ],
        10_000,
        CAP,
    )
    .is_err());
    let equal_fine = Box::leak(Box::new([0, 8, 16, 24, 32, 48, 64, 96, 128].map(clock)));
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [sets[0], sets[1], times(equal_fine)],
        10_000,
        CAP,
    )
    .is_err());
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [sets[0], sets[0], sets[2]],
        10_000,
        CAP,
    )
    .is_err());
    assert!(V2QuadraturePlan::new(
        admitted.balance_plan(),
        [sets[0], sets[1], sets[1]],
        10_000,
        CAP,
    )
    .is_err());

    let mut owner = ProbeFamily::new(probe).unwrap();
    let mut missing = V2BalanceQuadrature::new(admitted).unwrap();
    assert!(missing.measure(&owner).is_err());
    assert!(missing.is_terminated());
    assert!(missing.report().is_none());
    assert_eq!(missing.sample_counts(), [0; 3]);
    let charged = missing.charged_work();
    assert!(missing.measure(&owner).is_err());
    assert_eq!(missing.charged_work(), charged);

    owner.advance().unwrap();
    owner.advance().unwrap();
    let (_, mistimed_plan) = quadrature_plan(1e-5);
    let mut mistimed = V2BalanceQuadrature::new(mistimed_plan).unwrap();
    assert!(mistimed.measure(&owner).is_err());
    assert!(mistimed.report().is_none());
    assert!(mistimed.is_terminated());
}
