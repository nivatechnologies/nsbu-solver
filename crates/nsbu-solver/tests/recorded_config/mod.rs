//! Shared fixed-step configuration for runtime and private malformed-holder fixtures.
use nsbu_solver::{
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};
pub fn configuration() -> Configuration {
    Configuration {
        limits: RunLimits {
            endpoint: 8,
            step_ticks: 4,
            maximum_attempts: 3,
        },
        method: Method::CoxMatthews,
        tolerances: Tolerances {
            absolute: [1.0; 2],
            relative: [0.0; 2],
        },
    }
}
