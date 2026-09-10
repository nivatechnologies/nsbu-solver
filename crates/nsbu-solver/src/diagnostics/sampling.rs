//! Read-only oversampling of velocity and vorticity with explicitly sampled maxima.
use super::squares::finite;
use crate::{
    domain::{validate_spectrum, Domain, Layout},
    spectral::{modal, transfer_validated, FftPlan, FftWorkspace},
    storage::filled,
    Complex64, SolverError,
};

/// Maximum on a finite sampling grid, never a certified spatial supremum.
#[derive(Debug, Clone, Copy)]
pub struct SampledMaximum {
    /// Euclidean vector magnitude at the selected sample.
    pub value: f64,
    /// Unshifted physical coordinates in the periodic domain [0,L).
    pub position: [f64; 3],
    /// Lexicographically first grid index attaining the floating maximum.
    pub grid_index: [usize; 3],
    /// Number of samples with exactly equal binary64 magnitude, not mathematical multiplicity.
    pub ties: usize,
}

/// Successful immutable sample views borrow the workspace until dropped.
#[derive(Debug)]
pub struct SampledFields<'a> {
    /// Three physical velocity components in lexicographic sample order.
    pub velocity: [&'a [f64]; 3],
    /// Three physical vorticity components in the same sample order.
    pub vorticity: [&'a [f64]; 3],
    /// Actual sampling resolution, independent of the retained state grid.
    pub dimensions: [usize; 3],
    /// Largest sampled velocity magnitude and its unaligned location.
    pub velocity_maximum: SampledMaximum,
    /// Largest sampled vorticity magnitude and its unaligned location.
    pub vorticity_maximum: SampledMaximum,
    /// Six inverse scalar transforms, including the independently sampled curl.
    pub scalar_transforms: usize,
}

/// Separately reserved real-space diagnostics. No integrated state or force provider is owned.
#[derive(Debug)]
pub struct SamplingWorkspace {
    source: Domain,
    samples: Layout,
    fft: FftPlan,
    transform: FftWorkspace,
    staging: Vec<Complex64>,
    velocity: [Vec<f64>; 3],
    vorticity: [Vec<f64>; 3],
}
impl SamplingWorkspace {
    /// Complete element/header storage, excluding caller input and allocator overhead.
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
        let real = samples
            .real_len()
            .checked_mul(6 * 8)
            .ok_or(SolverError::SizeOverflow)?;
        let complex = samples
            .half_len()
            .checked_mul(16)
            .ok_or(SolverError::SizeOverflow)?;
        [real, complex, std::mem::size_of::<Self>()]
            .into_iter()
            .try_fold(FftPlan::reservation(samples)?, |total, bytes| {
                total.checked_add(bytes).ok_or(SolverError::SizeOverflow)
            })
    }

    /// Require a componentwise finer or equal sample grid and preflight before allocating.
    pub fn new(source: Domain, samples: Layout, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(source, samples)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let (fft, transform) = FftPlan::new(samples, cap)?;
        let n = samples.real_len();
        Ok(Self {
            source,
            samples,
            fft,
            transform,
            staging: filled(samples.half_len(), Complex64::new(0.0, 0.0))?,
            velocity: [filled(n, 0.0)?, filled(n, 0.0)?, filled(n, 0.0)?],
            vorticity: [filled(n, 0.0)?, filled(n, 0.0)?, filled(n, 0.0)?],
        })
    }

    /// Sample without allocation or phase alignment. An error returns no views of invalid scratch.
    /// Refining these maxima is an empirical check; it does not turn them into rigorous bounds.
    pub fn sample(&mut self, input: [&[Complex64]; 3]) -> Result<SampledFields<'_>, SolverError> {
        for component in input {
            validate_spectrum(self.source.layout(), component, 1e-12)?;
        }
        for (axis, component) in input.into_iter().enumerate() {
            transfer_validated(
                self.source.layout(),
                self.samples,
                component,
                &mut self.staging,
            );
            self.fft
                .inverse(&self.staging, &mut self.velocity[axis], &mut self.transform)?;
            self.curl_component(input, axis)?;
            self.fft.inverse(
                &self.staging,
                &mut self.vorticity[axis],
                &mut self.transform,
            )?;
        }
        let velocity = self.velocity.each_ref().map(Vec::as_slice);
        let vorticity = self.vorticity.each_ref().map(Vec::as_slice);
        Ok(SampledFields {
            velocity_maximum: maximum(self.source.lengths(), self.samples, velocity)?,
            vorticity_maximum: maximum(self.source.lengths(), self.samples, vorticity)?,
            velocity,
            vorticity,
            dimensions: self.samples.dimensions(),
            scalar_transforms: 6,
        })
    }

    fn curl_component(&mut self, input: [&[Complex64]; 3], axis: usize) -> Result<(), SolverError> {
        self.staging.fill(Complex64::new(0.0, 0.0));
        super::modes::visit(self.source, &mut |mode| {
            let curl = modal::curl(mode.wave, input.map(|field| field[mode.index]))?;
            let (destination, _) = self.samples.locate(mode.integer)?;
            self.staging[destination] = curl[axis];
            Ok(())
        })
    }
}

fn maximum(
    lengths: [f64; 3],
    layout: Layout,
    field: [&[f64]; 3],
) -> Result<SampledMaximum, SolverError> {
    let mut value = 0.0;
    let mut selected = 0;
    let mut ties = 0;
    for (index, &first) in field[0].iter().enumerate() {
        let magnitude = finite(first.hypot(field[1][index]).hypot(field[2][index]))?;
        if magnitude > value {
            value = magnitude;
            selected = index;
            ties = 1;
        } else if magnitude == value {
            ties += 1;
        }
    }
    let [nx, ny, nz] = layout.dimensions();
    let grid_index = [selected / (ny * nz), (selected / nz) % ny, selected % nz];
    let dimensions = [nx, ny, nz];
    Ok(SampledMaximum {
        value,
        ties,
        grid_index,
        position: std::array::from_fn(|axis| {
            (grid_index[axis] as f64 / dimensions[axis] as f64) * lengths[axis]
        }),
    })
}
