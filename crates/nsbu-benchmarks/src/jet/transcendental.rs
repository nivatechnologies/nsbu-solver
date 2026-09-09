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
        for k in 1..=4 {
            factor *= (exponent - f64::from(k) + 1.0) / f64::from(k);
            if factor == 0.0 {
                break;
            }
            term = term.times(delta)?;
            result = result.plus(term.scale(factor)?)?;
        }
        result.rescale(constant.powf(exponent), exponent * constant.ln())
    }

    /// Exponential composition through degree four, with no convergence loop.
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
        for k in 1..=4 {
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
        // At most five polynomial terms, each bounded by max(1,70*|delta|)^4.
        self.value() + 5.0_f64.ln() + 4.0 * (70.0_f64.ln() + magnitude.ln()).max(0.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logarithmic_majorant_and_strict_underflow_admission() {
        let small = Jet::variable(-3.0, 0).unwrap().scale(0.01).unwrap();
        assert!((small.exponential_log_bound() - (-0.03 + 5.0_f64.ln())).abs() < 1e-15);
        let large = Jet::variable(0.0, 0).unwrap().scale(1e85).unwrap();
        let bound = large.exponential_log_bound();
        assert!((bound - (5.0_f64.ln() + 4.0 * (70e85_f64).ln())).abs() < 1e-12);
        let boundary = large.plus(Jet::constant(-750.0 - bound).unwrap()).unwrap();
        assert_eq!(boundary.exponential_log_bound(), -750.0);
        assert_eq!(
            boundary.exp().unwrap_err(),
            BenchmarkError::ArithmeticResolution
        );
        assert_eq!(
            boundary
                .plus(Jet::constant(-1.0).unwrap())
                .unwrap()
                .exp()
                .unwrap(),
            Jet::constant(0.0).unwrap()
        );
    }
}
