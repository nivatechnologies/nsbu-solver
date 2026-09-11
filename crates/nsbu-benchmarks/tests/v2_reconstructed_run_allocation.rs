//! Isolated allocation audit for the exact-v2 reconstruction owner.

use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    smooth_observer::{v2::V2Observer, v2_reconstruction::V2ReconstructionObserver},
    v2_run::{ReconstructedPlan, ReconstructedRun, Settings},
};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, Layout, ResourcePlan, SpectralState, TickClock},
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
                endpoint: 384,
                step_ticks: 128,
                maximum_attempts: 3,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [0.0; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn main() {
    let settings = settings();
    let admission = Region::new(GLOBAL);
    let plan = ReconstructedPlan::from_rest(settings, 64 * 1024 * 1024).unwrap();
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));
    assert!(ReconstructedPlan::from_rest(settings, plan.resources().total() - 1).is_err());

    let balance =
        V2Observer::limits(settings.domain, settings.force, plan.observer_samples()).unwrap();
    let insufficient_plan = ResourcePlan::new(
        settings.domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: balance.storage_bytes,
            overhead: 0,
        },
        64 * 1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let rest =
        SpectralState::from_rest(insufficient_plan, settings.initial_clock, Epoch(0)).unwrap();
    let insufficient = Region::new(GLOBAL);
    assert!(V2ReconstructionObserver::new_v2(
        insufficient_plan,
        settings.force,
        plan.observer_samples(),
        &rest,
    )
    .is_err());
    let rejected = insufficient.change();
    assert_eq!((rejected.allocations, rejected.reallocations), (0, 0));

    let invalid = Region::new(GLOBAL);
    assert!(V2ReconstructionObserver::new_v2(
        plan.resources(),
        settings.force,
        plan.observer_samples(),
        &rest,
    )
    .is_err());
    let rejected = invalid.change();
    assert_eq!((rejected.allocations, rejected.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut run = ReconstructedRun::from_rest(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.resources().total());
    for _ in 0..2 {
        let steady = Region::new(GLOBAL);
        assert!(matches!(run.step().unwrap(), Outcome::Committed(_)));
        let spent = steady.change();
        assert_eq!(
            (spent.allocations, spent.deallocations, spent.reallocations),
            (0, 0, 0)
        );
    }
    println!("v2 reconstruction construction_bytes={} declared_bytes={} initial_and_endpoint_samples={} observer_work_units={} observer_transforms={} modal_visits={} steady_allocations=0", built.bytes_allocated, plan.resources().total(), run.observer_work().samples, run.observer_work().work_units, run.observer_work().scalar_transforms, run.observer().modal_visits());
}
