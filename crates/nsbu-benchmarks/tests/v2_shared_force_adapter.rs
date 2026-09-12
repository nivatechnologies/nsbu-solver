//! Actual recorded trajectories driven by one externally owned original-force table.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    smooth_observer::v2::V2Observer,
    v2_experiment::shared_force::{
        SharedForceAdapter, SharedForceAdapterSetPlan, SharedForceAttempt, SharedForceClock,
        SharedForceStream, SharedForceTable, SharedForceTablePlan,
    },
    v2_run::{Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Epoch, Layout, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace, forcing::PrescribedForce, indicator::Tolerances, method::Method,
        rhs::SpectralRhs, trajectory::RunLimits, transaction::CandidateState,
    },
};
use std::cell::RefCell;

const CAP: usize = 64 * 1024 * 1024;
const TARGET: u128 = 8192;
const STEP: u128 = 16;
const ENDPOINT: u128 = 32;

type Adapter<'a> = SharedForceAdapter<'a, 'a, 'a>;

struct Recorded<'a> {
    state: SpectralState,
    candidate: CandidateState,
    attempt: AttemptWorkspace,
    rhs: SpectralRhs<Adapter<'a>>,
    observer: V2Observer,
    history: RunHistory,
}
impl<'a> Recorded<'a> {
    fn new(plan: Plan, force: Adapter<'a>) -> Self {
        let settings = plan.settings();
        let resources = plan.resources();
        Self {
            state: SpectralState::from_rest(resources, settings.initial_clock, Epoch(0)).unwrap(),
            candidate: CandidateState::new(resources, settings.initial_clock, Epoch(0)).unwrap(),
            attempt: AttemptWorkspace::new_with_method(resources, settings.configuration.method)
                .unwrap(),
            rhs: SpectralRhs::new(
                settings.domain,
                force,
                settings.advective_limit,
                resources.classes()[5],
            )
            .unwrap(),
            observer: V2Observer::new(
                settings.domain,
                settings.force,
                settings.configuration.limits.maximum_attempts,
                plan.observer_limits().storage_bytes,
            )
            .unwrap(),
            history: RunHistory::new(
                settings.initial_clock,
                settings.configuration,
                RunHistory::reservation(settings.configuration).unwrap(),
            )
            .unwrap(),
        }
    }

    fn step(&mut self) -> Outcome {
        recorded_step(
            &mut self.state,
            &mut self.candidate,
            &mut self.attempt,
            &mut self.rhs,
            &mut self.observer,
            &mut self.history,
        )
        .unwrap()
    }
}

fn domain(n: usize) -> Domain {
    Domain::new([n; 3], [1.0; 3], 1.0).unwrap()
}

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, TARGET, elapsed, TARGET - elapsed).unwrap()
}

fn force() -> ForceSettings {
    ForceSettings {
        samples: Layout::new([16; 3]).unwrap(),
        workers: 0,
    }
}

