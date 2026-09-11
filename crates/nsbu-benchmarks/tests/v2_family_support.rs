//! Fixed admissible startup profile used by the exact-v2 family tests.
use nsbu_benchmarks::{runtime_force::ForceSettings, v2_experiment::FamilySettings};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
};

/// Joint family reservation limit.
pub const CAP: usize = 256 * 1024 * 1024;

/// Preserve the alpha absolute-tolerance ratio with caller-selected magnitude.
pub fn settings(tolerance: f64) -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [tolerance, 10.0 * tolerance],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}

/// Two positive startup samples synchronized with the largest step.
pub fn clocks() -> [TickClock; 3] {
    [0, 64, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}
