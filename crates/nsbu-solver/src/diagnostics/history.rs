//! Restartable measured balance quadrature; callers must supply committed-state provenance.
use super::{
    balances::BalanceSample,
    quadrature::{simpson, BalanceIntegral},
    squares::finite,
    summation::Sum,
};
use crate::{domain::TickClock, SolverError};

/// Constant-storage balance history including compensation and an unfinished Simpson pair.
/// Copying preserves all arithmetic history; it does not qualify the source samples.
#[derive(Debug, Clone, Copy)]
pub struct BalanceHistory {
    nodes: [(TickClock, BalanceSample); 2],
    initial: [f64; 2],
    sums: [Sum; 2],
    pending: bool,
    samples: usize,
    maximum_samples: usize,
}
impl BalanceHistory {
    /// Start at exact zero; reserve a finite number of accepted diagnostic samples.
    pub fn new(
        clock: TickClock,
        sample: BalanceSample,
        maximum_samples: usize,
    ) -> Result<Self, SolverError> {
        if clock.elapsed() != 0 {
            return Err(SolverError::InvalidClock);
        }
        if maximum_samples < 3 {
            return Err(SolverError::ResourceLimit);
        }
        validate(sample)?;
        Ok(Self {
            nodes: [(clock, sample); 2],
            initial: [sample.energy, sample.enstrophy],
            sums: [Sum::default(); 2],
            pending: false,
            samples: 1,
            maximum_samples,
        })
    }
    /// Latest supplied sample clock, including a pending midpoint.
    pub fn clock(self) -> TickClock {
        self.nodes[usize::from(self.pending)].0
    }
    /// Samples retained in the accumulated arithmetic, including the initial sample.
    pub fn samples(self) -> usize {
        self.samples
    }
    /// True when an endpoint is still required to complete the current Simpson pair.
    pub fn has_pending_midpoint(self) -> bool {
        self.pending
    }
    /// Form a private proposed history; the caller retains its original on failure or rejection.
    /// Each pair must have equal exact half-spans; later pairs may use another step size.
    pub fn with_sample(
        mut self,
        clock: TickClock,
        sample: BalanceSample,
    ) -> Result<Self, SolverError> {
        if self.samples == self.maximum_samples {
            return Err(SolverError::ResourceLimit);
        }
        let previous = self.clock();
        if clock.target() != previous.target()
            || clock.exponent() != previous.exponent()
            || clock.elapsed() <= previous.elapsed()
        {
            return Err(SolverError::InvalidClock);
        }
        validate(sample)?;
        if self.pending {
            let integral = simpson(
                [self.nodes[0].0, self.nodes[1].0, clock],
                [self.nodes[0].1, self.nodes[1].1, sample],
            )?;
            self.sums[0].add(integral.energy_rhs)?;
            self.sums[1].add(integral.enstrophy_rhs)?;
            self.nodes[0] = (clock, sample);
        } else {
            self.nodes[1] = (clock, sample);
        }
        self.pending = !self.pending;
        self.samples += 1;
        if !self.pending {
            self.integral()?;
        }
        Ok(self)
    }
    /// Integrals and defects through the last complete pair; pending samples are never dropped.
    pub fn integral(self) -> Result<BalanceIntegral, SolverError> {
        if self.pending {
            return Err(SolverError::InvalidPayload);
        }
        let energy_rhs = self.sums[0].finish()?;
        let enstrophy_rhs = self.sums[1].finish()?;
        Ok(BalanceIntegral {
            energy_rhs,
            enstrophy_rhs,
            energy_defect: finite(self.nodes[0].1.energy - self.initial[0] - energy_rhs)?,
            enstrophy_defect: finite(self.nodes[0].1.enstrophy - self.initial[1] - enstrophy_rhs)?,
        })
    }
}

fn validate(sample: BalanceSample) -> Result<(), SolverError> {
    let positive = [
        sample.norms.l2,
        sample.norms.h1,
        sample.norms.vorticity_l2,
        sample.norms.divergence_l2,
        sample.energy,
        sample.enstrophy,
        sample.energy_dissipation,
        sample.enstrophy_dissipation,
    ];
    if positive
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        || [
            sample.forcing_work,
            sample.stretching,
            sample.vorticity_forcing,
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}
