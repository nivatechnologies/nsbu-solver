//! Dedicated boxed-cache runtime allocation and cap audit.
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

fn main() {
    let settings = Settings {
        domain: Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method: Method::CoxMatthews,
            limits: RunLimits {
                endpoint: 128,
                step_ticks: 128,
                maximum_attempts: 1,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        },
        advective_limit: 0.3,
    };
    let admission = Region::new(GLOBAL);
    let plan = Plan::from_rest_cached(settings, 1 << 26).unwrap();
    assert!(Plan::from_rest_cached(settings, plan.resources().total() - 1).is_err());
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));
    let construction = Region::new(GLOBAL);
    let mut run = Run::from_rest(plan).unwrap();
    assert!(construction.change().bytes_allocated <= plan.resources().total());
    let steady = Region::new(GLOBAL);
    assert!(matches!(run.step().unwrap(), Outcome::Committed(_)));
    let used = steady.change();
    assert_eq!(
        (used.allocations, used.deallocations, used.reallocations),
        (0, 0, 0)
    );
    let work = run.cache_work().unwrap();
    assert_eq!((work.calls, work.provider_evaluations), (12, 5));
}
