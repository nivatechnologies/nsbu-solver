//! Full-double-band assembly of reconstructed physical-time PDE defects.
mod accumulation;
mod localized;

use super::{
    conservative::ConservativeWorkspace,
    norms::{NormSums, Norms},
};
use crate::{
    domain::{validate_spectrum, Domain},
    spectral::modal,
    Complex64, SolverError,
};
pub use localized::{
    CancellationChannels, NormChannels, ResidualBandLocalization, ResidualLocalization,
};

/// Geometry for borrowed residual assembly; owns no storage and never alters a trajectory.
#[derive(Debug, Clone, Copy)]
pub struct ResidualPlan {
    source: Domain,
    diagnostic: Domain,
}

impl ResidualPlan {
    /// Admit the retained and doubled geometries. FFT workspace admission is separate.
    pub fn new(source: Domain) -> Result<Self, SolverError> {
        Ok(Self {
            source,
            diagnostic: ConservativeWorkspace::diagnostic_domain(source)?,
        })
    }

    /// Assemble v_t + P div(v tensor v) - nu Delta v - P f on the complete double grid.
    /// `conservative` must be independently evaluated at the same probe and represents
    /// P(div(v tensor v) - f). Returned norms are samples, not time-supremum bounds.
    /// Errors invalidate output scratch; no reference field is substituted into an integrated state.
    pub fn evaluate(
        self,
        velocity: [&[Complex64]; 3],
        derivative: [&[Complex64]; 3],
        conservative: [&[Complex64]; 3],
        output: [&mut [Complex64]; 3],
    ) -> Result<Norms, SolverError> {
        for values in velocity.into_iter().chain(derivative) {
            validate_spectrum(self.source.layout(), values, 1e-12)?;
        }
        for values in conservative {
            validate_spectrum(self.diagnostic.layout(), values, 1e-12)?;
        }
        let layout = self.diagnostic.layout();
        if output
            .iter()
            .any(|values| values.len() != layout.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let mut norms = NormSums::default();
        let [a, b, c] = output;
        for (index, ((a, b), c)) in a.iter_mut().zip(b).zip(c).enumerate() {
            let position = layout.position(index)?;
            if layout.is_nyquist(position)? {
                *a = Complex64::new(0.0, 0.0);
                *b = *a;
                *c = *a;
                continue;
            }
            let mode = layout.mode(position)?;
            let k = modal::wavevector(self.diagnostic, mode)?;
            let mut value = std::array::from_fn(|axis| conservative[axis][index]);
            if let Ok((source_index, _)) = self.source.layout().locate(mode) {
                for (axis, value) in value.iter_mut().enumerate() {
                    *value += derivative[axis][source_index];
                    for wave in k {
                        *value +=
                            self.source.viscosity() * wave * (wave * velocity[axis][source_index]);
                    }
                }
            }
            norms.push(k, value, layout.weight(position)?)?;
            [*a, *b, *c] = value;
        }
        norms.finish()
    }
}
