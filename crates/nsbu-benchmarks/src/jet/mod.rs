//! Fixed degree-four Taylor algebra: coefficients are derivative divided by multi-index factorial.
mod index;
mod transcendental;
use crate::BenchmarkError;
use index::{COUNT, POWERS, PRODUCTS};

/// A four-variable degree-four Taylor polynomial, with no heap storage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jet {
    coefficients: [f64; COUNT],
}

impl Jet {
    /// A finite constant polynomial.
    pub fn constant(value: f64) -> Result<Self, BenchmarkError> {
        let mut coefficients = [0.0; COUNT];
        coefficients[0] = value;
        Self::checked(coefficients)
    }

    /// One independent variable, indexed x=0, y=1, z=2, physical time=3.
    pub fn variable(value: f64, axis: usize) -> Result<Self, BenchmarkError> {
        if axis >= 4 {
            return Err(BenchmarkError::InvalidInput);
        }
        let mut result = Self::constant(value)?;
        let mut power = [0; 4];
        power[axis] = 1;
        result.coefficients[Self::position(power)?] = 1.0;
        Ok(result)
    }

    /// Constant coefficient.
    pub fn value(self) -> f64 {
        self.coefficients[0]
    }
    /// Read-only complete coefficient vector in degree/lexicographic order.
    pub fn coefficients(&self) -> &[f64; 70] {
        &self.coefficients
    }
    /// Read a Taylor coefficient, refusing degrees above four.
    pub fn coefficient(self, power: [u8; 4]) -> Result<f64, BenchmarkError> {
        Ok(self.coefficients[Self::position(power)?])
    }

    /// Coefficientwise sum, refusing arithmetic overflow.
    pub fn plus(self, right: Self) -> Result<Self, BenchmarkError> {
        Self::checked(std::array::from_fn(|i| {
            self.coefficients[i] + right.coefficients[i]
        }))
    }
    /// Coefficientwise difference.
    pub fn minus(self, right: Self) -> Result<Self, BenchmarkError> {
        Self::checked(std::array::from_fn(|i| {
            self.coefficients[i] - right.coefficients[i]
        }))
    }
    /// Scalar multiplication.
    pub fn scale(self, factor: f64) -> Result<Self, BenchmarkError> {
        Self::checked(self.coefficients.map(|v| v * factor))
    }
    /// Total-degree-truncated convolution, using exactly 495 declared coefficient products.
    pub fn times(self, right: Self) -> Result<Self, BenchmarkError> {
        let mut result = [0.0; COUNT];
        for [a, b, out] in PRODUCTS {
            result[out as usize] += self.coefficients[a as usize] * right.coefficients[b as usize];
        }
        Self::checked(result)
    }
    /// Polynomial quotient with a strictly positive denominator constant.
    pub fn quotient(self, right: Self) -> Result<Self, BenchmarkError> {
        self.times(right.powf(-1.0)?)
    }

    /// Derivative polynomial, retaining the same fixed storage and zeroing unavailable top degree.
    pub fn derivative(self, axis: usize) -> Result<Self, BenchmarkError> {
        if axis >= 4 {
            return Err(BenchmarkError::InvalidInput);
        }
        let mut result = [0.0; COUNT];
        for (i, power) in POWERS.into_iter().enumerate() {
            let mut next = power;
            next[axis] += 1;
            if let Ok(source) = Self::position(next) {
                result[i] = f64::from(next[axis]) * self.coefficients[source];
            }
        }
        Self::checked(result)
    }

    fn position(power: [u8; 4]) -> Result<usize, BenchmarkError> {
        POWERS
            .iter()
            .position(|value| *value == power)
            .ok_or(BenchmarkError::InvalidInput)
    }
    fn checked(coefficients: [f64; COUNT]) -> Result<Self, BenchmarkError> {
        if coefficients.iter().any(|v| !v.is_finite()) {
            return Err(BenchmarkError::ArithmeticResolution);
        }
        Ok(Self { coefficients })
    }
}
