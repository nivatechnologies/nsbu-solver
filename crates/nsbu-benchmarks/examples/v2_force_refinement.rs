//! Bounded fixed-N exact-v2 force-grid refinement; no sufficiency status is emitted.
use nsbu_benchmarks::{
    v2_force_experiment::{ForceFamily, ForceFamilyError, ForceFamilyPlan, ForceFamilySettings},
    CASE_SHA256,
};
use nsbu_solver::{
    domain::TickClock,
    integrators::{indicator::Tolerances, method::Method},
    verification::times::TestedTimes,
};

fn main() -> Result<(), ForceFamilyError> {
    let clocks = [0, 256, 512].map(|t| TickClock::restore(-20, 8192, t, 8192 - t));
    let clocks = [clocks[0]?, clocks[1]?, clocks[2]?];
    let settings = ForceFamilySettings {
        grid: 4,
        force_grids: [4, 8, 16],
        workers: 0,
        step_ticks: 64,
        method: Method::CoxMatthews,
        endpoint: 512,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    };
    let plan = ForceFamilyPlan::new(
        settings,
        TestedTimes::new(&clocks, clocks.len())?,
        128 * 1024 * 1024,
    )?;
    println!("case=similarity-mms-v2 sha256={CASE_SHA256}");
    println!(
        "force_family_identity={:02x?} preflight={:?}",
        plan.identity(),
        plan.bounds()
    );
    println!("fixed_N=4 force_grids=[4,8,16] fixed_step=64 method=CM quantum=2^-20 endpoint=512");
    let mut family = ForceFamily::from_rest(plan)?;
    while let Some(sample) = family.advance()? {
        println!(
            "elapsed={} force_pair_full_velocity_L2={:?} force_pair_full_H1={:?}",
            sample.clock().elapsed(),
            sample.comparisons().map(|item| item.full.l2),
            sample.comparisons().map(|item| item.full.h1)
        );
    }
    println!("diagnostic-only; no force-sufficiency, convergence, or accepted-window claim; accepted_pde_windows=0");
    Ok(())
}

#[test]
fn documented_force_family_reaches_its_post_startup_endpoint() {
    main().unwrap();
}
