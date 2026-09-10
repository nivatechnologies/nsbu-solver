//! Complete physical/pressure refinements from independently evolved accepted-history probes.
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        diagnostics::{ProbeDiagnostics, ProbeDiagnosticsPlan},
        ProbeFamily, ProbePlan,
    },
    FamilyError, FamilyPlan, FamilySettings,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
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
    let times = [0, 7, 128].map(clock);
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
    let probes = ProbePlan::new(base, TestedTimes::new(&times, 3)?, 3, CAP)?;
    let plan = ProbeDiagnosticsPlan::new(
        probes,
        Layout::new([24; 3])?,
        [1e-8, 1e-7, 1e-6, 1e-7, 1e-8, 1e-7],
        3,
        CAP,
    )?;
    println!("CyclicSine complete physical probes; six independent rest trajectories; accepted concentrating windows=0");
    println!("grids=[4,8,12] macro_ticks=[64,32,16] quantum=2^-16 endpoint=128 probe_ticks=[0,7,128] sample_grid=24 methods=CM,HO");
    println!(
        "cap={CAP} probe_bounds={:?} diagnostic_bounds={:?}",
        probes.bounds(),
        plan.bounds()
    );
    if !dry {
        evolve(plan)?;
    }
    Ok(())
}
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).expect("fixed public physical clock profile")
}
fn evolve(plan: ProbeDiagnosticsPlan<'_>) -> Result<(), FamilyError> {
    let mut family = ProbeFamily::new(plan.probe_plan())?;
    let mut diagnostics = ProbeDiagnostics::new(plan)?;
    while family.advance()?.is_some() {
        let report = diagnostics.measure(&family)?;
        println!(
            "probe={} origins={:?}",
            report.clock().elapsed(),
            report.reconstruction().origins()
        );
        for quantity in report.quantities() {
            println!("quantity={quantity:?}");
        }
    }
    println!("charged={:?}; reconstructed diagnostic fields never replace integrated state; no continuous-window enclosure",diagnostics.charged_work());
    Ok(())
}
#[test]
fn public_preflight_and_complete_physical_probe_workflow_execute() {
    execute(&["--dry-run".into()]).unwrap();
    execute(&[]).unwrap();
    assert!(execute(&["bad".into()]).is_err());
    assert!(execute(&["--dry-run".into(), "extra".into()]).is_err());
}
