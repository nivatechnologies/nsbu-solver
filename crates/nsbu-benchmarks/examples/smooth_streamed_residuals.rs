//! Collect early and late conservative residuals from six actual accepted histories.
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        residuals::{ResidualFamily, ResidualFamilyPlan},
        ProbeFamily, ProbePlan,
    },
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
    let accepted = [0, 64, 128].map(clock);
    let probes = [0, 7, 31, 63, 95, 127, 128].map(clock);
    let base = FamilyPlan::new(
        FamilySettings {
            grids: [4, 8, 12],
            steps: [64, 32, 16],
            viscosity: 1.0,
            endpoint: 128,
            tolerances: Tolerances {
                absolute: [1e-2; 2],
                relative: [0.0; 2],
            },
            advective_limit: 0.3,
        },
        TestedTimes::new(&accepted, 3)?,
        CAP,
    )?;
    let owner = ProbePlan::new(base, TestedTimes::new(&probes, 7)?, 7, CAP)?;
    let plan = ResidualFamilyPlan::new(owner, &probes[1..6], 5, CAP)?;
    println!("CyclicSine independent streamed residuals; all six trajectories evolve from rest; accepted concentrating windows=0");
    println!("grids=[4,8,12] macro_ticks=[64,32,16] quantum=2^-16 endpoint=128 residual_ticks=[7,31,63,95,127] methods=CM,HO");
    println!(
        "cap={CAP} owner_bounds={:?} residual_bounds={:?}",
        owner.bounds(),
        plan.bounds()
    );
    if !dry {
        evolve(plan)?;
    }
    Ok(())
}
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).expect("fixed public residual clock profile")
}
fn evolve(plan: ResidualFamilyPlan<'_>) -> Result<(), FamilyError> {
    let mut family = ProbeFamily::new(plan.probe_plan())?;
    let mut residuals = ResidualFamily::new(plan)?;
    while let Some(probe) = family.advance()? {
        if residuals.next_time() != Some(probe.clock()) {
            continue;
        }
        let result = residuals.measure(&family)?;
        println!(
            "probe={} origins={:?} temporal={:?}",
            result.clock().elapsed(),
            result.reconstruction().origins(),
            result.temporal_geometry()
        );
        for (index, branch) in result.branches().iter().enumerate() {
            println!("branch={index} residual={:?}", branch.norms());
        }
        for (index, comparison) in result.comparisons().iter().enumerate() {
            println!("pair={index} complete_residual_difference={comparison:?}");
        }
    }
    println!("charged={:?} children={:?}; genuine non-stage sampling and nested geometry are not continuous-window error bounds",residuals.charged_work(),residuals.child_work());
    Ok(())
}
#[test]
fn public_streamed_residual_preflight_and_complete_workflow_execute() {
    execute(&["--dry-run".into()]).unwrap();
    execute(&[]).unwrap();
    assert!(execute(&["bad".into()]).is_err());
    assert!(execute(&["--dry-run".into(), "extra".into()]).is_err());
}
