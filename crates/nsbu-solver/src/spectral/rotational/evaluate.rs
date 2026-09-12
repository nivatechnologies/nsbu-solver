//! Rotational evaluation, component transforms, projection, and pressure.
use super::{RotationalWorkspace, TransformOwner};
use crate::{
    domain::{validate_spectrum, Layout},
    spectral::{modal, transfer_validated},
    Complex64, SolverError,
};

impl RotationalWorkspace {
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
        validate_inputs(layout, velocity, force, &output, pressure)?;
        self.spatial_fields(velocity)?;
        self.cross_product();
        self.transform_acceleration(force)?;
        self.project_and_pressure(&mut output, pressure)?;
        self.physical_pressure(pressure)
    }

    fn transform_acceleration(&mut self, force: [&[Complex64]; 3]) -> Result<(), SolverError> {
        let layout = self.domain.layout();
        match &mut self.transform {
            TransformOwner::Serial {
                fft,
                workspace,
                staging,
            } => {
                for (axis, source) in force.iter().enumerate() {
                    fft.forward(&self.vorticity[axis], staging, workspace)?;
                    transfer_validated(self.padded, layout, staging, &mut self.acceleration[axis]);
                    add_force(&mut self.acceleration[axis], source);
                }
            }
            TransformOwner::W3(pool) => {
                pool.forward3(&mut self.vorticity)?;
                for (axis, source) in force.iter().enumerate() {
                    pool.with_spectrum(axis, |staging| {
                        transfer_validated(
                            self.padded,
                            layout,
                            staging,
                            &mut self.acceleration[axis],
                        );
                    })?;
                    add_force(&mut self.acceleration[axis], source);
                }
            }
        }
        Ok(())
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
        match &mut self.transform {
            TransformOwner::Serial {
                fft,
                workspace,
                staging,
            } => {
                fft.forward(&self.vorticity[0], staging, workspace)?;
                transfer_validated(self.padded, self.domain.layout(), staging, &mut self.energy);
            }
            TransformOwner::W3(pool) => {
                pool.forward_one(&self.vorticity[0])?;
                pool.with_spectrum(0, |staging| {
                    transfer_validated(
                        self.padded,
                        self.domain.layout(),
                        staging,
                        &mut self.energy,
                    );
                })?;
            }
        }
        for (value, energy) in pressure.iter_mut().zip(&self.energy) {
            *value -= energy;
        }
        pressure[0] = Complex64::new(0.0, 0.0);
        crate::spectral::hermitian::finite(pressure)
    }
}

fn validate_inputs(
    layout: Layout,
    velocity: [&[Complex64]; 3],
    force: [&[Complex64]; 3],
    output: &[&mut [Complex64]; 3],
    pressure: &[Complex64],
) -> Result<(), SolverError> {
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
    Ok(())
}

fn add_force(acceleration: &mut [Complex64], force: &[Complex64]) {
    for (value, &forcing) in acceleration.iter_mut().zip(force) {
        *value += forcing;
    }
}
