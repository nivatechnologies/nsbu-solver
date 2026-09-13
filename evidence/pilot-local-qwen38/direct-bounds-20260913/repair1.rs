use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundError {
    NonFiniteInput,
    NegativeInput,
    ZeroFloor,
    Overflow,
}

impl fmt::Display for BoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundError::NonFiniteInput => write!(f, "input is not finite"),
            BoundError::NegativeInput => write!(f, "input is negative"),
            BoundError::ZeroFloor => write!(f, "floor must be strictly positive"),
            BoundError::Overflow => write!(f, "result overflows"),
        }
    }
}

impl std::error::Error for BoundError {}

pub fn norm_bounds(error_norm: f64, correction_norm: f64) -> Result<(f64, f64), BoundError> {
    if !error_norm.is_finite() || !correction_norm.is_finite() {
        return Err(BoundError::NonFiniteInput);
    }
    if error_norm < 0.0 || correction_norm < 0.0 {
        return Err(BoundError::NegativeInput);
    }

    let lower = (error_norm - correction_norm).abs();
    let upper = error_norm + correction_norm;

    if !upper.is_finite() {
        return Err(BoundError::Overflow);
    }

    Ok((lower, upper))
}

pub fn relative_peak_bounds(
    archived_relative_peak: f64,
    correction_peak: f64,
    floor: f64,
) -> Result<(f64, f64), BoundError> {
    if !archived_relative_peak.is_finite() || !correction_peak.is_finite() || !floor.is_finite() {
        return Err(BoundError::NonFiniteInput);
    }
    if archived_relative_peak < 0.0 || correction_peak < 0.0 {
        return Err(BoundError::NegativeInput);
    }
    if floor <= 0.0 {
        return Err(BoundError::ZeroFloor);
    }

    let u = correction_peak / floor;
    if !u.is_finite() {
        return Err(BoundError::Overflow);
    }

    let lower = (archived_relative_peak - u).max(0.0);
    let upper = archived_relative_peak + u;

    if !upper.is_finite() {
        return Err(BoundError::Overflow);
    }

    Ok((lower, upper))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_norm_bounds_ordinary() {
        assert_eq!(norm_bounds(1.0, 3.0), Ok((2.0, 4.0)));
        assert_eq!(norm_bounds(3.0, 1.0), Ok((2.0, 4.0)));
        assert_eq!(norm_bounds(0.0, 0.0), Ok((0.0, 0.0)));
        assert_eq!(norm_bounds(5.0, 5.0), Ok((0.0, 10.0)));
    }

    #[test]
    fn test_norm_bounds_rejections() {
        assert_eq!(norm_bounds(f64::NAN, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(norm_bounds(1.0, f64::NAN), Err(BoundError::NonFiniteInput));
        assert_eq!(norm_bounds(f64::INFINITY, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(norm_bounds(1.0, f64::INFINITY), Err(BoundError::NonFiniteInput));
        assert_eq!(norm_bounds(-f64::INFINITY, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(norm_bounds(1.0, -f64::INFINITY), Err(BoundError::NonFiniteInput));
        assert_eq!(norm_bounds(-1.0, 1.0), Err(BoundError::NegativeInput));
        assert_eq!(norm_bounds(1.0, -1.0), Err(BoundError::NegativeInput));
    }

    #[test]
    fn test_norm_bounds_overflow() {
        let max = f64::MAX;
        assert_eq!(norm_bounds(max, max), Err(BoundError::Overflow));
    }

    #[test]
    fn test_relative_peak_bounds_ordinary() {
        // Counterexample from prompt:
        // reference values [30, 1000], old values [60, 1000], new values [63, 1000], floor=1
        // P0 = 1, correction_peak/floor = 3, P1 = 1.1
        // archived_relative_peak = P0 = 1.0
        // correction_peak = 3.0
        // floor = 1.0
        // u = 3.0 / 1.0 = 3.0
        // lower = max(0, 1.0 - 3.0) = 0.0
        // upper = 1.0 + 3.0 = 4.0
        let result = relative_peak_bounds(1.0, 3.0, 1.0);
        assert_eq!(result, Ok((0.0, 4.0)));

        let (lower, upper) = result.unwrap();
        let p1 = 1.1;
        assert!(p1 >= lower && p1 <= upper, "P1 should be inside [lower, upper]");

        // Invalid absolute-value lower bound would be abs(1.0 - 3.0) = 2.0
        let invalid_lower = (1.0f64 - 3.0f64).abs();
        assert!(p1 < invalid_lower, "P1 should be below the invalid absolute-value lower bound");
    }

    #[test]
    fn test_relative_peak_bounds_rejections() {
        assert_eq!(relative_peak_bounds(f64::NAN, 1.0, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(relative_peak_bounds(1.0, f64::NAN, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(relative_peak_bounds(1.0, 1.0, f64::NAN), Err(BoundError::NonFiniteInput));

        assert_eq!(relative_peak_bounds(f64::INFINITY, 1.0, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(relative_peak_bounds(1.0, f64::INFINITY, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(relative_peak_bounds(1.0, 1.0, f64::INFINITY), Err(BoundError::NonFiniteInput));

        assert_eq!(relative_peak_bounds(-f64::INFINITY, 1.0, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(relative_peak_bounds(1.0, -f64::INFINITY, 1.0), Err(BoundError::NonFiniteInput));
        assert_eq!(relative_peak_bounds(1.0, 1.0, -f64::INFINITY), Err(BoundError::NonFiniteInput));

        assert_eq!(relative_peak_bounds(-1.0, 1.0, 1.0), Err(BoundError::NegativeInput));
        assert_eq!(relative_peak_bounds(1.0, -1.0, 1.0), Err(BoundError::NegativeInput));

        assert_eq!(relative_peak_bounds(1.0, 1.0, 0.0), Err(BoundError::ZeroFloor));
        assert_eq!(relative_peak_bounds(1.0, 1.0, -1.0), Err(BoundError::ZeroFloor));
    }

    #[test]
    fn test_relative_peak_bounds_overflow() {
        // Division overflow: correction_peak / floor
        let max = f64::MAX;
        let tiny = f64::MIN_POSITIVE;
        // max / tiny is inf
        assert_eq!(relative_peak_bounds(1.0, max, tiny), Err(BoundError::Overflow));

        // Addition overflow: archived_relative_peak + u
        // Use archived_relative_peak=f64::MAX and correction_peak=f64::MAX with floor=1.0
        // u = max / 1.0 = max (finite)
        // upper = max + max = inf (overflow)
        assert_eq!(relative_peak_bounds(max, max, 1.0), Err(BoundError::Overflow));
    }
}
