//! Accepted-node provenance binding across independent ordinary and probe families.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    binding::{NodeBindingError, NodeBindingPlan, NodeBindingStatus, NodeBindingWorkspace},
    probes::{ProbeFamily, ProbePlan},
    FamilyError, FamilyPlan, V2Family,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes, SolverError};
use v2_family_support::{clocks, settings, CAP};

fn probe_clocks(elapsed: &[u128]) -> Vec<TickClock> {
    elapsed
        .iter()
        .map(|&value| TickClock::restore(-20, 8192, value, 8192 - value).unwrap())
        .collect()
}

fn plans<'a>(
    accepted: &'a [TickClock; 3],
    probes: &'a [TickClock],
    attempts: usize,
) -> (FamilyPlan<'a>, ProbePlan<'a>, NodeBindingPlan<'a>) {
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probe = ProbePlan::new(
        family,
        TestedTimes::new(probes, probes.len()).unwrap(),
        probes.len(),
        CAP,
    )
    .unwrap();
    let binding = NodeBindingPlan::new(family, probe, attempts, CAP).unwrap();
    (family, probe, binding)
}

#[derive(Debug, PartialEq, Eq)]
struct Digest {
    coefficients: Vec<[u64; 2]>,
    state: Vec<[u128; 6]>,
}

fn digest(ordinary: &V2Family<'_>, probes: &ProbeFamily<'_>) -> Digest {
    let coefficients = (0..6)
        .flat_map(|branch| {
            [
                ordinary.branch(branch).unwrap().state(),
                probes.branch(branch).unwrap().state(),
            ]
            .into_iter()
            .flat_map(|state| {
                (0..3).flat_map(move |axis| {
                    state
                        .component(axis)
                        .unwrap()
                        .iter()
                        .map(|z| [z.re.to_bits(), z.im.to_bits()])
                })
            })
        })
        .collect();
    let state = (0..6)
        .flat_map(|branch| {
            let ordinary = ordinary.branch(branch).unwrap();
            let probe = probes.branch(branch).unwrap();
            [
                (
                    ordinary.state(),
                    ordinary.history().records().len(),
                    ordinary.work().len(),
                ),
                (
                    probe.state(),
                    probe.history().records().len(),
                    probe.work().len(),
                ),
            ]
            .map(|(state, history, work)| {
                [
                    state.clock().elapsed(),
                    state.clock().remaining(),
                    state.epoch().0,
                    state.accepted_steps(),
                    history as u128,
                    work as u128,
                ]
            })
        })
        .collect();
    Digest {
        coefficients,
        state,
    }
}

#[test]
fn exact_retained_nodes_bind_bitwise_without_mutating_either_family() {
    let accepted = clocks();
    let requested = probe_clocks(&[0, 64, 128]);
    let (family_plan, probe_plan, binding_plan) = plans(&accepted, &requested, 3);
    assert_eq!(binding_plan.bounds().work.node_lookups, 54);
    assert_eq!(binding_plan.bounds().work.coefficient_visits, 79_200);
    let mut ordinary = V2Family::new(family_plan).unwrap();
    let mut probes = ProbeFamily::new(probe_plan).unwrap();
    let mut binding = NodeBindingWorkspace::new(binding_plan);
    for clock in accepted {
        ordinary.advance().unwrap().unwrap();
        probes.advance().unwrap().unwrap();
        let before = digest(&ordinary, &probes);
        let sample = binding.measure(&ordinary, &probes).unwrap();
        assert_eq!(digest(&ordinary, &probes), before);
        assert_eq!(sample.clock(), clock);
        assert_eq!(sample.family_identity(), family_plan.identity());
        assert_eq!(sample.probe_identity(), probe_plan.identity());
        for status in sample.branches() {
            let NodeBindingStatus::Compared(node) = status else {
                panic!("scheduled node unexpectedly evicted");
            };
            assert_eq!(node.clock, clock);
            assert_eq!(
                node.origin,
                nsbu_benchmarks::v2_run::Origin::InternalFromRest
            );
            assert!(node.coefficients_equal);
        }
    }
    assert_eq!(binding.charged_work(), binding_plan.bounds().work);
}

