use nsbu_benchmarks::smooth_observer::{BalanceObserver, BalanceObserverLimits};
use nsbu_solver::{domain::Domain, integrators::method::Method};

#[path = "../smooth_support/mod.rs"]
mod smooth_support;

pub fn run_with_observer(
    method: Method,
    samples: usize,
) -> (
    smooth_support::SmoothRun,
    BalanceObserver,
    BalanceObserverLimits,
) {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).expect("fixed smooth domain is valid");
    let limits = BalanceObserver::limits(domain, samples).expect("observer declaration is bounded");
    let run = smooth_support::SmoothRun::new(domain, method, limits.storage_bytes);
    let observer = BalanceObserver::new(run.state.plan(), samples)
        .expect("run plan reserves the observer diagnostic class");
    (run, observer, limits)
}
