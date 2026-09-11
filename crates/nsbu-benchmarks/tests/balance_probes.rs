//! Refine quadrature on one unchanged reconstructed trajectory, with separate actual sample origins.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        balances::{
            quadrature::{BalanceQuadrature, QuadraturePlan},
            BalanceProbePlan,
        },
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{
    diagnostics::balances::BalanceSample, domain::TickClock, verification::times::TestedTimes,
};
use sha2::{Digest, Sha256};
fn clocks<const N: usize>(ticks: [u128; N]) -> [TickClock; N] {
    ticks.map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap())
}
fn plan<'a>(accepted: &'a [TickClock], times: &'a [TickClock]) -> ProbePlan<'a> {
    let family = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        smooth_family_support::CAP,
    )
    .unwrap();
    ProbePlan::new(
        family,
        TestedTimes::new(times, times.len()).unwrap(),
        times.len(),
        smooth_family_support::CAP,
    )
    .unwrap()
}
fn digest(family: &ProbeFamily<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    for index in 0..6 {
        let fields = family.fields(index).unwrap();
        let state = family.branch(index).unwrap().state();
        hash.update(state.clock().elapsed().to_le_bytes());
        for axis in 0..3 {
            for values in [
                fields.value[axis],
                fields.derivative[axis],
                state.component(axis).unwrap(),
            ] {
                for value in values {
                    hash.update(value.re.to_bits().to_le_bytes());
                    hash.update(value.im.to_bits().to_le_bytes());
                }
            }
        }
    }
    hash.finalize().into()
}
#[test]
fn three_quadratures_share_every_interpolated_value_without_modifying_the_six_states() {
    let accepted = smooth_family_support::clocks();
    let times = clocks([0, 7, 16, 32, 48, 64, 80, 96, 112, 128]);
    let middle = clocks([0, 32, 64, 96, 128]);
    let fine = clocks([0, 16, 32, 48, 64, 80, 96, 112, 128]);
    let probes = plan(&accepted, &times);
    let balance =
        BalanceProbePlan::new(probes, times.len() + 1, smooth_family_support::CAP).unwrap();
    let sets =
        [&accepted[..], &middle[..], &fine[..]].map(|s| TestedTimes::new(s, s.len()).unwrap());
    let admitted = QuadraturePlan::new(balance, sets, 1000, smooth_family_support::CAP).unwrap();
    assert_eq!(admitted.bounds().maximum_half_spans, [64, 32, 16]);
    let mut owner = ProbeFamily::new(probes).unwrap();
    let mut quad = BalanceQuadrature::new(admitted).unwrap();
    assert!(quad.report().is_none());
    assert!(quad.measure(&owner).is_err());
    assert_eq!(quad.charged_quadrature_work(), [18, 18]);
    for tick in times {
        owner.advance().unwrap().unwrap();
        let before = digest(&owner);
        let sample = quad.measure(&owner).unwrap();
        assert_eq!(sample.clock(), tick);
        assert_eq!(before, digest(&owner));
        if tick.elapsed() == 0 {
            assert_eq!(*sample.branches(), [BalanceSample::REST; 6]);
        } else {
            assert!(sample
                .branches()
                .iter()
                .all(|s| s.energy > 0.0 && s.forcing_work > 0.0));
        }
        if tick.elapsed() == 7 {
            assert_eq!(quad.sample_counts(), [1, 1, 1]);
        }
    }
    let report = quad.report().unwrap();
    assert_eq!(report.counts(), [3, 5, 9]);
    assert_eq!(report.endpoint().clock(), times[9]);
    for branch in 0..6 {
        let rows = report.integrals();
        let differences = [
            (rows[0][branch].energy_rhs - rows[1][branch].energy_rhs).abs(),
            (rows[1][branch].energy_rhs - rows[2][branch].energy_rhs).abs(),
        ];
        println!(
            "branch={branch} quadrature_changes={differences:?} defects={:?}",
            rows.map(|level| level[branch].energy_defect)
        );
        assert!(differences[0] > 0.0);
        assert!(differences[1] < differences[0] * 0.2);
        for level in rows {
            assert!(level[branch].energy_defect.is_finite());
            assert!(level[branch].enstrophy_defect.is_finite());
        }
    }
    assert!(!quad.is_terminated());
    assert_eq!(quad.charged_work().attempts, 11);
    assert_eq!(quad.charged_quadrature_work(), [198, 198]);
}

