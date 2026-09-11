//! Actual-run, identity and terminal-state checks for force-sampling refinement.
mod v2_family_oracle;
mod v2_force_family_support;
use v2_family_oracle::{assert_oracle, modal_oracle};
use v2_force_family_support::{clocks, settings, CAP};

use nsbu_benchmarks::{
    v2_force_experiment::{ForceFamily, ForceFamilyError, ForceFamilyPlan},
    v2_run::Origin,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan, domain::TickClock, integrators::method::Method,
    verification::times::TestedTimes,
};

#[test]
fn three_private_rest_trajectories_report_both_complete_force_pairs() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let plan = ForceFamilyPlan::new(settings(1e-5), times, CAP).unwrap();
    assert_eq!(plan.times().as_slice(), &clocks);
    assert_eq!(plan.bounds().attempts, 24);
    assert!(plan.bounds().storage_bytes < CAP);
    assert!(plan.bounds().integration_calls > plan.bounds().attempts);
    assert!(plan.bounds().provider_work_units > 0);
    assert!(plan.bounds().scalar_transforms > 0);
    assert!(plan.bounds().comparison_work_units > 0);
    assert!(plan.branch_plan(2).is_some());
    assert!(plan.branch_plan(3).is_none());
    let mut family = ForceFamily::from_rest(plan).unwrap();

    for index in 0..3 {
        let branch = family.branch(index).unwrap();
        assert_eq!(branch.origin(), Origin::InternalFromRest);
        assert_eq!(branch.state().clock().elapsed(), 0);
        assert_eq!(
            branch.plan().settings().domain.layout().dimensions(),
            [4; 3]
        );
        assert_eq!(
            branch.plan().settings().force.samples.dimensions(),
            [settings(1e-5).force_grids[index]; 3]
        );
        for prior in 0..index {
            assert_ne!(
                branch.state().component(0).unwrap().as_ptr(),
                family
                    .branch(prior)
                    .unwrap()
                    .state()
                    .component(0)
                    .unwrap()
                    .as_ptr()
            );
        }
    }

    for expected_clock in clocks {
        let sample = family.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), expected_clock);
        assert_eq!(sample.identity(), plan.identity());
        for index in 0..3 {
            assert_eq!(
                family.branch(index).unwrap().state().clock(),
                expected_clock
            );
        }
        for difference in sample.comparisons() {
            assert!(difference.full.l2.is_finite());
            assert!(difference.full.h1.is_finite());
            assert_eq!(difference.full, difference.common);
            assert_eq!(difference.newly_resolved.l2, 0.0);
            assert_eq!(difference.newly_resolved.h1, 0.0);
        }
        if expected_clock.elapsed() == 512 {
            check_endpoint_oracle(&family, sample.comparisons());
        }
    }
    assert!(family.advance().unwrap().is_none());
}

fn check_endpoint_oracle(
    family: &ForceFamily<'_>,
    actual: [nsbu_solver::diagnostics::comparison::BandComparison; 2],
) {
    for (slot, (left, right)) in [(0, 1), (1, 2)].into_iter().enumerate() {
        let a = family.branch(left).unwrap();
        let b = family.branch(right).unwrap();
        let expected = modal_oracle(
            a.plan().settings().domain,
            b.plan().settings().domain,
            [
                a.state().component(0).unwrap(),
                a.state().component(1).unwrap(),
                a.state().component(2).unwrap(),
            ],
            [
                b.state().component(0).unwrap(),
                b.state().component(1).unwrap(),
                b.state().component(2).unwrap(),
            ],
        );
        assert_oracle(actual[slot], expected);
        assert!(actual[slot].full.l2 > 1e-12);
        assert!(actual[slot].full.h1 > actual[slot].full.l2);

        let state = a.state();
        let aliased = ComparisonPlan::new(state.plan().domain(), state.plan().domain())
            .unwrap()
            .compare(
                [
                    state.component(0).unwrap(),
                    state.component(1).unwrap(),
                    state.component(2).unwrap(),
                ],
                [
                    state.component(0).unwrap(),
                    state.component(1).unwrap(),
                    state.component(2).unwrap(),
                ],
            )
            .unwrap();
        assert_eq!(aliased.full.l2, 0.0);
        assert_ne!(actual[slot].full.l2, aliased.full.l2);
    }
}

#[test]
fn stopped_child_is_terminal_without_rollback_or_fresh_attempts() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let mut family =
        ForceFamily::from_rest(ForceFamilyPlan::new(settings(1e-40), times, CAP).unwrap()).unwrap();
    family.advance().unwrap();
    assert!(matches!(
        family.advance(),
        Err(ForceFamilyError::BranchStopped { branch: 0, .. })
    ));
    let stopped = family.branch(0).unwrap();
    let clock = stopped.state().clock();
    let attempts = stopped.history().controller().attempted();
    let bits = state_bits(stopped);
    assert!(attempts > 0);
    assert_eq!(stopped.work().len(), attempts);
    assert!(matches!(
        family.advance(),
        Err(ForceFamilyError::Terminated)
    ));
    let stopped = family.branch(0).unwrap();
    assert_eq!(stopped.state().clock(), clock);
    assert_eq!(stopped.history().controller().attempted(), attempts);
    assert_eq!(state_bits(stopped), bits);
    assert_eq!(family.branch(1).unwrap().state().clock().elapsed(), 0);
    assert_eq!(family.branch(2).unwrap().state().clock().elapsed(), 0);
}

