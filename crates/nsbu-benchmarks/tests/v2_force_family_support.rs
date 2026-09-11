//! Fixed post-startup profile for the exact-v2 force-sampling family.
use nsbu_benchmarks::v2_force_experiment::ForceFamilySettings;
use nsbu_solver::{
    domain::TickClock,
    integrators::{indicator::Tolerances, method::Method},
};

/// Aggregate cap used by the bounded test profiles.
pub const CAP: usize = 128 * 1024 * 1024;

/// Fixed N4/M4-M8-M16 post-startup settings with a selectable error tolerance.
pub fn settings(tolerance: f64) -> ForceFamilySettings {
    ForceFamilySettings {
        grid: 4,
        force_grids: [4, 8, 16],
        workers: 0,
        step_ticks: 64,
        method: Method::CoxMatthews,
        endpoint: 512,
        tolerances: Tolerances {
            absolute: [tolerance, 10.0 * tolerance],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}

/// Rest, intermediate and endpoint clocks for the post-startup profile.
#[allow(dead_code)]
pub fn clocks() -> [TickClock; 3] {
    [0, 256, 512].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}
