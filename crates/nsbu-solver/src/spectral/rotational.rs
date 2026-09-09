//! Three-halves padded rotational nonlinearity and physical-pressure reconstruction.
use super::transfer::transfer_validated;
use super::{modal, FftPlan, FftWorkspace};
use crate::{
    domain::{validate_spectrum, Domain, Layout},
    storage::filled,
    Complex64, SolverError,
};

/// Private scratch for one trajectory's rotational operator evaluations.
/// All vectors are allocated during construction, never during evaluation.
#[derive(Debug)]
pub struct RotationalWorkspace {
    domain: Domain,
    padded: Layout,
    fft: FftPlan,
    transform: FftWorkspace,
    velocity: [Vec<f64>; 3],
    vorticity: [Vec<f64>; 3],
    curl: [Vec<Complex64>; 3],
    acceleration: [Vec<Complex64>; 3],
    staging: Vec<Complex64>,
    energy: Vec<Complex64>,
}

impl RotationalWorkspace {
    /// Exact element storage and object-header reservation; caller adds allocator overhead.
    pub fn reservation(domain: Domain) -> Result<usize, SolverError> {
        let padded = domain.padded_layout()?;
        let fft = FftPlan::reservation(padded)?;
        // Six real arrays, seven retained complex arrays and one padded complex staging array.
        let real = padded
            .real_len()
            .checked_mul(6 * 8)
            .ok_or(SolverError::SizeOverflow)?;
        let retained = domain
            .layout()
            .half_len()
            .checked_mul(7 * 16)
            .ok_or(SolverError::SizeOverflow)?;
        let stage = padded
            .half_len()
            .checked_mul(16)
            .ok_or(SolverError::SizeOverflow)?;
        [real, retained, stage, std::mem::size_of::<Self>()]
            .iter()
            .try_fold(fft, |sum, &value| {
                sum.checked_add(value).ok_or(SolverError::SizeOverflow)
            })
    }

    /// Allocate only after the complete operator reservation fits the explicit cap.
    pub fn new(domain: Domain, cap: usize) -> Result<Self, SolverError> {
        let bytes = Self::reservation(domain)?;
        if bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let padded = domain.padded_layout()?;
        let (fft, transform) = FftPlan::new(padded, cap)?;
        let real = padded.real_len();
        let h = domain.layout().half_len();
        let zero = Complex64::new(0.0, 0.0);
        Ok(Self {
            domain,
            padded,
            fft,
            transform,
            velocity: [filled(real, 0.0)?, filled(real, 0.0)?, filled(real, 0.0)?],
            vorticity: [filled(real, 0.0)?, filled(real, 0.0)?, filled(real, 0.0)?],
            curl: [filled(h, zero)?, filled(h, zero)?, filled(h, zero)?],
            acceleration: [filled(h, zero)?, filled(h, zero)?, filled(h, zero)?],
            staging: filled(padded.half_len(), zero)?,
            energy: filled(h, zero)?,
        })
    }

    /// Compute P Fourier(u cross curl(u)) + P f, and physical mean-zero pressure.
    /// Force coefficients remain unprojected until pressure has been reconstructed.
    /// Errors invalidate output scratch; this method cannot mutate the input state or time.
    pub fn evaluate(
        &mut self,
        velocity: [&[Complex64]; 3],
        force: [&[Complex64]; 3],
        mut output: [&mut [Complex64]; 3],
        pressure: &mut [Complex64],
    ) -> Result<(), SolverError> {
        let layout = self.domain.layout();
        for values in velocity.into_iter().chain(force) {
            let scale = values
                .iter()
                .fold(1.0_f64, |s, v| s.max(v.re.abs()).max(v.im.abs()));
            validate_spectrum(layout, values, 64.0 * f64::EPSILON * scale)?;
        }
        if output
            .iter()
            .any(|values| values.len() != layout.half_len())
            || pressure.len() != layout.half_len()
        {
            return Err(SolverError::InvalidPayload);
        }
        self.spatial_fields(velocity)?;
        self.cross_product();
        for (axis, source) in force.iter().enumerate() {
            self.fft.forward(
                &self.vorticity[axis],
                &mut self.staging,
                &mut self.transform,
            )?;
            transfer_validated(
                self.padded,
                layout,
                &self.staging,
                &mut self.acceleration[axis],
            );
            for (value, &forcing) in self.acceleration[axis].iter_mut().zip(*source) {
                *value += forcing;
            }
        }
        self.project_and_pressure(&mut output, pressure)?;
        self.physical_pressure(pressure)
    }

