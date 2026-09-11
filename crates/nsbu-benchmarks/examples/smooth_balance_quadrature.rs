//! Refine physical-time balance quadrature without changing independently integrated trajectories.
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        balances::{
            quadrature::{BalanceQuadrature, QuadraturePlan},
            BalanceProbePlan,
        },
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
    let middle = [0, 32, 64, 96, 128].map(clock);
    let fine = [0, 16, 32, 48, 64, 80, 96, 112, 128].map(clock);
    let probes = [0, 7, 16, 32, 48, 64, 80, 96, 112, 128].map(clock);
    let sets = [&accepted[..], &middle[..], &fine[..]].map(|s| TestedTimes::new(s, s.len()));
    let [a, b, c] = sets;
    let sets = [a?, b?, c?];
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
        sets[0],
        CAP,
    )?;
    let owner = ProbePlan::new(base, TestedTimes::new(&probes, 10)?, 10, CAP)?;
    let balance = BalanceProbePlan::new(owner, 10, CAP)?;
    let plan = QuadraturePlan::new(balance, sets, 1000, CAP)?;
    println!("CyclicSine independent balance quadrature; all six trajectories evolve from rest; accepted concentrating windows=0");
    println!("grids=[4,8,12] macro_ticks=[64,32,16] quantum=2^-16 endpoint=128 probe_ticks=[0,7,16,32,48,64,80,96,112,128] methods=CM,HO");
    println!(
        "quadrature_samples=[3,5,9] cap={CAP} balance_bounds={:?} quadrature_bounds={:?}",
        balance.bounds(),
        plan.bounds()
    );
    if !dry {
        evolve(plan)?;
    }
    Ok(())
}
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).expect("fixed public quadrature clock profile")
}
fn evolve(plan: QuadraturePlan<'_>) -> Result<(), FamilyError> {
    let mut owner = ProbeFamily::new(plan.balance_plan().probe_plan())?;
    let mut consumer = BalanceQuadrature::new(plan)?;
    while owner.advance()?.is_some() {
        let sample = consumer.measure(&owner)?;
        println!(
            "probe={} origins={:?} quadrature_counts={:?}",
            sample.clock().elapsed(),
            sample.reconstruction().origins(),
            consumer.sample_counts()
        );
        for (branch, balance) in sample.branches().iter().enumerate() {
            println!("branch={branch} balance={balance:?}");
        }
    }
    let report = consumer.report().ok_or(FamilyError::InvalidFamily)?;
    for (level, integrals) in report.integrals().iter().enumerate() {
        for (branch, integral) in integrals.iter().enumerate() {
            println!("quadrature_level={level} branch={branch} integral={integral:?}");
        }
    }
    println!("charged={:?} quadrature_charged={:?}; reconstructed sample and quadrature errors require separate budgets",consumer.charged_work(),consumer.charged_quadrature_work());
    Ok(())
}
#[test]
fn public_balance_preflight_and_complete_three_level_workflow_execute() {
    execute(&["--dry-run".into()]).unwrap();
    execute(&[]).unwrap();
    assert!(execute(&["bad".into()]).is_err());
    assert!(execute(&["--dry-run".into(), "extra".into()]).is_err());
}
