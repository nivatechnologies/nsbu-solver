//! Small but fully independent six-branch experiment profile.
use nsbu_benchmarks::smooth_experiment::FamilySettings;
use nsbu_solver::{domain::TickClock, integrators::indicator::Tolerances};
pub const CAP: usize = 128 * 1024 * 1024;
pub fn settings(tolerance: f64) -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        viscosity: 1.0,
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [tolerance; 2],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}
pub fn clocks() -> [TickClock; 3] {
    [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap())
}