    fn spatial_fields(&mut self, velocity: [&[Complex64]; 3]) -> Result<(), SolverError> {
        let layout = self.domain.layout();
        for (index, &u0) in velocity[0].iter().enumerate() {
            let position = layout.position(index)?;
            let value = if layout.is_nyquist(position)? {
                [Complex64::new(0.0, 0.0); 3]
            } else {
                let k = modal::wavevector(self.domain, layout.mode(position)?)?;
                modal::curl(k, [u0, velocity[1][index], velocity[2][index]])?
            };
            for (axis, &coefficient) in value.iter().enumerate() {
                self.curl[axis][index] = coefficient;
            }
        }
        for (axis, component) in velocity.into_iter().enumerate() {
            transfer_validated(layout, self.padded, component, &mut self.staging);
            self.fft
                .inverse(&self.staging, &mut self.velocity[axis], &mut self.transform)?;
            transfer_validated(layout, self.padded, &self.curl[axis], &mut self.staging);
            self.fft.inverse(
                &self.staging,
                &mut self.vorticity[axis],
                &mut self.transform,
            )?;
        }
        Ok(())
    }

    fn cross_product(&mut self) {
        // Curl storage becomes the product only after reading all three local components.
        for index in 0..self.padded.real_len() {
            let u = [
                self.velocity[0][index],
                self.velocity[1][index],
                self.velocity[2][index],
            ];
            let w = [
                self.vorticity[0][index],
                self.vorticity[1][index],
                self.vorticity[2][index],
            ];
            let product = [
                u[1] * w[2] - u[2] * w[1],
                u[2] * w[0] - u[0] * w[2],
                u[0] * w[1] - u[1] * w[0],
            ];
            for (axis, &value) in product.iter().enumerate() {
                self.vorticity[axis][index] = value;
            }
        }
    }

    fn project_and_pressure(
        &self,
        output: &mut [&mut [Complex64]; 3],
        pressure: &mut [Complex64],
    ) -> Result<(), SolverError> {
        let layout = self.domain.layout();
        for (index, p) in pressure.iter_mut().enumerate() {
            let position = layout.position(index)?;
            let value = if layout.is_nyquist(position)? {
                *p = Complex64::new(0.0, 0.0);
                [Complex64::new(0.0, 0.0); 3]
            } else {
                let k = modal::wavevector(self.domain, layout.mode(position)?)?;
                let raw = std::array::from_fn(|axis| self.acceleration[axis][index]);
                *p = modal::modified_pressure(k, raw)?;
                modal::project(k, raw)?
            };
            for (axis, &coefficient) in value.iter().enumerate() {
                output[axis][index] = coefficient;
            }
        }
        Ok(())
    }

    fn physical_pressure(&mut self, pressure: &mut [Complex64]) -> Result<(), SolverError> {
        for (index, value) in self.vorticity[0].iter_mut().enumerate() {
            *value = 0.5
                * (self.velocity[0][index].powi(2)
                    + self.velocity[1][index].powi(2)
                    + self.velocity[2][index].powi(2));
        }
        self.fft
            .forward(&self.vorticity[0], &mut self.staging, &mut self.transform)?;
        transfer_validated(
            self.padded,
            self.domain.layout(),
            &self.staging,
            &mut self.energy,
        );
        for (value, energy) in pressure.iter_mut().zip(&self.energy) {
            *value -= energy;
        }
        pressure[0] = Complex64::new(0.0, 0.0);
        super::hermitian::finite(pressure)
    }
}
