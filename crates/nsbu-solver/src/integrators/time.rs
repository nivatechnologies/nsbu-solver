//! Exact binary64 interval admission; force clocks remain independent integer counts.
use crate::SolverError;

/// Convert a positive dyadic interval only if its complete significand is representable.
/// Refuses underflow, overflow and lost low bits instead of silently rounding ticks.
pub fn binary_duration(ticks: u128, exponent: i32) -> Result<f64, SolverError> {
    if ticks == 0 {
        return Err(SolverError::InvalidStep);
    }
    let shift = ticks.trailing_zeros();
    let significand = ticks >> shift;
    let power = i64::from(exponent) + i64::from(shift);
    if significand >> 53 != 0 || !(-1074..=1023).contains(&power) {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    let biased = (power + 1023).max(1) as u64;
    let down = (-1022 - power).max(0) as u32;
    let scale = f64::from_bits((biased << 52) >> down);
    let duration = significand as f64 * scale;
    if !duration.is_finite() {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    Ok(duration)
}
