//! Shared declared storage for bounded unit-domain Cox–Matthews trajectory tests.
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan},
    integrators::attempt::AttemptWorkspace,
};
pub fn plan() -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 4096,
        },
        1 << 20,
        Epoch(0),
    )
    .unwrap()
}
