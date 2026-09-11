//! Isolated exact-v2 admission and steady-state allocation audit.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn settings() -> Settings {
    Settings {
        domain: Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method: Method::CoxMatthews,
            limits: RunLimits {
                endpoint: 4096,
                step_ticks: 128,
                maximum_attempts: 32,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn main() {
    let config = settings();
    let admission = Region::new(GLOBAL);
    let plan = Plan::from_rest(config, 1 << 26).unwrap();
    let stats = admission.change();
    assert_eq!((stats.allocations, stats.reallocations), (0, 0));
    assert!(Plan::from_rest(config, plan.resources().total() - 1).is_err());
    let refusal = admission.change();
    assert_eq!((refusal.allocations, refusal.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut run = Run::from_rest(plan).unwrap();
    let construction_stats = construction.change();
    assert!(construction_stats.bytes_allocated <= plan.resources().total());

    for _ in 0..2 {
        let steady = Region::new(GLOBAL);
        assert!(matches!(run.step().unwrap(), Outcome::Committed(_)));
        let stats = steady.change();
        assert_eq!(
            (stats.allocations, stats.deallocations, stats.reallocations),
            (0, 0, 0)
        );
    }
    assert_eq!(run.work().len(), 2);
    println!(
        "exact-v2 allocation admission=0 construction_bytes={} steady_steps=2 declared_bytes={}",
        construction_stats.bytes_allocated,
        plan.resources().total()
    );
}
