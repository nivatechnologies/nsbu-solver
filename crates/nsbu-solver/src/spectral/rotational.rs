//! Three-halves padded rotational nonlinearity and physical-pressure reconstruction.
use super::transfer::transfer_validated;
use super::{
    modal, FftBackend, FftCatalog, FftPlan, FftWorkspace, W3FftIdentity, W3FftMode, W3FftPool,
};
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
    transform: TransformOwner,
    velocity: [Vec<f64>; 3],
    vorticity: [Vec<f64>; 3],
    curl: [Vec<Complex64>; 3],
    acceleration: [Vec<Complex64>; 3],
    energy: Vec<Complex64>,
}

enum TransformOwner {
    Serial {
        fft: FftPlan,
        workspace: FftWorkspace,
        staging: Vec<Complex64>,
    },
    W3(W3FftPool),
}

impl std::fmt::Debug for TransformOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serial { .. } => formatter.write_str("SerialFftOwner"),
            Self::W3(pool) => formatter
                .debug_tuple("W3FftOwner")
                .field(&pool.identity())
                .finish(),
        }
    }
}

impl RotationalWorkspace {
    /// Exact element storage and object-header reservation; caller adds allocator overhead.
    pub fn reservation(domain: Domain) -> Result<usize, SolverError> {
        Self::reservation_inner(domain, None)
    }

    /// Reservation when immutable FFT plans are shared by an execution-owned catalog.
    pub fn reservation_with_catalog(
        domain: Domain,
        catalog: &FftCatalog,
    ) -> Result<usize, SolverError> {
        Self::reservation_inner(domain, Some(catalog))
    }

    /// Workspace-only reservation for an enclosing execution-owned backend catalog.
    pub fn reservation_with_fft_backend(
        domain: Domain,
        backend: FftBackend,
    ) -> Result<usize, SolverError> {
        let padded = domain.padded_layout()?;
        let fft = FftPlan::reservation_with_shared_backend(padded, backend)?;
        Self::reservation_parts(domain, padded, fft)
    }

    /// Opt-in reservation for a separately owned bidirectional W3 operator pool.
    pub fn reservation_with_w3_fft_backend(
        domain: Domain,
        backend: FftBackend,
    ) -> Result<usize, SolverError> {
        let base = Self::reservation_with_fft_backend(domain, backend)?;
        let additional = W3FftPool::additional_reservation_with_backend(
            domain.padded_layout()?,
            backend,
            W3FftMode::Bidirectional,
        )?;
        base.checked_add(additional)
            .ok_or(SolverError::SizeOverflow)
    }

    fn reservation_inner(
        domain: Domain,
        catalog: Option<&FftCatalog>,
    ) -> Result<usize, SolverError> {
        let padded = domain.padded_layout()?;
        let fft = match catalog {
            Some(catalog) => FftPlan::reservation_from_catalog(padded, catalog)?,
            None => FftPlan::reservation(padded)?,
        };
        Self::reservation_parts(domain, padded, fft)
    }

    fn reservation_parts(domain: Domain, padded: Layout, fft: usize) -> Result<usize, SolverError> {
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
        Self::allocate(domain, padded, fft, transform)
    }

    /// Allocate mutable operator storage while sharing an admitted immutable FFT catalog.
    pub fn new_with_catalog(
        domain: Domain,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let bytes = Self::reservation_with_catalog(domain, catalog)?;
        if bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let padded = domain.padded_layout()?;
        let (fft, transform) = FftPlan::new_from_catalog(padded, catalog, cap)?;
        Self::allocate(domain, padded, fft, transform)
    }

    /// Construct the explicit experimental W3 operator without changing serial defaults.
    pub fn new_with_catalog_w3(
        domain: Domain,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let total = Self::reservation_with_w3_fft_backend(domain, catalog.backend())?;
        if total > cap {
            return Err(SolverError::ResourceLimit);
        }
        let base = Self::reservation_with_catalog(domain, catalog)?;
        let mut owner = Self::new_with_catalog(domain, catalog, base)?;
        let additional = total.checked_sub(base).ok_or(SolverError::SizeOverflow)?;
        let TransformOwner::Serial {
            fft,
            workspace,
            staging,
        } = owner.transform
        else {
            return Err(SolverError::InvalidPayload);
        };
        owner.transform = TransformOwner::W3(W3FftPool::from_scalar_lane(
            owner.padded,
            catalog,
            W3FftMode::Bidirectional,
            (fft, workspace, staging),
            additional,
        )?);
        Ok(owner)
    }

    /// W3 execution identity, present only for the explicit opt-in constructor.
    pub fn w3_fft_identity(&self) -> Option<W3FftIdentity> {
        match &self.transform {
            TransformOwner::Serial { .. } => None,
            TransformOwner::W3(pool) => Some(pool.identity()),
        }
    }

    fn allocate(
        domain: Domain,
        padded: Layout,
        fft: FftPlan,
        transform: FftWorkspace,
    ) -> Result<Self, SolverError> {
        let real = padded.real_len();
        let h = domain.layout().half_len();
        let zero = Complex64::new(0.0, 0.0);
        Ok(Self {
            domain,
            padded,
            transform: TransformOwner::Serial {
                fft,
                workspace: transform,
                staging: filled(padded.half_len(), zero)?,
            },
            velocity: [filled(real, 0.0)?, filled(real, 0.0)?, filled(real, 0.0)?],
            vorticity: [filled(real, 0.0)?, filled(real, 0.0)?, filled(real, 0.0)?],
            curl: [filled(h, zero)?, filled(h, zero)?, filled(h, zero)?],
            acceleration: [filled(h, zero)?, filled(h, zero)?, filled(h, zero)?],
            energy: filled(h, zero)?,
        })
    }

    /// Advective grid measure for the fields from the last successful evaluation.
    /// Call only after evaluate succeeds; scratch is invalid after any evaluation error.
    pub fn advective_number(&self, duration: f64) -> Result<f64, SolverError> {
        if !duration.is_finite() || duration <= 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let dimensions = self.domain.layout().dimensions();
        let lengths = self.domain.lengths();
        let maximum_wave = std::array::from_fn::<_, 3, _>(|axis| {
            std::f64::consts::TAU * (dimensions[axis] / 2 - 1) as f64 / lengths[axis]
        });
        let mut maximum = 0.0_f64;
        for index in 0..self.padded.real_len() {
            let measure = (0..3)
                .map(|axis| self.velocity[axis][index].abs() * maximum_wave[axis])
                .sum::<f64>();
            maximum = maximum.max(measure);
        }
        let number = duration * maximum;
        if !number.is_finite() {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        Ok(number)
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
        super::hermitian::finite(pressure)
    }
}

fn add_force(acceleration: &mut [Complex64], force: &[Complex64]) {
    for (value, &forcing) in acceleration.iter_mut().zip(force) {
        *value += forcing;
    }
}

#[cfg(test)]
mod tests;
