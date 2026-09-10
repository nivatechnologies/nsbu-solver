//! Stream a complete fine time manifest from independent accepted histories with bounded lookahead.
use nsbu_benchmarks::smooth_experiment::{
    probes::{ProbeFamily, ProbePlan},
    FamilyError, FamilyPlan, FamilySettings,
};
use nsbu_solver::{
    domain::TickClock, integrators::indicator::Tolerances, verification::times::TestedTimes,
    SolverError,
};
const CAP: usize = 128 * 1024 * 1024;
fn main() -> Result<(), FamilyError> {
    execute(&std::env::args().skip(1).collect::<Vec<_>>())
}
fn execute(args: &[String]) -> Result<(), FamilyError> {
    let dry = match args {
        [] => false,
        [flag] if flag == "--dry-run" => true,
        _ => return Err(SolverError::InvalidPayload.into()),
    };
    let coarse = [0, 64, 128].map(clock);
    let middle = [0, 31, 64, 95, 128].map(clock);
    let fine = [0, 7, 31, 63, 64, 95, 127, 128].map(clock);
    let coarse = TestedTimes::new(&coarse, 3)?;
    let middle = TestedTimes::new(&middle, 5)?;
    let fine = TestedTimes::new(&fine, 8)?;
    if !middle.refines(coarse) || !fine.refines(middle) {
        return Err(FamilyError::InvalidFamily);
    }
    let settings = FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        viscosity: 1.0,
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-2; 2],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    };
    let family = FamilyPlan::new(settings, coarse, CAP)?;
    let plan = ProbePlan::new(family, fine, 8, CAP)?;
    println!("CyclicSine accepted-history probes; independent from rest; accepted concentrating windows=0");
    println!(
        "grids=[4,8,12] macro_ticks=[64,32,16] quantum=2^-16 endpoint_ticks=128 methods=CM,HO"
    );
    println!("time_sets=[0,64,128];[0,31,64,95,128];[0,7,31,63,64,95,127,128]");
    println!(
        "cap={CAP} bounds={:?}; initial lookahead=two actual macro steps per branch",
        plan.bounds()
    );
    if !dry {
        evolve(plan)?;
    }
    Ok(())
}
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).expect("fixed public clock profile")
}
fn evolve(plan: ProbePlan<'_>) -> Result<(), FamilyError> {
    let mut family = ProbeFamily::new(plan)?;
    while let Some(sample) = family.advance()? {
        println!(
            "probe_ticks={} state_ticks={:?} nodes={:?} full_field_H1={:?} derivative_H1={:?}",
            sample.clock().elapsed(),
            sample.origins().map(|o| o.state_clock.elapsed()),
            sample
                .origins()
                .map(|o| o.accepted_nodes.map(|t| t.elapsed())),
            sample.values().map(|v| v.full.h1),
            sample.derivatives().map(|v| v.full.h1)
        );
    }
    println!("charged={:?}; interpolation is diagnostic, not an accepted-state replacement or a window enclosure",family.charged_work());
    Ok(())
}
#[test]
fn documented_nested_time_probe_workflow_and_preflight_execute() {
    execute(&["--dry-run".to_owned()]).unwrap();
    execute(&[]).unwrap();
    assert!(execute(&["bad".to_owned()]).is_err());
    assert!(execute(&["--dry-run".to_owned(), "extra".to_owned()]).is_err());
}
