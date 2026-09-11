//! Bounded independently evolved concentrating family; no window qualification is emitted.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyError, FamilyPlan, FamilySettings, V2Family},
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};

fn main() -> Result<(), FamilyError> {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-20, 8192, t, 8192 - t));
    let clocks = [clocks[0]?, clocks[1]?, clocks[2]?];
    let settings = FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3])?,
            workers: 0,
        },
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [1e-5; 2],
        },
        advective_limit: 0.3,
    };
    let plan = FamilyPlan::new(settings, TestedTimes::new(&clocks, 3)?, 128 * 1024 * 1024)?;
    println!("case=similarity-mms-v2 sha256={CASE_SHA256}");
    println!(
        "family_identity={:02x?} preflight={:?}",
        plan.identity(),
        plan.bounds()
    );
    println!("grids=[4,8,12] fixed_force_grid=12 steps=[64,32,16] quantum=2^-20 endpoint=128");
    let mut family = V2Family::new(plan)?;
    while let Some(sample) = family.advance()? {
        println!(
            "elapsed={} space_full_H1={:?} space_new_modes_H1={:?} time_full_H1={:?} method_full_H1={:.12e}",
            sample.clock().elapsed(),
            sample.space().map(|c| c.full.h1),
            sample.space().map(|c| c.newly_resolved.h1),
            sample.time().map(|c| c.full.h1),
            sample.method().full.h1
        );
    }
    println!("diagnostic-only; accepted_pde_windows=0; force/reference/arithmetic/local and off-stage qualification remain missing");
    Ok(())
}

#[test]
fn documented_exact_v2_family_reaches_all_declared_sample_times() {
    main().unwrap();
}
