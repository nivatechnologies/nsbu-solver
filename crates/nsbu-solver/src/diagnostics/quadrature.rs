//! Simpson balance quadrature over explicitly supplied start/midpoint/endpoint samples.
use super::{balances::BalanceSample, hermite::coordinates, squares::finite, summation::Sum};
use crate::{domain::TickClock, SolverError};

/// Measured integral and endpoint defects. Each experiment must refine this quadrature separately.
#[derive(Debug, Clone, Copy)]
pub struct BalanceIntegral {
    /// Integrated forcing work minus viscous energy dissipation.
    pub energy_rhs: f64,
    /// Integrated stretching plus vorticity forcing minus enstrophy dissipation.
    pub enstrophy_rhs: f64,
    /// Endpoint energy change minus the measured integrated right-hand side.
    pub energy_defect: f64,
    /// Endpoint enstrophy change minus the measured integrated right-hand side.
    pub enstrophy_defect: f64,
}

/// This function checks clock geometry, not sample provenance. Accepted-state lineage is
/// the experiment's responsibility; integrator stage values are not accepted history samples.
pub fn simpson(
    nodes: [TickClock; 3],
    samples: [BalanceSample; 3],
) -> Result<BalanceIntegral, SolverError> {
    let (_, duration) = coordinates(nodes, nodes[0])?;
    let weights = [duration / 6.0, duration * (2.0 / 3.0), duration / 6.0];
    if weights[0] == 0.0 {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    let mut energy = Sum::default();
    let mut enstrophy = Sum::default();
    for (sample, weight) in samples.into_iter().zip(weights) {
        energy.add(weight * finite(sample.forcing_work - sample.energy_dissipation)?)?;
        enstrophy.add(
            weight
                * finite(
                    sample.stretching + sample.vorticity_forcing - sample.enstrophy_dissipation,
                )?,
        )?;
    }
    let energy_rhs = energy.finish()?;
    let enstrophy_rhs = enstrophy.finish()?;
    Ok(BalanceIntegral {
        energy_rhs,
        enstrophy_rhs,
        energy_defect: finite(samples[2].energy - samples[0].energy - energy_rhs)?,
        enstrophy_defect: finite(samples[2].enstrophy - samples[0].enstrophy - enstrophy_rhs)?,
    })
}