#[test]
fn lookahead_eviction_is_missing_and_never_substitutes_current_or_interpolated_state() {
    let accepted = clocks();
    let requested = probe_clocks(&[0, 63, 128]);
    let (family_plan, probe_plan, binding_plan) = plans(&accepted, &requested, 3);
    let mut ordinary = V2Family::new(family_plan).unwrap();
    let mut probes = ProbeFamily::new(probe_plan).unwrap();
    ordinary.advance().unwrap();
    probes.advance().unwrap();
    probes.advance().unwrap();
    assert!(probes.branch(0).unwrap().state().clock().elapsed() > 0);
    let before = digest(&ordinary, &probes);
    let sample = NodeBindingWorkspace::new(binding_plan)
        .measure(&ordinary, &probes)
        .unwrap();
    assert_eq!(digest(&ordinary, &probes), before);
    assert_eq!(
        sample.branches(),
        [
            NodeBindingStatus::MissingRetainedNode,
            NodeBindingStatus::MissingRetainedNode,
            NodeBindingStatus::MissingRetainedNode,
            compared_at_zero(sample.branches()[3]),
            compared_at_zero(sample.branches()[4]),
            NodeBindingStatus::MissingRetainedNode,
        ]
    );
}

fn compared_at_zero(status: NodeBindingStatus) -> NodeBindingStatus {
    let NodeBindingStatus::Compared(node) = status else {
        panic!("long-step branch should retain rest");
    };
    assert_eq!(node.clock.elapsed(), 0);
    assert!(node.coefficients_equal);
    status
}

#[test]
fn admission_stale_foreign_terminal_and_exhausted_requests_retain_schedule() {
    let accepted = clocks();
    let requested = probe_clocks(&[0, 64, 128]);
    let (family_plan, probe_plan, binding_plan) = plans(&accepted, &requested, 4);
    assert!(NodeBindingPlan::new(family_plan, probe_plan, 2, CAP).is_err());
    assert!(NodeBindingPlan::new(
        family_plan,
        probe_plan,
        4,
        binding_plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(NodeBindingPlan::new(family_plan, probe_plan, usize::MAX, usize::MAX).is_err());
    let altered = FamilyPlan::new(
        settings(2e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let altered_probe = ProbePlan::new(
        altered,
        TestedTimes::new(&requested, requested.len()).unwrap(),
        requested.len(),
        CAP,
    )
    .unwrap();
    assert!(NodeBindingPlan::new(family_plan, altered_probe, 3, CAP).is_err());

    let mut ordinary = V2Family::new(family_plan).unwrap();
    let probes = ProbeFamily::new(probe_plan).unwrap();
    let mut binding = NodeBindingWorkspace::new(binding_plan);
    assert!(binding.measure(&ordinary, &probes).is_err());
    assert_eq!(binding.next_time(), Some(accepted[0]));
    ordinary.advance().unwrap();

    let foreign_times = probe_clocks(&[0, 7, 128]);
    let foreign_probe_plan = ProbePlan::new(
        family_plan,
        TestedTimes::new(&foreign_times, foreign_times.len()).unwrap(),
        foreign_times.len(),
        CAP,
    )
    .unwrap();
    let foreign = ProbeFamily::new(foreign_probe_plan).unwrap();
    assert!(matches!(
        binding.measure(&ordinary, &foreign),
        Err(NodeBindingError::Family(FamilyError::InvalidFamily))
    ));
    assert_eq!(binding.next_time(), Some(accepted[0]));
    binding.measure(&ordinary, &probes).unwrap();
    assert_eq!(binding.next_time(), Some(accepted[1]));
    assert_eq!(binding.remaining(), 1);
    assert!(binding.measure(&ordinary, &probes).is_err());
    assert_eq!(binding.remaining(), 0);
    assert!(matches!(
        binding.measure(&ordinary, &probes),
        Err(NodeBindingError::Numerical(
            SolverError::ProviderBudgetExceeded
        ))
    ));

    let low_family = FamilyPlan::new(
        settings(1e-40),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let low_probe = ProbePlan::new(
        low_family,
        TestedTimes::new(&requested, requested.len()).unwrap(),
        requested.len(),
        CAP,
    )
    .unwrap();
    let low_binding = NodeBindingPlan::new(low_family, low_probe, 3, CAP).unwrap();
    let mut low_ordinary = V2Family::new(low_family).unwrap();
    let mut failed = ProbeFamily::new(low_probe).unwrap();
    low_ordinary.advance().unwrap();
    assert!(failed.advance().is_err());
    let before = digest(&low_ordinary, &failed);
    let mut observer = NodeBindingWorkspace::new(low_binding);
    assert!(matches!(
        observer.measure(&low_ordinary, &failed),
        Err(NodeBindingError::Family(FamilyError::Terminated))
    ));
    assert_eq!(digest(&low_ordinary, &failed), before);
    assert_eq!(observer.next_time(), Some(accepted[0]));
}
