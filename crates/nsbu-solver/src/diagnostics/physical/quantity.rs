//! Exact component/derivative schedules for complete Cartesian physical fields.
use crate::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    Complex64, SolverError,
};

/// Borrowed full scalar or vector spectra. Their layouts belong to the comparison plan.
#[derive(Debug, Clone, Copy)]
pub enum PhysicalField<'a> {
    /// Scalar pressure or another independently supplied scalar field; no mean removal.
    Scalar(&'a [Complex64]),
    /// Complete vector field in fixed Cartesian component order.
    Vector([&'a [Complex64]; 3]),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Domain;

    #[test]
    fn sampled_curl_has_the_physical_orientation() {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
        let mut y = zero.clone();
        for mode in [[1, 0, 0], [-1, 0, 0]] {
            y[domain.layout().locate(mode).unwrap().0].re = 0.5;
        }
        let field = PhysicalField::Vector([&zero, &y, &zero]);
        let mut work = DerivativeWorkspace::new(domain, domain.layout(), 1 << 20).unwrap();
        let mut values = vec![0.0; domain.layout().real_len()];
        PhysicalQuantity::Vorticity
            .sample(2, field, &mut work, &mut values)
            .unwrap();
        for (i, value) in values.iter().enumerate() {
            let x = (i / 16) as f64 / 4.0;
            let k = std::f64::consts::TAU;
            assert!((value + k * (k * x).sin()).abs() < 1e-14);
        }
    }
}
impl<'a> PhysicalField<'a> {
    fn component(self, index: usize) -> &'a [Complex64] {
        match self {
            Self::Scalar(value) => value,
            Self::Vector(values) => values[index],
        }
    }
}

/// Physical field whose complete pointwise difference is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalQuantity {
    /// Scalar values, including a separately constructed pressure field.
    Scalar,
    /// Three spatial derivatives of a scalar field.
    ScalarGradient,
    /// Three vector components.
    Vector,
    /// Nine ordered vector-component/coordinate derivatives.
    Gradient,
    /// All 27 ordered vector-component/coordinate/coordinate derivatives.
    Hessian,
    /// Three curl components from independently sampled Cartesian partial derivatives.
    Vorticity,
}
impl PhysicalQuantity {
    /// Ordered scalar entries in the complete field; mixed Hessian entries count twice.
    pub fn components(self) -> usize {
        match self {
            Self::Scalar => 1,
            Self::ScalarGradient | Self::Vector | Self::Vorticity => 3,
            Self::Gradient => 9,
            Self::Hessian => 27,
        }
    }
    /// Exact scalar inverse-transform count for both sides of one successful comparison.
    pub fn scalar_transforms(self) -> usize {
        if self == Self::Vorticity {
            12
        } else {
            2 * self.components()
        }
    }
    pub(super) fn admit(self, field: PhysicalField<'_>) -> Result<(), SolverError> {
        let scalar = matches!(self, Self::Scalar | Self::ScalarGradient);
        if scalar != matches!(field, PhysicalField::Scalar(_)) {
            return Err(SolverError::InvalidPayload);
        }
        Ok(())
    }
    pub(super) fn sample(
        self,
        index: usize,
        field: PhysicalField<'_>,
        workspace: &mut DerivativeWorkspace,
        output: &mut [f64],
    ) -> Result<(), SolverError> {
        let (component, orders) = self.entry(index);
        let derivative = Derivative::new(orders)?;
        output.copy_from_slice(
            workspace
                .sample(field.component(component), derivative)?
                .values,
        );
        if self == Self::Vorticity {
            let mut orders = [0; 3];
            orders[(index + 2) % 3] = 1;
            let other =
                workspace.sample(field.component((index + 1) % 3), Derivative::new(orders)?)?;
            for (value, second) in output.iter_mut().zip(other.values) {
                *value = super::finite(*value - second)?;
            }
        }
        Ok(())
    }
    // Called only for admitted complete component indices; there is no partial-tensor API.
    fn entry(self, index: usize) -> (usize, [u8; 3]) {
        let mut orders = [0; 3];
        let component = match self {
            Self::Scalar => 0,
            Self::ScalarGradient => {
                orders[index] = 1;
                0
            }
            Self::Vector => index,
            Self::Gradient => {
                orders[index % 3] = 1;
                index / 3
            }
            Self::Hessian => {
                orders[(index / 3) % 3] += 1;
                orders[index % 3] += 1;
                index / 9
            }
            Self::Vorticity => {
                orders[(index + 1) % 3] = 1;
                (index + 2) % 3
            }
        };
        (component, orders)
    }
}
