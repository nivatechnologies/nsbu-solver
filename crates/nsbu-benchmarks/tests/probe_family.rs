//! Earlier off-stage samples require streaming accepted history, never a reset or late extrapolation.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    probes::{ProbeFamily, ProbePlan},
    FamilyError, FamilyPlan, SmoothFamily,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
fn clocks() -> [TickClock; 8] {
    [0, 7, 31, 63, 64, 95, 127, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap())
}

#[test]
fn streaming_probes_preserve_early_history_and_match_independent_final_state_words() {
    let accepted = smooth_family_support::clocks();
    let fine = clocks();
    let cap = smooth_family_support::CAP;
    let family_plan = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let plan = ProbePlan::new(family_plan, TestedTimes::new(&fine, 8).unwrap(), 8, cap).unwrap();
    assert_eq!(plan.bounds().admission_geometry_checks, 48);
    assert_eq!(plan.tested_times().as_slice(), &fine);
    assert_eq!(plan.family_plan().settings().endpoint, 128);
    let mut run = ProbeFamily::new(plan).unwrap();
    assert_eq!(run.plan().bounds(), plan.bounds());
    assert!(run.fields(0).is_none());
    let mut early = None;
    for clock in fine {
        assert_eq!(run.next_time(), Some(clock));
        let sample = run.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), clock);
        for (index, origin) in sample.origins().iter().enumerate() {
            let view = run.fields(index).unwrap();
            let state = run.branch(index).unwrap().state();
            assert_eq!(
                origin.accepted_nodes,
                run.branch(index)
                    .unwrap()
                    .observer()
                    .last_accepted_clocks()
                    .unwrap()
            );
            assert_eq!(origin.state_clock, state.clock());
            assert_eq!(view.origin, *origin);
            assert_eq!(view.clock, clock);
            assert_eq!(view.domain, state.plan().domain());
            assert!(origin.accepted_nodes[0].elapsed() <= clock.elapsed());
            assert!(origin.accepted_nodes[2].elapsed() >= clock.elapsed());
            if clock.elapsed() == 0 {
                assert!(view
                    .value
                    .iter()
                    .all(|v| v.iter().all(|c| c.re == 0.0 && c.im == 0.0)));
            }
            if clock.elapsed() == 128 {
                for (axis, values) in view.value.into_iter().enumerate() {
                    assert_eq!(values, state.component(axis).unwrap());
                }
            }
            assert!(view
                .derivative
                .iter()
                .all(|v| v.iter().all(|c| c.re.is_finite() && c.im.is_finite())));
        }
        assert!(sample.values().iter().all(|v| v.full.h1.is_finite()));
        assert!(sample.derivatives().iter().all(|v| v.full.h1.is_finite()));
        if clock.elapsed() == 7 {
            early = Some(sample);
            assert!(sample.values()[3].full.h1 > 0.0);
        }
    }
    assert_eq!(early.unwrap().clock().elapsed(), 7);
    assert!(!run.is_terminated());
    assert_eq!(run.charged_work(), plan.bounds().work);
    assert_eq!(run.next_time(), None);
    assert!(run.advance().unwrap().is_none());
    assert!(run.fields(6).is_none());
    assert!(run.branch(6).is_none());
    // A late query of the final bounded history cannot recover the already emitted early probe.
    let mut value = vec![
        nsbu_solver::Complex64::new(0.0, 0.0);
        run.fields(2).unwrap().domain.layout().half_len()
    ];
    let mut derivative = value.clone();
    assert!(run
        .branch(2)
        .unwrap()
        .observer()
        .reconstruct(fine[1], 0, &mut value, &mut derivative)
        .is_err());
    let mut direct = SmoothFamily::new(family_plan).unwrap();
    while direct.advance().unwrap().is_some() {}
    compare_words(&run, &direct);
}
fn compare_words(run: &ProbeFamily<'_>, direct: &SmoothFamily<'_>) {
    for index in 0..6 {
        let left = run.branch(index).unwrap();
        let right = direct.branch(index).unwrap();
        assert_eq!(left.state().clock(), right.state().clock());
        for axis in 0..3 {
            for (a, b) in left
                .state()
                .component(axis)
                .unwrap()
                .iter()
                .zip(right.state().component(axis).unwrap())
            {
                assert_eq!(
                    [a.re.to_bits(), a.im.to_bits()],
                    [b.re.to_bits(), b.im.to_bits()]
                );
            }
        }
    }
}

#[test]
fn lookahead_window_memory_and_attempt_admission_are_explicit() {
    let accepted = smooth_family_support::clocks();
    let fine = clocks();
    let cap = smooth_family_support::CAP;
    let family = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let times = TestedTimes::new(&fine, 8).unwrap();
    let plan = ProbePlan::new(family, times, 8, cap).unwrap();
    assert!(ProbePlan::new(family, times, 8, plan.bounds().joint_storage_bytes - 1).is_err());
    assert!(ProbePlan::new(family, times, 7, cap).is_err());
    assert!(ProbePlan::new(family, times, usize::MAX, cap).is_err());
    let changed = fine.map(|t| TickClock::restore(-15, 512, t.elapsed(), t.remaining()).unwrap());
    assert!(ProbePlan::new(family, TestedTimes::new(&changed, 8).unwrap(), 8, cap).is_err());
    let short = [accepted[0], accepted[1]];
    let short_times = TestedTimes::new(&short, 2).unwrap();
    assert!(ProbePlan::new(family, short_times, 8, cap).is_err());
    let mut settings = smooth_family_support::settings(1e-2);
    settings.endpoint = 64;
    let family = FamilyPlan::new(settings, short_times, cap).unwrap();
    assert!(ProbePlan::new(family, short_times, 2, cap).is_err());
}

#[test]
fn a_rejected_branch_cannot_publish_partial_probe_fields_or_retry() {
    let accepted = smooth_family_support::clocks();
    let fine = clocks();
    let cap = smooth_family_support::CAP;
    let family = FamilyPlan::new(
        smooth_family_support::settings(1e-30),
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let plan = ProbePlan::new(family, TestedTimes::new(&fine, 8).unwrap(), 8, cap).unwrap();
    let mut run = ProbeFamily::new(plan).unwrap();
    assert!(run.advance().is_err());
    assert!(run.is_terminated());
    assert!(run.fields(0).is_none());
    assert_eq!(run.next_time(), Some(fine[0]));
    assert_eq!(run.charged_work().attempts, 1);
    let charged = run.charged_work();
    assert!(matches!(run.advance(), Err(FamilyError::Terminated)));
    assert_eq!(run.charged_work(), charged);
}
