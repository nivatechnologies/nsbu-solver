//! Shared basic state-only reservation; attempt/diagnostic storage is separate.
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan};
pub fn plan(epoch: Epoch) -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let extra = ExtraStorage {
        fft: 0,
        force: 0,
        diagnostics: 0,
        overhead: 4096,
    };
    ResourcePlan::new(domain, extra, 1 << 20, epoch).unwrap()
}
