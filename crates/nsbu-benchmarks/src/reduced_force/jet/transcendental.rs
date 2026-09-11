//! Bounded formal compositions, with logarithmic recovery of underflowed exponential scales.
use super::{index::COUNT, Jet};
use crate::BenchmarkError;

impl Jet {
    /// Fractional power by the finite binomial polynomial; the constant must be positive.
    pub fn powf(self, exponent: f64) -> Result<Self, BenchmarkError> {
        if self.value() <= 0.0 || !exponent.is_finite() {
            return Err(BenchmarkError::InvalidInput);
        }
        let constant = self.value();
        let mut delta = self;
        delta.coefficients[0] = 0.0;
        delta = delta.scale(1.0 / constant)?;
        let mut result = Self::constant(1.0)?;
        let mut term = result;
        let mut factor = 1.0;
        for k in 1..=3 {
            factor *= (exponent - f64::from(k) + 1.0) / f64::from(k);
            if factor == 0.0 {
                break;
            }
            term = term.times(delta)?;
            result = result.plus(term.scale(factor)?)?;
        }
        result.rescale(constant.powf(exponent), exponent * constant.ln())
    }

    /// Exponential composition through degree three, with no convergence loop.
    pub fn exp(self) -> Result<Self, BenchmarkError> {
        let constant = self.value();
        let mut delta = self;
        delta.coefficients[0] = 0.0;
        let log_bound = self.exponential_log_bound();
        if log_bound < -750.0 {
            return Self::constant(0.0);
        }
        let mut result = Self::constant(1.0)?;
        let mut term = result;
        let mut factorial = 1.0;
        for k in 1..=3 {
            term = term.times(delta)?;
            factorial *= f64::from(k);
            result = result.plus(term.scale(1.0 / factorial)?)?;
        }
        result.rescale(constant.exp(), constant)
    }

    /// Upper logarithmic magnitude estimate for every formal exponential coefficient.
    /// This bounds exact polynomial magnitudes, not floating-point evaluation error.
    pub fn exponential_log_bound(self) -> f64 {
        let magnitude = self.coefficients[1..]
            .iter()
            .fold(0.0_f64, |a, b| a.max(b.abs()));
        // At most four polynomial terms, each bounded by max(1,20*|delta|)^3.
        self.value() + 4.0_f64.ln() + 3.0 * (20.0_f64.ln() + magnitude.ln()).max(0.0)
    }

    fn rescale(self, scale: f64, log_scale: f64) -> Result<Self, BenchmarkError> {
        if scale != 0.0 {
            return self.scale(scale);
        }
        let mut result = [0.0; COUNT];
        for (out, coefficient) in result.iter_mut().zip(self.coefficients) {
            if coefficient != 0.0 {
                *out = coefficient.signum() * (log_scale + coefficient.abs().ln()).exp();
            }
        }
        Self::checked(result)
    }
}