#[test]
fn nonnested_incomplete_uneven_and_nonshrinking_quadrature_meshes_are_refused() {
    let accepted = smooth_family_support::clocks();
    let times = clocks([
        0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128,
    ]);
    let middle = clocks([0, 32, 64, 96, 128]);
    let fine = clocks([0, 16, 32, 48, 64, 80, 96, 112, 128]);
    let probes = plan(&accepted, &times);
    let cap = smooth_family_support::CAP;
    assert!(BalanceProbePlan::new(probes, times.len() - 1, cap).is_err());
    assert!(BalanceProbePlan::new(probes, usize::MAX, cap).is_err());
    assert!(BalanceProbePlan::new(probes, times.len(), 1).is_err());
    let balance = BalanceProbePlan::new(probes, times.len(), cap).unwrap();
    let sets =
        [&accepted[..], &middle[..], &fine[..]].map(|s| TestedTimes::new(s, s.len()).unwrap());
    let admitted = QuadraturePlan::new(balance, sets, 10000, cap).unwrap();
    assert_eq!(admitted.time_sets()[2].as_slice(), fine);
    assert_eq!(admitted.balance_plan().bounds(), balance.bounds());
    assert!(QuadraturePlan::new(
        balance,
        sets,
        admitted.bounds().admission_comparisons - 1,
        cap
    )
    .is_err());
    assert!(QuadraturePlan::new(
        balance,
        sets,
        10000,
        admitted.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(QuadraturePlan::new(balance, [sets[0], sets[0], sets[2]], 10000, cap).is_err());
    let even = clocks([0, 16, 32, 64, 96, 128]);
    let uneven = clocks([0, 32, 64, 80, 128]);
    let missing = clocks([0, 4, 8, 16, 32, 48, 64, 80, 96, 112, 128]);
    for raw in [&even[..], &missing[..]] {
        let times = TestedTimes::new(raw, raw.len()).unwrap();
        assert!(QuadraturePlan::new(balance, [sets[0], sets[1], times], 10000, cap).is_err());
    }
    assert!(QuadraturePlan::new(
        balance,
        [sets[0], TestedTimes::new(&uneven, 5).unwrap(), sets[2]],
        10000,
        cap
    )
    .is_err());
    let partial = clocks([0, 16, 32, 64, 96, 112, 128]);
    let stagnant = clocks([0, 8, 16, 24, 32, 64, 96, 112, 128]);
    let levels = [
        sets[0],
        TestedTimes::new(&partial, 7).unwrap(),
        TestedTimes::new(&stagnant, 9).unwrap(),
    ];
    assert!(QuadraturePlan::new(balance, levels, 10000, cap).is_err());
}

#[test]
fn changed_physics_and_skipped_probes_spend_only_aggregate_allowances() {
    use nsbu_benchmarks::smooth_experiment::probes::balances::BalanceProbes;
    let accepted = smooth_family_support::clocks();
    let times = clocks([0, 7, 128]);
    let good = plan(&accepted, &times);
    let cap = smooth_family_support::CAP;
    let admission = BalanceProbePlan::new(good, 3, cap).unwrap();
    let mut consumer = BalanceProbes::new(admission).unwrap();
    let mut settings = smooth_family_support::settings(1e-2);
    settings.viscosity = 2.0;
    let base = FamilyPlan::new(settings, TestedTimes::new(&accepted, 3).unwrap(), cap).unwrap();
    let wrong = ProbePlan::new(base, TestedTimes::new(&times, 3).unwrap(), 3, cap).unwrap();
    let mut changed = ProbeFamily::new(wrong).unwrap();
    changed.advance().unwrap();
    assert!(consumer.measure(&changed).is_err());
    assert_eq!(consumer.child_work().map(|w| w.samples), [0; 6]);
    assert_eq!(consumer.charged_work().attempts, 1);
    assert!(!consumer.is_terminated());
    let mut owner = ProbeFamily::new(good).unwrap();
    owner.advance().unwrap();
    consumer.measure(&owner).unwrap();
    owner.advance().unwrap();
    owner.advance().unwrap();
    assert!(consumer.measure(&owner).is_err());
    assert_eq!(consumer.child_work().map(|w| w.samples), [1; 6]);
    assert_eq!(consumer.next_time(), Some(times[1]));
    assert_eq!(consumer.remaining(), 0);
    assert!(consumer.measure(&owner).is_err());
    assert_eq!(consumer.charged_work().attempts, 3);
}
