//! Independent six-branch exact-v2 family tests.
mod v2_family_oracle;
mod v2_family_support;
use v2_family_oracle::{assert_oracle, modal_oracle};

use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyError, FamilyPlan, V2Family},
    v2_run::Origin,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::method::Method,
    verification::times::TestedTimes,
};
use v2_family_support::{clocks, settings, CAP};

#[test]
fn six_branches_are_private_and_compare_complete_actual_bands() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let plan = FamilyPlan::new(settings(1e-5), times, CAP).unwrap();
    assert_eq!(plan.times().as_slice(), &clocks);
    assert!(plan.branch_plan(0).is_some());
    assert!(plan.branch_plan(6).is_none());
    assert!(plan.bounds().storage_bytes < CAP);
    assert!(plan.bounds().integration_calls > 0);
    assert!(plan.bounds().provider_work_units > 0);
    assert!(plan.bounds().scalar_transforms > 0);
    assert!(plan.bounds().comparison_work_units > 0);
    assert_eq!(plan.settings().force.samples, Layout::new([12; 3]).unwrap());
    let mut family = V2Family::new(plan).unwrap();

    for a in 0..6 {
        let run = family.branch(a).unwrap();
        assert_eq!(run.state().clock().elapsed(), 0);
        assert_eq!(run.origin(), Origin::InternalFromRest);
        for b in 0..a {
            assert_ne!(
                run.state().component(0).unwrap().as_ptr(),
                family
                    .branch(b)
                    .unwrap()
                    .state()
                    .component(0)
                    .unwrap()
                    .as_ptr()
            );
        }
    }
    assert!(family.branch(6).is_none());
    for index in 0..6 {
        assert_eq!(
            family
                .branch(index)
                .unwrap()
                .plan()
                .settings()
                .force
                .samples,
            plan.settings().force.samples
        );
    }
    assert_eq!(
        family.branch(0).unwrap().plan().settings().force.samples,
        plan.settings().force.samples
    );
    assert_eq!(
        family
            .branch(3)
            .unwrap()
            .plan()
            .settings()
            .configuration
            .limits
            .step_ticks,
        64
    );
    assert_eq!(
        family
            .branch(4)
            .unwrap()
            .plan()
            .settings()
            .configuration
            .limits
            .step_ticks,
        32
    );
    assert_eq!(
        family
            .branch(2)
            .unwrap()
            .plan()
            .settings()
            .configuration
            .limits
            .step_ticks,
        16
    );
    assert_eq!(
        family
            .branch(5)
            .unwrap()
            .plan()
            .settings()
            .configuration
            .method,
        Method::HochbruckOstermann
    );
    for expected in clocks {
        let sample = family.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), expected);
        assert_eq!(sample.identity(), plan.identity());
        for index in 0..6 {
            assert_eq!(family.branch(index).unwrap().state().clock(), expected);
        }
        for comparison in sample
            .space()
            .into_iter()
            .chain(sample.time())
            .chain([sample.method()])
        {
            assert!(comparison.full.l2.is_finite());
            assert!(comparison.full.h1.is_finite());
            assert!(comparison.full.l2 >= comparison.common.l2);
            assert!(comparison.full.l2 >= comparison.newly_resolved.l2);
            assert!(comparison.mean_error.iter().all(|value| value.is_finite()));
        }
        if expected.elapsed() == 128 {
            let pairs = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];
            let actual = sample
                .space()
                .into_iter()
                .chain(sample.time())
                .chain([sample.method()])
                .collect::<Vec<_>>();
            for (slot, &(left, right)) in pairs.iter().enumerate() {
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
            }
        }
    }
    assert!(family.advance().unwrap().is_none());
}

#[test]
fn stopped_branch_is_terminal_and_retains_committed_work() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let mut family = V2Family::new(FamilyPlan::new(settings(1e-40), times, CAP).unwrap()).unwrap();
    family.advance().unwrap();
    let result = family.advance();
    assert!(matches!(
        result,
        Err(FamilyError::BranchStopped { branch: 0, .. })
    ));
    let stopped = family.branch(0).unwrap();
    let clock = stopped.state().clock();
    let attempts = stopped.history().controller().attempted();
    let words: Vec<_> = (0..3)
        .flat_map(|axis| {
            stopped
                .state()
                .component(axis)
                .unwrap()
                .iter()
                .map(|z| (z.re.to_bits(), z.im.to_bits()))
        })
        .collect();
    assert!(matches!(family.advance(), Err(FamilyError::Terminated)));
    let stopped = family.branch(0).unwrap();
    assert_eq!(stopped.state().clock(), clock);
    assert_eq!(stopped.history().controller().attempted(), attempts);
    let after: Vec<_> = (0..3)
        .flat_map(|axis| {
            stopped
                .state()
                .component(axis)
                .unwrap()
                .iter()
                .map(|z| (z.re.to_bits(), z.im.to_bits()))
        })
        .collect();
    assert_eq!(words, after);
    for index in 1..6 {
        assert_eq!(family.branch(index).unwrap().state().clock().elapsed(), 0);
    }
    let stopped = family.branch(0).unwrap();
    assert!(stopped.history().controller().attempted() >= 1);
    assert_eq!(
        stopped.work().len(),
        stopped.history().controller().attempted()
    );
    assert_eq!(
        family.branch(1).unwrap().history().controller().attempted(),
        0
    );
    assert!(!family.branch(0).unwrap().work().is_empty());
}

