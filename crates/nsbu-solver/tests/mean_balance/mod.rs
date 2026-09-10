//! Physical balance of the shared spatially uniform cosine-force test trajectory.
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, norms::Norms},
    domain::SpectralState,
};
pub fn measured(state: &SpectralState) -> BalanceSample {
    let velocity: [f64; 3] = std::array::from_fn(|axis| state.component(axis).unwrap()[0].re);
    let l2 = velocity.iter().map(|u| u * u).sum::<f64>().sqrt();
    let force = (state.clock().elapsed() as f64).cos();
    BalanceSample {
        norms: Norms {
            l2,
            h1: l2,
            vorticity_l2: 0.0,
            divergence_l2: 0.0,
        },
        energy: 0.5 * l2 * l2,
        enstrophy: 0.0,
        energy_dissipation: 0.0,
        forcing_work: force * velocity.iter().sum::<f64>(),
        stretching: 0.0,
        enstrophy_dissipation: 0.0,
        vorticity_forcing: 0.0,
    }
}