fn settings(domain: Domain, method: Method) -> Settings {
    Settings {
        domain,
        force: force(),
        initial_clock: clock(0),
        configuration: Configuration {
            method,
            limits: RunLimits {
                endpoint: ENDPOINT,
                step_ticks: STEP,
                maximum_attempts: 2,
            },
            tolerances: Tolerances {
                absolute: [1.0; 2],
                relative: [1.0; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn attempts(method: Method) -> [SharedForceAttempt; 2] {
    [
        SharedForceAttempt::new(clock(0), STEP, method).unwrap(),
        SharedForceAttempt::new(clock(STEP), STEP, method).unwrap(),
    ]
}

fn requested(attempt: SharedForceAttempt) -> Vec<TickClock> {
    let stages = attempt.start().stages(attempt.ticks()).unwrap();
    let indices: &[usize] = match attempt.method() {
        Method::CoxMatthews => &[0, 2, 2, 4, 0, 1, 1, 2, 2, 3, 3, 4],
        Method::HochbruckOstermann => &[0, 2, 2, 4, 2, 0, 1, 1, 2, 1, 2, 3, 3, 4, 3],
    };
    indices.iter().map(|index| stages[*index]).collect()
}

fn manifest(
    stream_attempts: &[[SharedForceAttempt; 2]; 6],
    domains: [usize; 6],
) -> Vec<SharedForceClock> {
    (0..=ENDPOINT)
        .step_by((STEP / 4) as usize)
        .map(|elapsed| {
            let mut copies = [0usize; 3];
            for (stream, domain) in stream_attempts.iter().zip(domains) {
                for attempt in stream {
                    copies[domain] += requested(*attempt)
                        .iter()
                        .filter(|value| value.elapsed() == elapsed)
                        .count();
                }
            }
            SharedForceClock::new(clock(elapsed), copies).unwrap()
        })
        .collect()
}

fn words(state: &SpectralState) -> [Vec<(u64, u64)>; 3] {
    std::array::from_fn(|axis| {
        state
            .component(axis)
            .unwrap()
            .iter()
            .map(|value| (value.re.to_bits(), value.im.to_bits()))
            .collect()
    })
}

fn equal_history(left: &RunHistory, right: &RunHistory) {
    assert_eq!(left.controller().clock(), right.controller().clock());
    assert_eq!(
        left.controller().committed(),
        right.controller().committed()
    );
    assert_eq!(left.records().len(), right.records().len());
    for (a, b) in left.records().iter().zip(right.records()) {
        assert_eq!(a.start, b.start);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.sample, b.sample);
    }
}

#[test]
fn six_independent_recorded_trajectories_match_direct_force_at_every_commit() {
    let retained = [domain(4), domain(8), domain(12)];
    let domain_indices = [0, 1, 2, 0, 1, 2];
    let methods = [
        Method::CoxMatthews,
        Method::CoxMatthews,
        Method::CoxMatthews,
        Method::HochbruckOstermann,
        Method::HochbruckOstermann,
        Method::HochbruckOstermann,
    ];
    let stream_attempts = methods.map(attempts);
    let table_manifest = manifest(&stream_attempts, domain_indices);
    let streams: [SharedForceStream<'_>; 6] = std::array::from_fn(|index| {
        SharedForceStream::new(domain_indices[index], &stream_attempts[index]).unwrap()
    });
    let total_calls = methods.iter().map(|method| method.rhs_calls() * 2).sum();
    let table_plan =
        SharedForceTablePlan::new(force(), retained, &table_manifest, total_calls, CAP).unwrap();
    let set = SharedForceAdapterSetPlan::new(table_plan, &streams, CAP).unwrap();
    assert_eq!(set.bounds().work.calls, total_calls);
    assert_eq!(set.bounds().work.attempts, 12);
    assert_eq!(
        set.bounds().storage_bytes - table_plan.bounds().storage_bytes,
        set.streams().len() * set.bounds().handle_storage_bytes
    );

    let table = RefCell::new(SharedForceTable::new(table_plan).unwrap());
    let direct_plans: Vec<_> = domain_indices
        .iter()
        .zip(methods)
        .map(|(index, method)| Plan::from_rest(settings(retained[*index], method), CAP).unwrap())
        .collect();
    let mut direct: Vec<_> = direct_plans
        .iter()
        .copied()
        .map(|plan| Run::from_rest(plan).unwrap())
        .collect();
    let mut shared: Vec<_> = direct_plans
        .iter()
        .copied()
        .enumerate()
        .map(|(index, plan)| Recorded::new(plan, set.adapter(index, &table).unwrap()))
        .collect();

    for step in 0..2 {
        for index in 0..6 {
            let direct_outcome = direct[index].step().unwrap();
            let shared_outcome = shared[index].step();
            assert!(matches!(direct_outcome, Outcome::Committed(_)));
            assert_eq!(shared_outcome, direct_outcome);
            assert_eq!(words(&shared[index].state), words(direct[index].state()));
            equal_history(&shared[index].history, direct[index].history());
            assert_eq!(
                shared[index].observer.consumption(),
                direct[index].observer_work()
            );
            assert_eq!(
                shared[index].rhs.consumption()[0],
                methods[index].rhs_calls()
            );
            assert_eq!(
                shared[index].rhs.consumption()[2],
                10 * methods[index].rhs_calls()
            );
            assert_eq!(
                direct[index].work()[step].integration()[2],
                13 * methods[index].rhs_calls()
            );
        }
    }
    let owner = table.borrow();
    assert_eq!(owner.charged_work().copy_attempts, total_calls);
    assert_eq!(owner.charged_work(), table_plan.bounds().work);
    assert!(!owner.is_terminated());
    let charged = shared.iter().fold(
        nsbu_benchmarks::v2_experiment::shared_force::SharedForceAdapterWork::default(),
        |mut total, run| {
            let work = run.rhs.provider().charged_work();
            total.attempts += work.attempts;
            total.calls += work.calls;
            total.clock_comparisons += work.clock_comparisons;
            total.binding_checks += work.binding_checks;
            total.table_work_units += work.table_work_units;
            total
        },
    );
    assert_eq!(charged, set.bounds().work);
}

fn one_attempt_plan() -> ([SharedForceAttempt; 3], [SharedForceClock; 5]) {
    let attempts = [SharedForceAttempt::new(clock(0), STEP, Method::CoxMatthews).unwrap(); 3];
    let multiplicities = [2, 2, 4, 2, 2];
    let manifest = std::array::from_fn(|index| {
        SharedForceClock::new(clock(index as u128 * STEP / 4), [multiplicities[index]; 3]).unwrap()
    });
    (attempts, manifest)
}

#[test]
fn joint_admission_rejects_schedule_or_cap_mismatch_before_table_allocation() {
    let retained = [domain(4), domain(8), domain(12)];
    let (attempts, manifest) = one_attempt_plan();
    let streams = [
        SharedForceStream::new(0, &attempts[0..1]).unwrap(),
        SharedForceStream::new(1, &attempts[1..2]).unwrap(),
        SharedForceStream::new(2, &attempts[2..3]).unwrap(),
    ];
    let table = SharedForceTablePlan::new(force(), retained, &manifest, 36, CAP).unwrap();
    let admitted = SharedForceAdapterSetPlan::new(table, &streams, CAP).unwrap();
    assert!(matches!(
        SharedForceAdapterSetPlan::new(table, &streams, admitted.bounds().joint_peak_bytes - 1),
        Err(nsbu_solver::SolverError::ResourceLimit)
    ));
    assert!(matches!(
        SharedForceAdapterSetPlan::new(table, &[], CAP),
        Err(nsbu_solver::SolverError::InvalidPayload)
    ));
    let ho = [SharedForceAttempt::new(clock(0), STEP, Method::HochbruckOstermann).unwrap()];
    let wrong = [
        SharedForceStream::new(0, &ho).unwrap(),
        streams[1],
        streams[2],
    ];
    assert!(matches!(
        SharedForceAdapterSetPlan::new(table, &wrong, CAP),
        Err(nsbu_solver::SolverError::InvalidPayload)
    ));
}

#[test]
fn borrow_conflict_and_incomplete_attempt_fail_closed_without_touching_table_or_output() {
    let retained = [domain(4), domain(8), domain(12)];
    let (attempts, manifest) = one_attempt_plan();
    let streams = [
        SharedForceStream::new(0, &attempts[0..1]).unwrap(),
        SharedForceStream::new(1, &attempts[1..2]).unwrap(),
        SharedForceStream::new(2, &attempts[2..3]).unwrap(),
    ];
    let table_plan = SharedForceTablePlan::new(force(), retained, &manifest, 36, CAP).unwrap();
    let set = SharedForceAdapterSetPlan::new(table_plan, &streams, CAP).unwrap();
    let owner = RefCell::new(SharedForceTable::new(table_plan).unwrap());
    let mut conflicting = set.adapter(0, &owner).unwrap();
    let limits = conflicting.limits().unwrap();
    conflicting.begin_attempt(clock(0), STEP, limits).unwrap();
    let mut output = std::array::from_fn(|_| {
        vec![nsbu_solver::Complex64::new(3.0, -7.0); retained[0].layout().half_len()]
    });
    let before = output.clone();
    let guard = owner.borrow_mut();
    assert_eq!(
        conflicting
            .evaluate(clock(0), limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap_err(),
        nsbu_solver::SolverError::ProviderBudgetExceeded
    );
    drop(guard);
    assert_eq!(output, before);
    assert!(conflicting.is_terminated());
    assert_eq!(conflicting.charged_work().attempts, 1);
    assert_eq!(conflicting.charged_work().calls, 1);
    assert_eq!(owner.borrow().charged_work().copy_attempts, 0);
    assert!(owner.borrow().current().is_none());
    assert!(!owner.borrow().is_terminated());

    let mut incomplete = set.adapter(1, &owner).unwrap();
    let limits = incomplete.limits().unwrap();
    incomplete.begin_attempt(clock(0), STEP, limits).unwrap();
    assert_eq!(
        incomplete.begin_attempt(clock(0), STEP, limits),
        Err(nsbu_solver::SolverError::ProviderBudgetExceeded)
    );
    assert_eq!(incomplete.charged_work().attempts, 1);
    assert!(incomplete.is_terminated());
    assert_eq!(owner.borrow().charged_work().copy_attempts, 0);
}