#[test]
fn invalid_family_profiles_and_caps_are_refused_before_owner_allocation() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let original = settings(1e-5);
    assert!(FamilyPlan::new(original, times, 1).is_err());
    let approved = FamilyPlan::new(original, times, CAP).unwrap();
    assert!(FamilyPlan::new(original, times, approved.bounds().storage_bytes - 1).is_err());
    for changed in [
        nsbu_benchmarks::v2_experiment::FamilySettings {
            grids: [4, 4, 8],
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            grids: [4, 8, 6],
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            steps: [128, 65, 32],
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            steps: [128, 64, 0],
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            endpoint: 64,
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            force: ForceSettings {
                samples: Layout::new([4; 3]).unwrap(),
                ..original.force
            },
            ..original
        },
    ] {
        assert!(FamilyPlan::new(changed, times, CAP).is_err());
    }
    let wrong_times =
        [0, 32, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    assert!(FamilyPlan::new(original, TestedTimes::new(&wrong_times, 3).unwrap(), CAP).is_err());
}

#[test]
fn identity_changes_with_every_meaningful_admitted_input() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let mut original = settings(1e-5);
    original.force.samples = Layout::new([16; 3]).unwrap();
    let base = FamilyPlan::new(original, times, CAP).unwrap().identity();
    assert_eq!(
        base,
        FamilyPlan::new(original, times, CAP).unwrap().identity()
    );
    let variants = [
        nsbu_benchmarks::v2_experiment::FamilySettings {
            grids: [4, 8, 16],
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            steps: [64, 32, 8],
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            advective_limit: 0.2,
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            force: ForceSettings {
                workers: 1,
                ..original.force
            },
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            tolerances: nsbu_solver::integrators::indicator::Tolerances {
                absolute: [2e-5; 2],
                relative: [1e-8; 2],
            },
            ..original
        },
        nsbu_benchmarks::v2_experiment::FamilySettings {
            force: ForceSettings {
                samples: Layout::new([24; 3]).unwrap(),
                ..original.force
            },
            ..original
        },
    ];
    for variant in variants {
        let other = FamilyPlan::new(variant, times, CAP).unwrap();
        assert_ne!(base, other.identity());
    }
    let endpoint_variant = nsbu_benchmarks::v2_experiment::FamilySettings {
        endpoint: 512,
        ..original
    };
    let altered = [0, 256, 512]
        .map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let other = FamilyPlan::new(
        endpoint_variant,
        TestedTimes::new(&altered, 3).unwrap(),
        CAP,
    )
    .unwrap();
    assert_ne!(base, other.identity());
    let shorter =
        [0, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    assert_ne!(
        base,
        FamilyPlan::new(original, TestedTimes::new(&shorter, 2).unwrap(), CAP)
            .unwrap()
            .identity()
    );
}

#[test]
fn canonical_identity_matches_independent_little_endian_fixture_and_exact_clock() {
    let times = clocks();
    let settings = settings(1e-5);
    let plan = FamilyPlan::new(settings, TestedTimes::new(&times, 3).unwrap(), CAP).unwrap();
    let actual: String = plan.identity().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        actual,
        include_str!("fixtures/v2-family-identity-v1.sha256").trim()
    );
    assert_eq!(plan.bounds().identity_bytes, 484);
    let rescaled = [0, 64, 128].map(|t| TickClock::restore(-21, 16384, t, 16384 - t).unwrap());
    let changed = FamilyPlan::new(settings, TestedTimes::new(&rescaled, 3).unwrap(), CAP).unwrap();
    assert_ne!(plan.identity(), changed.identity());
    let wrong_target = [0, 64, 128].map(|t| TickClock::restore(-20, 16384, t, 16384 - t).unwrap());
    assert!(FamilyPlan::new(settings, TestedTimes::new(&wrong_target, 3).unwrap(), CAP).is_err());
}

#[test]
fn extreme_exact_window_refuses_unrepresentable_attempt_count() {
    let target = 1u128 << 113;
    let endpoint = 1u128 << 112;
    let times = [0, endpoint].map(|t| TickClock::restore(-120, target, t, target - t).unwrap());
    let config = nsbu_benchmarks::v2_experiment::FamilySettings {
        endpoint,
        ..settings(1e-5)
    };
    assert!(matches!(
        FamilyPlan::new(config, TestedTimes::new(&times, 2).unwrap(), usize::MAX),
        Err(FamilyError::Numerical(
            nsbu_solver::SolverError::SizeOverflow
        ))
    ));
}
