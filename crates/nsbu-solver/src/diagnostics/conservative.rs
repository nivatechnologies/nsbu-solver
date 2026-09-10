//! Independent conservative products on a separately reserved double grid.
use crate::{
    domain::{validate_spectrum, Domain},
    spectral::{modal, transfer_validated, FftPlan, FftWorkspace},
    storage::filled,
    Complex64, SolverError,
};

/// Scratch for all six distinct products in div(u tensor u), retaining their full band.
/// No integrated state is owned or changed. Force must be sampled separately on the output grid.
#[derive(Debug)]
pub struct ConservativeWorkspace {
    source: Domain,
    diagnostic: Domain,
    fft: FftPlan,
    transform: FftWorkspace,
    velocity: [Vec<f64>; 3],
    product: Vec<f64>,
    staging: Vec<Complex64>,
    convection: [Vec<Complex64>; 3],
}

impl ConservativeWorkspace {
    /// Double each axis with checked sizes; diagnostic FFT admission is checked separately.
    pub fn diagnostic_domain(source: Domain) -> Result<Domain, SolverError> {
        let mut dimensions = source.layout().dimensions();
        for dimension in &mut dimensions {
            *dimension = dimension.checked_mul(2).ok_or(SolverError::SizeOverflow)?;
        }
        Domain::new(dimensions, source.lengths(), source.viscosity())
    }

    /// Element storage and headers, excluding allocator overhead and caller input/output buffers.
    pub fn reservation(source: Domain) -> Result<usize, SolverError> {
        let layout = Self::diagnostic_domain(source)?.layout();
        let real = layout
            .real_len()
            .checked_mul(4 * 8)
            .ok_or(SolverError::SizeOverflow)?;
        let complex = layout
            .half_len()
            .checked_mul(4 * 16)
            .ok_or(SolverError::SizeOverflow)?;
        [real, complex, std::mem::size_of::<Self>()]
            .into_iter()
            .try_fold(FftPlan::reservation(layout)?, |total, bytes| {
                total.checked_add(bytes).ok_or(SolverError::SizeOverflow)
            })
    }

    /// Allocate all operator storage after its complete reservation fits the cap.
    pub fn new(source: Domain, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(source)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = Self::diagnostic_domain(source)?;
        let layout = diagnostic.layout();
        let (fft, transform) = FftPlan::new(layout, cap)?;
        let real = layout.real_len();
        let half = layout.half_len();
        let zero = Complex64::new(0.0, 0.0);
        Ok(Self {
            source,
            diagnostic,
            fft,
            transform,
            velocity: [filled(real, 0.0)?, filled(real, 0.0)?, filled(real, 0.0)?],
            product: filled(real, 0.0)?,
            staging: filled(half, zero)?,
            convection: [
                filled(half, zero)?,
                filled(half, zero)?,
                filled(half, zero)?,
            ],
        })
    }

    /// Evaluate P(div(u tensor u) - f) and physical mean-zero pressure on the full double grid.
    /// Uses nine scalar transforms. Errors invalidate output scratch. This is not a residual
    /// until a separately reconstructed time derivative and viscous term have been added.
    pub fn evaluate(
        &mut self,
        velocity: [&[Complex64]; 3],
        force: [&[Complex64]; 3],
        output: [&mut [Complex64]; 3],
        pressure: &mut [Complex64],
    ) -> Result<(), SolverError> {
        for values in velocity {
            validate_spectrum(self.source.layout(), values, 1e-12)?;
        }
        for values in force {
            validate_spectrum(self.diagnostic.layout(), values, 1e-12)?;
        }
        let half = self.diagnostic.layout().half_len();
        if output.iter().any(|v| v.len() != half) || pressure.len() != half {
            return Err(SolverError::InvalidPayload);
        }
        for (axis, values) in velocity.into_iter().enumerate() {
            transfer_validated(
                self.source.layout(),
                self.diagnostic.layout(),
                values,
                &mut self.staging,
            );
            self.fft
                .inverse(&self.staging, &mut self.velocity[axis], &mut self.transform)?;
            self.convection[axis].fill(Complex64::new(0.0, 0.0));
        }
        for row in 0..3 {
            for column in row..3 {
                self.tensor_product(row, column)?;
            }
        }
        self.finish(force, output, pressure)
    }

    fn tensor_product(&mut self, row: usize, column: usize) -> Result<(), SolverError> {
        for (index, value) in self.product.iter_mut().enumerate() {
            *value = self.velocity[row][index] * self.velocity[column][index];
        }
        self.fft
            .forward(&self.product, &mut self.staging, &mut self.transform)?;
        let layout = self.diagnostic.layout();
        for (index, &product) in self.staging.iter().enumerate() {
            let position = layout.position(index)?;
            if layout.is_nyquist(position)? {
                continue;
            }
            let k = modal::wavevector(self.diagnostic, layout.mode(position)?)?;
            self.convection[row][index] += Complex64::i() * k[column] * product;
            if row != column {
                self.convection[column][index] += Complex64::i() * k[row] * product;
            }
        }
        Ok(())
    }

    fn finish(
        &self,
        force: [&[Complex64]; 3],
        output: [&mut [Complex64]; 3],
        pressure: &mut [Complex64],
    ) -> Result<(), SolverError> {
        let layout = self.diagnostic.layout();
        for (index, p) in pressure.iter_mut().enumerate() {
            let position = layout.position(index)?;
            let vector = if layout.is_nyquist(position)? {
                *p = Complex64::new(0.0, 0.0);
                [*p; 3]
            } else {
                let k = modal::wavevector(self.diagnostic, layout.mode(position)?)?;
                let raw =
                    std::array::from_fn(|axis| self.convection[axis][index] - force[axis][index]);
                *p = poisson_pressure(k, raw)?;
                modal::project(k, raw)?
            };
            for (axis, &value) in vector.iter().enumerate() {
                output[axis][index] = value;
            }
        }
        Ok(())
    }
}

fn poisson_pressure(
    k: [f64; 3],
    convection_minus_force: [Complex64; 3],
) -> Result<Complex64, SolverError> {
    let scale = k.into_iter().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return Ok(Complex64::new(0.0, 0.0));
    }
    let direction = k.map(|value| value / scale);
    let squared = direction
        .into_iter()
        .map(|value| value * value)
        .sum::<f64>();
    let divergence = direction
        .into_iter()
        .zip(convection_minus_force)
        .map(|(a, b)| a * b)
        .sum::<Complex64>();
    let pressure = Complex64::i() * (divergence / squared) / scale;
    super::squares::finite(pressure.re)?;
    super::squares::finite(pressure.im)?;
    Ok(pressure)
}
