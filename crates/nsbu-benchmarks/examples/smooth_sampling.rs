//! Complete sampling refinements over a bounded six-trajectory smooth experiment.
use nsbu_benchmarks::smooth_experiment::{
    sampling::{SamplingPlan, SamplingWorkspace},
    FamilyError, FamilyPlan, FamilySettings, SmoothFamily,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
    SolverError,
};
const CAP: usize = 128 * 1024 * 1024;

fn main() -> Result<(), FamilyError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    execute(&args)
}
fn execute(args: &[String]) -> Result<(), FamilyError> {
    let dry = match args {
        [] => false,
        [flag] if flag == "--dry-run" => true,
        _ => return Err(SolverError::InvalidPayload.into()),
    };
    let clocks = [0, 64, 128]
        .map(|t| TickClock::restore(-16, 512, t, 512 - t))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let (family, plan) = admit(&clocks, CAP)?;
    println!("CyclicSine independent-from-rest sampling study; accepted concentrating windows=0");
    println!(
        "grids=[4,8,12] macro_ticks=[64,32,16] quantum=2^-16 endpoint_ticks=128 methods=CM,HO"
    );
    println!(
        "sampling={:?} floors={:?} cap={CAP} bounds={:?}",
        plan.sample_layouts().map(|v| v.dimensions()),
        plan.relative_floors(),
        plan.bounds()
    );
    if !dry {
        evolve(family, plan)?;
    }
    Ok(())
}
fn admit(
    clocks: &[TickClock],
    cap: usize,
) -> Result<(FamilyPlan<'_>, SamplingPlan<'_>), FamilyError> {
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
    let family = FamilyPlan::new(settings, TestedTimes::new(clocks, 3)?, cap)?;
    let layouts = [24, 32, 48].map(|n| Layout::new([n; 3]));
    let [a, b, c] = layouts;
    let plan = SamplingPlan::new(
        family,
        [a?, b?, c?],
        [1e-8, 1e-7, 1e-6, 1e-7, 1e-8, 1e-7],
        3,
        cap,
    )?;
    Ok((family, plan))
}
fn evolve(plan: FamilyPlan<'_>, sampling: SamplingPlan<'_>) -> Result<(), FamilyError> {
    let mut family = SmoothFamily::new(plan)?;
    let mut consumer = SamplingWorkspace::new(sampling)?;
    while family.advance()?.is_some() {
        let sample = consumer.measure(&family)?;
        for quantity in sample.quantities() {
            for pair in 0..5 {
                let values = quantity.pair(pair)?;
                println!(
                    "tick={} {:?} pair={} rms={:?} peak={:?} relative_peak={:?} changes={:?}",
                    sample.clock().elapsed(),
                    quantity.quantity(),
                    pair,
                    values.map(|v| v.rms_error),
                    values.map(|v| v.peak_error),
                    values.map(|v| v.peak_relative_error),
                    quantity.changes(pair)?
                );
            }
        }
    }
    println!(
        "charged={:?}; no sampling convergence or error floor is inferred automatically",
        consumer.charged_work()
    );
    Ok(())
}

#[test]
fn documented_sampling_walkthrough_and_preflight_execute() {
    execute(&["--dry-run".to_owned()]).unwrap();
    execute(&[]).unwrap();
    assert!(execute(&["bad".to_owned()]).is_err());
    assert!(execute(&["--dry-run".to_owned(), "extra".to_owned()]).is_err());
}
#[test]
fn complete_joint_reservation_is_required_before_allocating() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let (_, plan) = admit(&clocks, CAP).unwrap();
    assert!(admit(&clocks, plan.bounds().joint_storage_bytes - 1).is_err());
}
