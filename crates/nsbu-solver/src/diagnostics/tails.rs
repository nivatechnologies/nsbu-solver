//! Directional retained-spectrum tails with explicit integer cutoffs and no filtering.
use super::norms::{NormSums, Norms};
use crate::{
    domain::{validate_spectrum, Domain},
    Complex64, SolverError,
};

/// Three measured Fourier tails. The subsets overlap and must not be summed as a partition.
#[derive(Debug, Clone, Copy)]
pub struct DirectionalTails {
    /// Inclusive absolute-mode thresholds for the three Cartesian directions.
    pub cutoffs: [usize; 3],
    /// Physical volume-average norms for each directional subset, including velocity and curl.
    pub norms: [Norms; 3],
}

/// Allocation-free tail measurement over complete borrowed spectra.
#[derive(Debug, Clone, Copy)]
pub struct TailPlan {
    domain: Domain,
    cutoffs: [usize; 3],
}
impl TailPlan {
    /// Require nonzero cutoffs strictly inside each retained band, excluding vacuous Nyquist tails.
    pub fn new(domain: Domain, cutoffs: [usize; 3]) -> Result<Self, SolverError> {
        if cutoffs
            .into_iter()
            .zip(domain.layout().dimensions())
            .any(|(cutoff, n)| cutoff == 0 || cutoff >= n / 2)
        {
            return Err(SolverError::InvalidIndex);
        }
        Ok(Self { domain, cutoffs })
    }

    /// Measure every stored coefficient with correct half-spectrum multiplicity.
    /// No state filtering, mean subtraction, alignment or source modification is performed.
    pub fn measure(self, input: [&[Complex64]; 3]) -> Result<DirectionalTails, SolverError> {
        let layout = self.domain.layout();
        for values in input {
            validate_spectrum(layout, values, 1e-12)?;
        }
        let mut sums: [NormSums; 3] = std::array::from_fn(|_| NormSums::default());
        super::modes::visit(self.domain, &mut |mode| {
            let value = input.map(|field| field[mode.index]);
            for (axis, sum) in sums.iter_mut().enumerate() {
                if mode.integer[axis].unsigned_abs() >= self.cutoffs[axis] {
                    sum.push(mode.wave, value, mode.weight)?;
                }
            }
            Ok(())
        })?;
        let [x, y, z] = sums;
        Ok(DirectionalTails {
            cutoffs: self.cutoffs,
            norms: [x.finish()?, y.finish()?, z.finish()?],
        })
    }
}
