//! Numerical samples come from synchronized independent branch states, never imported numbers.
mod smooth_family_support;
use nsbu_benchmarks::{
    smooth_experiment::{FamilyError, FamilyPlan, SmoothFamily},
    smooth_run::Origin,
};
use nsbu_solver::{integrators::method::Method, verification::times::TestedTimes};
use smooth_family_support::{clocks, settings, CAP};

#[test]
fn six_branches_stay_independent_and_report_complete_fine_band_differences() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let plan = FamilyPlan::new(settings(1e-2), times, CAP).unwrap();
    assert!(plan.bounds().storage_bytes < CAP);
    let mut family = SmoothFamily::new(plan).unwrap();
    for a in 0..6 {
        let left = family.branch(a).unwrap();
        assert_eq!(left.state().clock().elapsed(), 0);
        assert_eq!(left.origin(), Origin::InternalFromRest);
        for b in 0..a {
            assert_ne!(
                left.state().component(0).unwrap().as_ptr(),
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
    assert_eq!(
        family
            .branch(5)
            .unwrap()
            .history()
            .controller()
            .configuration()
            .method,
        Method::HochbruckOstermann
    );
    for expected in clocks {
        let sample = family.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), expected);
        for index in 0..6 {
            assert_eq!(family.branch(index).unwrap().state().clock(), expected);
        }
        for difference in sample
            .space()
            .into_iter()
            .chain(sample.time())
            .chain([sample.method()])
        {
            assert!(difference.full.h1.is_finite());
            assert!(difference.full.l2 >= difference.newly_resolved.l2);
            assert!(difference.full.l2 >= difference.common.l2);
        }
        if expected.elapsed() > 0 {
            let gaps = sample.time();
            assert!(gaps[1].full.h1 < gaps[0].full.h1 * 0.25);
            assert!(sample.method().full.h1 < 1e-8);
        }
    }
    let mut reconstruction =
        nsbu_benchmarks::smooth_experiment::reconstruction::ReconstructionWorkspace::new(
            plan, 2, CAP,
        )
        .unwrap();
    let stage = nsbu_solver::domain::TickClock::restore(-16, 512, 124, 388).unwrap();
    assert!(reconstruction.measure(&family, stage).is_err());
    let probe = nsbu_solver::domain::TickClock::restore(-16, 512, 127, 385).unwrap();
    let comparison = reconstruction.measure(&family, probe).unwrap();
    assert!(comparison
        .geometry()
        .levels()
        .into_iter()
        .all(|level| level.time() == probe));
    assert!(comparison.values()[1].full.h1 < comparison.values()[0].full.h1);
    assert!(comparison
        .derivatives()
        .into_iter()
        .all(|difference| difference.full.h1 < 1e-4));
    assert_eq!(reconstruction.remaining(), 0);
    assert!(reconstruction.measure(&family, probe).is_err());
    for index in 0..6 {
        assert_eq!(family.branch(index).unwrap().state().clock().elapsed(), 128);
    }
    assert!(family.advance().unwrap().is_none());
    let spent = (0..6)
        .map(|i| {
            family
                .branch(i)
                .unwrap()
                .work()
                .iter()
                .map(|w| w.calls())
                .sum::<usize>()
        })
        .sum::<usize>();
    assert_eq!(spent, family.plan().bounds().integration_calls);
}
#[test]
fn a_rejected_branch_terminates_the_family_without_advancing_other_states() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let mut family =
        SmoothFamily::new(FamilyPlan::new(settings(1e-40), times, CAP).unwrap()).unwrap();
    family.advance().unwrap();
    assert!(matches!(
        family.advance(),
        Err(FamilyError::BranchStopped { branch: 0, .. })
    ));
    assert!(matches!(family.advance(), Err(FamilyError::Terminated)));
    for index in 0..6 {
        assert_eq!(family.branch(index).unwrap().state().clock().elapsed(), 0);
    }
    assert_eq!(
        family.branch(0).unwrap().history().controller().attempted(),
        1
    );
    assert_eq!(
        family.branch(1).unwrap().history().controller().attempted(),
        0
    );
}
#[test]
fn aggregate_cap_and_incompatible_family_settings_are_refused_before_allocation() {
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    assert!(FamilyPlan::new(settings(1e-2), times, 1).is_err());
    let original = settings(1e-2);
    let approved = FamilyPlan::new(original, times, CAP).unwrap();
    assert!(FamilyPlan::new(original, times, approved.bounds().storage_bytes - 1).is_err());
    for changed in [
        nsbu_benchmarks::smooth_experiment::FamilySettings {
            grids: [4, 4, 12],
            ..original
        },
        nsbu_benchmarks::smooth_experiment::FamilySettings {
            steps: [16, 8, 0],
            ..original
        },
        nsbu_benchmarks::smooth_experiment::FamilySettings {
            steps: [16, 7, 4],
            ..original
        },
        nsbu_benchmarks::smooth_experiment::FamilySettings {
            endpoint: 64,
            ..original
        },
    ] {
        assert!(FamilyPlan::new(changed, times, CAP).is_err());
    }
}

#[test]
fn reconstruction_admission_missing_history_and_domain_mismatch_keep_spent_attempts() {
    use nsbu_benchmarks::smooth_experiment::reconstruction::ReconstructionWorkspace;
    use nsbu_solver::domain::{Domain, TickClock};
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let plan = FamilyPlan::new(settings(1e-2), times, CAP).unwrap();
    let domain = Domain::new([12; 3], [1.0; 3], 1.0).unwrap();
    assert!(ReconstructionWorkspace::reservation(domain, 0).is_err());
    assert!(ReconstructionWorkspace::reservation(domain, usize::MAX).is_err());
    assert!(ReconstructionWorkspace::new(plan, 1, 1).is_err());
    let mut workspace = ReconstructionWorkspace::new(plan, 2, CAP).unwrap();
    let family = SmoothFamily::new(plan).unwrap();
    let probe = TickClock::restore(-16, 512, 127, 385).unwrap();
    assert!(workspace.measure(&family, probe).is_err());
    assert_eq!(workspace.remaining(), 1);
    let mut changed = settings(1e-2);
    changed.grids = [4, 8, 16];
    let other = SmoothFamily::new(FamilyPlan::new(changed, times, CAP).unwrap()).unwrap();
    assert!(workspace.measure(&other, probe).is_err());
    assert_eq!(workspace.remaining(), 0);
    assert_eq!(family.branch(2).unwrap().state().clock().elapsed(), 0);
}
