//! Run a small bounded `CyclicSine` trajectory from rest and print recorded measurements.
use nsbu_benchmarks::smooth_run::SmoothRun;
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{
        indicator::Tolerances,
        method::Method,
        trajectory::{RunLimits, StopReason},
    },
    SolverError,
};

fn main() -> Result<(), SolverError> {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0)?;
    let clock = TickClock::from_rest(-20, 1 << 20)?;
    let configuration = Configuration {
        limits: RunLimits {
            endpoint: 4096,
            step_ticks: 1024,
            maximum_attempts: 4,
        },
        method: Method::CoxMatthews,
        tolerances: Tolerances {
            absolute: [1e-2; 2],
            relative: [0.0; 2],
        },
    };
    let observer_samples = 4;
    let mut plan = SmoothRun::from_rest(
        domain,
        clock,
        configuration,
        observer_samples,
        0.3,
        8 * 1024 * 1024,
    )?;
    while plan.history().controller().stopped().is_none() {
        match plan.step()? {
            Outcome::Committed(indicators) => {
                let record = plan
                    .history()
                    .records()
                    .last()
                    .ok_or(SolverError::InvalidPayload)?;
                let sample = record.sample.ok_or(SolverError::InvalidPayload)?;
                println!(
                    "committed elapsed={} energy={:.6e} enstrophy={:.6e} ratios={:?}",
                    plan.state().clock().elapsed(),
                    sample.energy,
                    sample.enstrophy,
                    indicators.ratios,
                );
            }
            Outcome::Rejected(indicators) => {
                println!("local-error rejection: ratios={:?}", indicators.ratios);
            }
            Outcome::Refused { cause, indicators } => {
                println!("bounded refusal: cause={cause:?} indicators={indicators:?}");
            }
        }
    }
    let controller = plan.history().controller();
    let diagnostic = plan.observer_work();
    println!(
        "stopped={:?} attempts={} committed={} observer_samples={} observer_work={} transforms={}",
        controller.stopped(),
        controller.attempted(),
        controller.committed(),
        diagnostic.samples,
        diagnostic.work_units,
        diagnostic.scalar_transforms,
    );
    if controller.stopped() != Some(StopReason::EndpointReached) {
        return Err(SolverError::RetryLimit);
    }
    Ok(())
}

#[test]
fn documented_example_reaches_its_endpoint() {
    main().expect("the documented bounded example must complete");
}
