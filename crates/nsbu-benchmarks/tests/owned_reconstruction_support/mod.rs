//! Bounded owned reconstruction profile shared by numerical and allocation contracts.
use nsbu_benchmarks::smooth_run::ReconstructedRun;
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};
pub const CAP: usize = 8 * 1024 * 1024;
pub fn configuration(method: Method, tolerance: f64) -> Configuration {
    Configuration {
        method,
        limits: RunLimits {
            endpoint: 4096,
            step_ticks: 1024,
            maximum_attempts: 4,
        },
        tolerances: Tolerances {
            absolute: [tolerance; 2],
            relative: [0.0; 2],
        },
    }
}
pub fn run(method: Method, tolerance: f64, samples: usize) -> ReconstructedRun {
    ReconstructedRun::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-20, 1 << 20).unwrap(),
        configuration(method, tolerance),
        samples,
        0.3,
        CAP,
    )
    .unwrap()
}
