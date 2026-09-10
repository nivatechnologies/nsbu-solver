//! Complete-band scalar values and spatial derivatives on a separately admitted sample grid.
use super::squares::finite;
use crate::{
    domain::{validate_spectrum, Domain, Layout},
    spectral::{modal, FftPlan, FftWorkspace},
    storage::filled,
    Complex64, SolverError,
};

/// A Cartesian partial derivative of total order at most two, including the value itself.
/// Mixed derivatives commute mathematically; one canonical multi-index identifies them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Derivative([u8; 3]);
impl Derivative {
    /// Reject unsupported orders before any field traversal or allocation.
    pub fn new(orders: [u8; 3]) -> Result<Self, SolverError> {
        if orders.into_iter().map(u16::from).sum::<u16>() > 2 {
            return Err(SolverError::InvalidPayload);
        }
        Ok(Self(orders))
    }
    /// Canonical x/y/z derivative counts. `[0,0,0]` samples the scalar itself.
    pub fn orders(self) -> [u8; 3] {
        self.0
    }
    fn apply(self, wave: [f64; 3], mut value: Complex64) -> Result<Complex64, SolverError> {
        // At most two products. Applying one derivative at a time avoids forming k^2
        // before multiplying a potentially small coefficient.
        for (frequency, count) in wave.into_iter().zip(self.0) {
            for _ in 0..count {
                value *= Complex64::new(0.0, frequency);
            }
        }
        finite(value.re)?;
        finite(value.im)?;
        Ok(value)
    }
}

/// Read-only physical samples of one complete scalar spectrum and declared derivative.
/// These values do not establish a spatial supremum, region integral or trajectory origin.
#[derive(Debug)]
pub struct ScalarSamples<'a> {
    /// Scalar values in x/y/z lexicographic order, z varying fastest.
    pub values: &'a [f64],
    /// Actual diagnostic grid, which may differ from the retained source grid.
    pub layout: Layout,
    /// Unchanged physical domain lengths.
    pub lengths: [f64; 3],
    /// Applied Cartesian partial derivative.
    pub derivative: Derivative,
}
impl ScalarSamples<'_> {
    /// Unaligned physical coordinate in `[0,L)` for one actual sample.
    pub fn point(&self, index: usize) -> Result<[f64; 3], SolverError> {
        if index >= self.layout.real_len() {
            return Err(SolverError::InvalidPayload);
        }
        let [nx, ny, nz] = self.layout.dimensions();
        let position = [index / (ny * nz), (index / nz) % ny, index % nz];
        let counts = [nx, ny, nz];
        Ok(std::array::from_fn(|axis| {
            self.lengths[axis] * (position[axis] as f64 / counts[axis] as f64)
        }))
    }
}

/// Reusable scalar diagnostic scratch; caller spectra and integrated states stay borrowed.
/// Sample velocity components, gradient/Hessian entries or mean-zero physical pressure
/// sequentially, without retaining a separate physical grid for every tensor component.
#[derive(Debug)]
pub struct DerivativeWorkspace {
    source: Domain,
    samples: Layout,
    fft: FftPlan,
    transform: FftWorkspace,
    staging: Vec<Complex64>,
    values: Vec<f64>,
}
impl DerivativeWorkspace {
    // Only the complete comparison layer rebinds a source, after validating its
    // unchanged physical geometry, full sample-grid admission and stored reservation.
    pub(crate) fn bind_source(&mut self, source: Domain) {
        self.source = source;
    }
    /// Element/header reservation, excluding caller storage and allocator overhead.
    /// A sample grid must retain every source mode componentwise, including high modes.
    pub fn reservation(source: Domain, samples: Layout) -> Result<usize, SolverError> {
        if source
            .layout()
            .dimensions()
            .into_iter()
            .zip(samples.dimensions())
            .any(|(a, b)| a > b)
        {
            return Err(SolverError::InvalidDomain);
        }
        let real = samples.real_len().checked_mul(8);
        let complex = samples.half_len().checked_mul(16);
        let buffers = real
            .and_then(|a| complex.and_then(|b| a.checked_add(b)))
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)?;
        FftPlan::reservation(samples)?
            .checked_add(buffers)
            .ok_or(SolverError::SizeOverflow)
    }
    /// Admit the complete owned workspace before allocating any arrays.
    pub fn new(source: Domain, samples: Layout, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(source, samples)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let (fft, transform) = FftPlan::new(samples, cap)?;
        Ok(Self {
            source,
            samples,
            fft,
            transform,
            staging: filled(samples.half_len(), Complex64::new(0.0, 0.0))?,
            values: filled(samples.real_len(), 0.0)?,
        })
    }
    /// Differentiate every strict-band mode with physical `ik`, zero-pad and apply one
    /// inverse scalar FFT. No allocation, alignment, projection or mean subtraction occurs.
    /// Input must obey the committed-spectrum contract. Failure returns no sample view;
    /// scratch may change, while input bytes remain untouched and the workspace is reusable.
    pub fn sample(
        &mut self,
        input: &[Complex64],
        derivative: Derivative,
    ) -> Result<ScalarSamples<'_>, SolverError> {
        let source = self.source.layout();
        validate_spectrum(source, input, 1e-12)?;
        self.staging.fill(Complex64::new(0.0, 0.0));
        for (index, &coefficient) in input.iter().enumerate() {
            let position = source.position(index)?;
            if source.is_nyquist(position)? {
                continue;
            }
            let mode = source.mode(position)?;
            let wave = modal::wavevector(self.source, mode)?;
            let (target, _) = self.samples.locate(mode)?;
            self.staging[target] = derivative.apply(wave, coefficient)?;
        }
        self.fft
            .inverse(&self.staging, &mut self.values, &mut self.transform)?;
        Ok(ScalarSamples {
            values: &self.values,
            layout: self.samples,
            lengths: self.source.lengths(),
            derivative,
        })
    }
}
