//! Joint-construction bound and zero-steady-allocation recorded-step audit.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    smooth_observer::v2::V2Observer,
    v2_experiment::shared_force::{
        SharedForceAdapterSet, SharedForceAdapterSetPlan, SharedForceAttempt, SharedForceClock,
        SharedForceStream, SharedForceTablePlan,
    },
    v2_run::{Plan, Settings},
};
use nsbu_solver::{
    domain::{Domain, Epoch, Layout, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace, indicator::Tolerances, method::Method, rhs::SpectralRhs,
        trajectory::RunLimits, transaction::CandidateState,
    },
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

const CAP: usize = 64 * 1024 * 1024;
const TARGET: u128 = 8192;
const STEP: u128 = 16;

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, TARGET, elapsed, TARGET - elapsed).unwrap()
}

fn force() -> ForceSettings {
    ForceSettings {
        samples: Layout::new([16; 3]).unwrap(),
        workers: 0,
    }
}

fn settings(domain: Domain) -> Settings {
    Settings {
        domain,
        force: force(),
        initial_clock: clock(0),
        configuration: Configuration {
            method: Method::CoxMatthews,
            limits: RunLimits {
                endpoint: STEP,
                step_ticks: STEP,
                maximum_attempts: 1,
            },
            tolerances: Tolerances {
                absolute: [1.0; 2],
                relative: [1.0; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn main() {
    let domains = [4, 8, 12].map(|n| Domain::new([n; 3], [1.0; 3], 1.0).unwrap());
    let attempts = [SharedForceAttempt::new(clock(0), STEP, Method::CoxMatthews).unwrap(); 3];
    let streams = [
        SharedForceStream::new(0, &attempts[0..1]).unwrap(),
        SharedForceStream::new(1, &attempts[1..2]).unwrap(),
        SharedForceStream::new(2, &attempts[2..3]).unwrap(),
    ];
    let multiplicities = [2, 2, 4, 2, 2];
    let manifest: [SharedForceClock; 5] = std::array::from_fn(|index| {
        SharedForceClock::new(clock(index as u128 * STEP / 4), [multiplicities[index]; 3]).unwrap()
    });
    let table = SharedForceTablePlan::new(force(), domains, &manifest, 36, CAP).unwrap();
    let adapters = SharedForceAdapterSetPlan::new(table, &streams, CAP).unwrap();
    let run = Plan::from_rest(settings(domains[0]), CAP).unwrap();
    let declared = adapters
        .bounds()
        .joint_peak_bytes
        .checked_add(run.resources().total())
        .unwrap();

    let construction = Region::new(&INSTRUMENTED_SYSTEM);
    let owner = SharedForceAdapterSet::new(adapters).unwrap();
    let adapter = owner.adapter(0).unwrap();
    let _unused_middle = owner.adapter(1).unwrap();
    let _unused_fine = owner.adapter(2).unwrap();
    let resources = run.resources();
    let configuration = run.settings().configuration;
    let mut state =
        SpectralState::from_rest(resources, run.settings().initial_clock, Epoch(0)).unwrap();
    let mut candidate =
        CandidateState::new(resources, run.settings().initial_clock, Epoch(0)).unwrap();
    let mut attempt = AttemptWorkspace::new_with_method(resources, configuration.method).unwrap();
    let mut rhs = SpectralRhs::new(
        domains[0],
        adapter,
        run.settings().advective_limit,
        resources.classes()[5],
    )
    .unwrap();
    let mut observer =
        V2Observer::new(domains[0], force(), 1, run.observer_limits().storage_bytes).unwrap();
    let mut history = RunHistory::new(
        run.settings().initial_clock,
        configuration,
        RunHistory::reservation(configuration).unwrap(),
    )
    .unwrap();
    let allocated = construction.change();
    assert!(allocated.bytes_allocated <= declared);

    let steady = Region::new(&INSTRUMENTED_SYSTEM);
    assert!(matches!(
        recorded_step(
            &mut state,
            &mut candidate,
            &mut attempt,
            &mut rhs,
            &mut observer,
            &mut history,
        )
        .unwrap(),
        Outcome::Committed(_)
    ));
    let change = steady.change();
    assert_eq!(
        (
            change.allocations,
            change.reallocations,
            change.deallocations
        ),
        (0, 0, 0)
    );
    assert_eq!(owner.table_work().unwrap().copy_attempts, 12);
    eprintln!(
        "shared adapter construction_bytes={} declared_joint={} steady_allocations={}",
        allocated.bytes_allocated, declared, change.allocations
    );
}
