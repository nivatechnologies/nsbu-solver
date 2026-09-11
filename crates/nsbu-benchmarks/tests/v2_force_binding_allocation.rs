//! Isolated planning, construction, and steady-binding allocation audit.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyPlan, FamilySettings, V2Family},
    v2_force_experiment::{
        binding::{ForceBindingPlan, ForceBindingWorkspace},
        ForceFamily, ForceFamilyPlan, ForceFamilySettings,
    },
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::{indicator::Tolerances, method::Method},
    verification::times::TestedTimes,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;
const CAP: usize = 256 * 1024 * 1024;

fn main() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap());
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let tolerances = Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [0.0; 2],
    };
    let force_plan = ForceFamilyPlan::new(
        ForceFamilySettings {
            grid: 4,
            force_grids: [8, 16, 32],
            workers: 0,
            step_ticks: 16,
            method: Method::CoxMatthews,
            endpoint: 128,
            tolerances,
            advective_limit: 0.3,
        },
        times,
        CAP,
    )
    .unwrap();
    let ordinary_plan = FamilyPlan::new(
        FamilySettings {
            grids: [4, 8, 12],
            steps: [64, 32, 16],
            force: ForceSettings {
                samples: Layout::new([16; 3]).unwrap(),
                workers: 0,
            },
            endpoint: 128,
            tolerances,
            advective_limit: 0.3,
        },
        times,
        CAP,
    )
    .unwrap();
    let admission = Region::new(GLOBAL);
    let plan = ForceBindingPlan::new(force_plan, ordinary_plan, 1, 0, CAP).unwrap();
    assert!(ForceBindingPlan::new(
        force_plan,
        ordinary_plan,
        1,
        0,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut force = ForceFamily::from_rest(force_plan).unwrap();
    let mut ordinary = V2Family::new(ordinary_plan).unwrap();
    let mut binding = ForceBindingWorkspace::new(plan);
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    let steady = Region::new(GLOBAL);
    for _ in clocks {
        ordinary.advance().unwrap().unwrap();
        let raw = force.advance().unwrap().unwrap();
        binding.measure(&force, &ordinary, raw).unwrap();
    }
    let spent = steady.change();
    assert_eq!(
        (spent.allocations, spent.deallocations, spent.reallocations),
        (0, 0, 0)
    );
    println!(
        "force binding construction_bytes={} joint_bytes={} coefficient_words={} steady_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
        binding.charged_work().1
    );
}
