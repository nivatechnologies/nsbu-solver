//! Identical bounded actual-from-rest evolution for state and derived-observable exports.
use nsbu_benchmarks::smooth_run::{ReconstructedPlan, ReconstructedRun};
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    SolverError,
};

pub fn plan(n: usize, method: Method, cap: usize) -> Result<ReconstructedPlan, SolverError> {
    if !matches!(n, 4 | 8 | 12) {
        return Err(SolverError::InvalidDomain);
    }
    ReconstructedPlan::from_rest(
        Domain::new([n; 3], [1.0; 3], 1.0)?,
        TickClock::from_rest(-16, 512)?,
        Configuration {
            method,
            limits: RunLimits {
                endpoint: 128,
                step_ticks: 16,
                maximum_attempts: 8,
            },
            tolerances: Tolerances {
                absolute: [1e-2; 2],
                relative: [0.0; 2],
            },
        },
        9,
        0.3,
        cap,
    )
}

// Numerical evolution owns all guards and state; serialization only borrows its completed result.
pub fn evolve(n: usize, method: Method, cap: usize) -> Result<ReconstructedRun, SolverError> {
    let plan = plan(n, method, cap)?;
    let mut run = ReconstructedRun::from_rest(
        plan.resources().domain(),
        TickClock::from_rest(-16, 512)?,
        plan.configuration(),
        plan.observer_samples(),
        0.3,
        cap,
    )?;
    for _ in 0..8 {
        if !matches!(run.step()?, Outcome::Committed(_)) {
            return Err(SolverError::RetryLimit);
        }
    }
    Ok(run)
}
