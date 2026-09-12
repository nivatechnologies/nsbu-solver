//! Allocation isolation for one actual globally gauged pressure report.
use nsbu_benchmarks::v2_experiment::{
    diagnostic::StartupProfile,
    pressure_reference::{ImportedGauge, PressureReferencePlan, PressureReferenceWorkspace},
    V2Family,
};
use nsbu_solver::domain::Layout;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;
const CAP: usize = 256 * 1024 * 1024;
const CLOCK0: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock0.json");
const CLOCK64: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock64.json");
const CLOCK128: &[u8] = include_bytes!("../data/v2-pressure-gauge/clock128.json");

fn main() {
    let gauges =
        [CLOCK0, CLOCK64, CLOCK128].map(|bytes| ImportedGauge::load(bytes, bytes.len()).unwrap());
    let startup = StartupProfile::new().unwrap();
    let diagnostic = startup.plan(CAP).unwrap();
    let family_plan = diagnostic.family_plan();
    let plan = PressureReferencePlan::new(
        family_plan,
        gauges,
        Layout::new([24; 3]).unwrap(),
        [1e-8, 1e-7],
        CAP,
    )
    .unwrap();
    let bounds = plan.bounds();
    let mut family = V2Family::new(family_plan).unwrap();
    let construction = Region::new(GLOBAL);
    let mut workspace = PressureReferenceWorkspace::new(plan).unwrap();
    let construction_change = construction.change();
    assert!(construction_change.bytes_allocated <= bounds.storage_bytes);
    family.advance().unwrap();
    let region = Region::new(GLOBAL);
    let sample = workspace.measure(&family).unwrap();
    let change = region.change();
    assert_eq!(sample.clock().elapsed(), 0);
    assert_eq!(sample.branches().len(), 6);
    assert_eq!(change.allocations, 0);
    assert_eq!(change.deallocations, 0);
    eprintln!(
        "pressure reference construction_bytes={} consumer_bytes={} joint_bytes={} hash_bytes={} reference_evaluations={} transforms={} steady_allocations={}",
        construction_change.bytes_allocated,
        bounds.storage_bytes,
        bounds.joint_storage_bytes,
        bounds.imported_gauge_hash_bytes,
        workspace.charged_work().reference_evaluations,
        workspace.charged_work().scalar_transforms,
        change.allocations
    );
}
