//! Full-fine-band primary comparisons without alignment, recentering or missing-mode suppression.
use super::norms::{NormSums, Norms};
use crate::{
    domain::{validate_spectrum, Domain},
    spectral::modal,
    Complex64, SolverError,
};

/// Empirical field differences split into orthogonal retained-mode subsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandComparison {
    /// Primary full-fine-band error, including all newly resolved modes.
    pub full: Norms,
    /// Error restricted to the strict coarse retained band.
    pub common: Norms,
    /// Fine coefficients outside the strict coarse band, compared with exact zero.
    pub newly_resolved: Norms,
    /// Difference of physical mean velocity components; no mean is removed.
    pub mean_error: [f64; 3],
}

/// Geometry/work admission for a comparison over existing borrowed spectra; owns no heap storage.
#[derive(Debug, Clone, Copy)]
pub struct ComparisonPlan {
    coarse: Domain,
    fine: Domain,
}
impl ComparisonPlan {
    /// Require identical physical domains/viscosity and a componentwise finer or equal grid.
    pub fn new(coarse: Domain, fine: Domain) -> Result<Self, SolverError> {
        if coarse.lengths() != fine.lengths()
            || coarse.viscosity() != fine.viscosity()
            || coarse
                .layout()
                .dimensions()
                .into_iter()
                .zip(fine.layout().dimensions())
                .any(|(a, b)| a > b)
        {
            return Err(SolverError::InvalidDomain);
        }
        Ok(Self { coarse, fine })
    }
    /// Component validation visits plus fixed-cost vector coefficient visits, not primitive FLOPs.
    pub fn work_units(self) -> usize {
        // Layout bounds each half length by isize::MAX/16, so this sum fits usize.
        3 * (self.coarse.layout().half_len() + self.fine.layout().half_len())
            + self.fine.layout().half_len()
    }
    /// Compare complete strict spectra. Input validation follows the committed-state contract.
    pub fn compare(
        self,
        coarse: [&[Complex64]; 3],
        fine: [&[Complex64]; 3],
    ) -> Result<BandComparison, SolverError> {
        for values in coarse {
            validate_spectrum(self.coarse.layout(), values, 1e-12)?;
        }
        for values in fine {
            validate_spectrum(self.fine.layout(), values, 1e-12)?;
        }
        let mut common = NormSums::default();
        let mut newly_resolved = NormSums::default();
        let layout = self.fine.layout();
        for (index, _) in fine[0].iter().enumerate() {
            let position = layout.position(index)?;
            if layout.is_nyquist(position)? {
                continue;
            }
            let mode = layout.mode(position)?;
            // Stored fine modes have nonnegative third coordinate, so a successful
            // coarse lookup uses that same half plane and never conjugates.
            let earlier = self
                .coarse
                .layout()
                .locate(mode)
                .ok()
                .map(|(index, _)| index);
            let difference = std::array::from_fn(|axis| {
                fine[axis][index] - earlier.map_or(Complex64::new(0.0, 0.0), |i| coarse[axis][i])
            });
            let sums = if earlier.is_some() {
                &mut common
            } else {
                &mut newly_resolved
            };
            sums.push(
                modal::wavevector(self.fine, mode)?,
                difference,
                layout.weight(position)?,
            )?;
        }
        let common = common.finish()?;
        let newly_resolved = newly_resolved.finish()?;
        Ok(BandComparison {
            full: common.orthogonal_sum(newly_resolved)?,
            common,
            newly_resolved,
            mean_error: std::array::from_fn(|axis| fine[axis][0].re - coarse[axis][0].re),
        })
    }
}