fn state_bits(run: &nsbu_benchmarks::v2_run::Run) -> Vec<(u64, u64)> {
    (0..3)
        .flat_map(|axis| {
            run.state()
                .component(axis)
                .unwrap()
                .iter()
                .map(|value| (value.re.to_bits(), value.im.to_bits()))
        })
        .collect()
}

#[test]
fn invalid_profiles_caps_and_manifests_are_refused_during_admission() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let base = settings(1e-5);
    assert!(ForceFamilyPlan::new(base, times, 1).is_err());
    let admitted = ForceFamilyPlan::new(base, times, CAP).unwrap();
    assert!(ForceFamilyPlan::new(base, times, admitted.bounds().storage_bytes - 1).is_err());
    for changed in [
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            force_grids: [0, 4, 8],
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            force_grids: [2, 4, 8],
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            force_grids: [4, 4, 8],
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            force_grids: [4, 8, 12],
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            step_ticks: 0,
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            endpoint: 500,
            ..base
        },
    ] {
        assert!(ForceFamilyPlan::new(changed, times, CAP).is_err());
    }
    let off_step = [0, 255, 512].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap());
    assert!(ForceFamilyPlan::new(
        base,
        TestedTimes::new(&off_step, off_step.len()).unwrap(),
        CAP
    )
    .is_err());
}

#[test]
fn identity_binds_every_policy_word_and_complete_exact_manifest() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let base = nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
        force_grids: [8, 16, 32],
        ..settings(1e-5)
    };
    let identity = ForceFamilyPlan::new(base, times, CAP).unwrap().identity();
    assert_eq!(
        identity,
        ForceFamilyPlan::new(base, times, CAP).unwrap().identity()
    );
    let variants = [
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings { grid: 8, ..base },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            force_grids: [4, 8, 16],
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings { workers: 1, ..base },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            step_ticks: 32,
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            method: Method::HochbruckOstermann,
            ..base
        },
        nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
            advective_limit: 0.2,
            ..base
        },
    ];
    for variant in variants {
        assert_ne!(
            identity,
            ForceFamilyPlan::new(variant, times, CAP)
                .unwrap()
                .identity()
        );
    }
    for tolerances in [
        nsbu_solver::integrators::indicator::Tolerances {
            absolute: [2e-5, base.tolerances.absolute[1]],
            ..base.tolerances
        },
        nsbu_solver::integrators::indicator::Tolerances {
            absolute: [base.tolerances.absolute[0], 2e-4],
            ..base.tolerances
        },
        nsbu_solver::integrators::indicator::Tolerances {
            relative: [1e-8, base.tolerances.relative[1]],
            ..base.tolerances
        },
        nsbu_solver::integrators::indicator::Tolerances {
            relative: [base.tolerances.relative[0], 1e-8],
            ..base.tolerances
        },
    ] {
        let variant =
            nsbu_benchmarks::v2_force_experiment::ForceFamilySettings { tolerances, ..base };
        assert_ne!(
            identity,
            ForceFamilyPlan::new(variant, times, CAP)
                .unwrap()
                .identity()
        );
    }
    let shorter = [0, 512].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap());
    assert_ne!(
        identity,
        ForceFamilyPlan::new(
            base,
            TestedTimes::new(&shorter, shorter.len()).unwrap(),
            CAP
        )
        .unwrap()
        .identity()
    );
    let rescaled = [0, 256, 512].map(|t| TickClock::restore(-21, 16384, t, 16384 - t).unwrap());
    assert_ne!(
        identity,
        ForceFamilyPlan::new(
            base,
            TestedTimes::new(&rescaled, rescaled.len()).unwrap(),
            CAP
        )
        .unwrap()
        .identity()
    );
    assert_eq!(
        ForceFamilyPlan::new(base, times, CAP)
            .unwrap()
            .bounds()
            .identity_bytes,
        412
    );

    let canonical = ForceFamilyPlan::new(settings(1e-5), times, CAP).unwrap();
    let hexadecimal: String = canonical
        .identity()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        hexadecimal,
        include_str!("fixtures/v2-force-family-identity-v1.sha256").trim()
    );

    let later_clocks = [0, 512, 1024].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap());
    let later = nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
        endpoint: 1024,
        ..base
    };
    assert_ne!(
        identity,
        ForceFamilyPlan::new(
            later,
            TestedTimes::new(&later_clocks, later_clocks.len()).unwrap(),
            CAP
        )
        .unwrap()
        .identity()
    );
}
