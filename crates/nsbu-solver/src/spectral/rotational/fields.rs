//! Curl preparation, inverse component transforms, and cross product.
use super::{RotationalWorkspace, TransformOwner};
use crate::{
    spectral::{modal, transfer_validated},
    Complex64, SolverError,
};

impl RotationalWorkspace {
    pub(super) fn spatial_fields(
        &mut self,
        velocity: [&[Complex64]; 3],
    ) -> Result<(), SolverError> {
        let layout = self.domain.layout();
        self.fill_curl(velocity)?;
        match &mut self.transform {
            TransformOwner::Serial {
                fft,
                workspace,
                staging,
            } => {
                for (axis, component) in velocity.into_iter().enumerate() {
                    transfer_validated(layout, self.padded, component, staging);
                    fft.inverse(staging, &mut self.velocity[axis], workspace)?;
                    transfer_validated(layout, self.padded, &self.curl[axis], staging);
                    fft.inverse(staging, &mut self.vorticity[axis], workspace)?;
                }
            }
            TransformOwner::W3(pool) => {
                for (axis, component) in velocity.into_iter().enumerate() {
                    pool.prepare_inverse(axis, |staging| {
                        transfer_validated(layout, self.padded, component, staging);
                    })?;
                }
                pool.inverse3(&mut self.velocity)?;
                for axis in 0..3 {
                    pool.prepare_inverse(axis, |staging| {
                        transfer_validated(layout, self.padded, &self.curl[axis], staging);
                    })?;
                }
                pool.inverse3(&mut self.vorticity)?;
            }
        }
        Ok(())
    }

    fn fill_curl(&mut self, velocity: [&[Complex64]; 3]) -> Result<(), SolverError> {
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
        Ok(())
    }

    pub(super) fn cross_product(&mut self) {
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
}
