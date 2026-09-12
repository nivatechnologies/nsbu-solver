//! Explicit W3 reservation, construction, and execution identity.
use super::SpectralRhs;
use crate::{
    domain::Domain,
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::{FftBackend, FftCatalog, RotationalWorkspace, W3FftIdentity},
    SolverError,
};

impl<F: PrescribedForce> SpectralRhs<F> {
    /// Opt-in reservation for an independently owned W3 rotational operator.
    pub fn reservation_with_w3_fft_backend(
        domain: Domain,
        limits: ForceLimits,
        backend: FftBackend,
    ) -> Result<usize, SolverError> {
        Self::validate_limits(limits)?;
        let operator = RotationalWorkspace::reservation_with_w3_fft_backend(domain, backend)?;
        Self::reservation_parts(domain, limits, operator)
    }

    /// Construct only the explicit W3 rotational path; serial constructors are unchanged.
    pub fn new_with_catalog_w3(
        domain: Domain,
        force: F,
        advective_limit: f64,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = force.limits().ok_or(SolverError::UnknownProviderCost)?;
        let storage_bytes =
            Self::reservation_with_w3_fft_backend(domain, limits, catalog.backend())?;
        if storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        if !advective_limit.is_finite() || advective_limit <= 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let operator = RotationalWorkspace::new_with_catalog_w3(domain, catalog, cap)?;
        Self::allocate(
            domain,
            force,
            limits,
            advective_limit,
            storage_bytes,
            operator,
        )
    }

    /// Experimental W3 operator identity; absent for every existing constructor.
    pub fn w3_fft_identity(&self) -> Option<W3FftIdentity> {
        self.operator.w3_fft_identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::TickClock, integrators::forcing::ForceWork, Complex64};

    struct Fixture(Option<ForceLimits>);

    impl PrescribedForce for Fixture {
        fn limits(&self) -> Option<ForceLimits> {
            self.0
        }

        fn evaluate(
            &mut self,
            _time: TickClock,
            _limit: ForceLimits,
            _output: [&mut [Complex64]; 3],
        ) -> Result<ForceWork, SolverError> {
            Ok(ForceWork {
                work_units: 0,
                scalar_transforms: 0,
            })
        }
    }

    #[test]
    fn explicit_w3_rhs_admission_reports_identity_and_rejects_invalid_inputs() {
        let backend = FftBackend::RustFft6_4_1AvxFma;
        if backend.ensure_available().is_err() {
            return;
        }
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let catalog_bytes = FftCatalog::reservation(backend).unwrap();
        let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
        let limits = ForceLimits {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
            remaining_divisor: 1,
        };
        let bytes =
            SpectralRhs::<Fixture>::reservation_with_w3_fft_backend(domain, limits, backend)
                .unwrap();
        assert!(matches!(
            SpectralRhs::new_with_catalog_w3(domain, Fixture(None), 0.3, &catalog, bytes),
            Err(SolverError::UnknownProviderCost)
        ));
        assert!(matches!(
            SpectralRhs::new_with_catalog_w3(
                domain,
                Fixture(Some(limits)),
                0.3,
                &catalog,
                bytes - 1,
            ),
            Err(SolverError::ResourceLimit)
        ));
        assert!(matches!(
            SpectralRhs::new_with_catalog_w3(
                domain,
                Fixture(Some(limits)),
                f64::NAN,
                &catalog,
                bytes,
            ),
            Err(SolverError::InvalidStep)
        ));
        let rhs =
            SpectralRhs::new_with_catalog_w3(domain, Fixture(Some(limits)), 0.3, &catalog, bytes)
                .unwrap();
        let identity = rhs.w3_fft_identity().unwrap();
        assert_eq!(identity.backend, backend);
        assert_eq!(identity.layout, domain.padded_layout().unwrap());
    }
}
