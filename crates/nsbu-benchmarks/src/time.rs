//! Separate elapsed and remaining conversion for the exact v2 dyadic clock.
use crate::BenchmarkError;
use nsbu_solver::domain::TickClock;

/// Arithmetic record for separate conversion of the two physical time coordinates.
#[derive(Debug, Clone, Copy)]
pub struct BenchmarkTime {
    elapsed: f64,
    remaining: f64,
    errors: [f64; 2],
}

impl BenchmarkTime {
    /// Require target=1/128 exactly, before separately converting either tick count.
    pub fn new(clock: TickClock) -> Result<Self, BenchmarkError> {
        let target = clock.target();
        if !target.is_power_of_two()
            || i64::from(clock.exponent()) + i64::from(target.ilog2()) != -7
        {
            return Err(BenchmarkError::ClockIdentity);
        }
        let quantum = 2.0_f64.powi(clock.exponent());
        let elapsed = clock.elapsed() as f64 * quantum;
        let remaining = clock.remaining() as f64 * quantum;
        let errors = [
            conversion_error(clock.elapsed(), elapsed),
            conversion_error(clock.remaining(), remaining),
        ];
        Ok(Self {
            elapsed,
            remaining,
            errors,
        })
    }

    /// Startup coordinate; never reconstructed from the remaining coordinate.
    pub fn elapsed(self) -> f64 {
        self.elapsed
    }
    /// Positive concentrating coordinate; never obtained by subtracting rounded elapsed time.
    pub fn remaining(self) -> f64 {
        self.remaining
    }
    /// Conservative absolute rounding estimates; exact significands receive zero.
    pub fn rounding_estimates(self) -> [f64; 2] {
        self.errors
    }
}

fn conversion_error(count: u128, value: f64) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let significand = count >> count.trailing_zeros();
    if significand >> 53 == 0 {
        0.0
    } else {
        f64::EPSILON * value
    }
}
