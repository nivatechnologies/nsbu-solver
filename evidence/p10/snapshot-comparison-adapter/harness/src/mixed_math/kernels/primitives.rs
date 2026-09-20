//! Scalar and complex arithmetic primitives shared by the band accumulators.
//!
//! Every function here is a byte-for-byte relocation from the original
//! `mixed_math` module: same floating-point operation order, same guards and
//! the same refusal messages.

use nsbu_solver::Complex64;

pub(super) fn curl(k: [f64; 3], u: [Complex64; 3]) -> [Complex64; 3] {
    [
        k[1] * u[2] - k[2] * u[1],
        k[2] * u[0] - k[0] * u[2],
        k[0] * u[1] - k[1] * u[0],
    ]
}

pub(super) fn real_inner(left: Complex64, right: Complex64) -> f64 {
    left.re * right.re + left.im * right.im
}

#[derive(Clone, Copy, Default)]
pub(super) struct Compensated {
    sum: f64,
    correction: f64,
}

impl Compensated {
    pub(super) fn push(&mut self, value: f64) -> Result<(), String> {
        let value = finite(value)?;
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = finite(next)?;
        Ok(())
    }

    pub(super) fn finish(self) -> Result<f64, String> {
        finite(self.sum)
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct ScaledSquares {
    scale: f64,
    sum: f64,
}

impl ScaledSquares {
    pub(super) fn complex(&mut self, value: Complex64, weight: f64) -> Result<(), String> {
        self.push(value.re, weight)?;
        self.push(value.im, weight)
    }

    pub(super) fn push(&mut self, value: f64, weight: f64) -> Result<(), String> {
        let value = value.abs();
        if !value.is_finite() || !weight.is_finite() || weight < 0.0 {
            return Err("mixed diagnostic norm arithmetic is nonfinite".into());
        }
        if value == 0.0 || weight == 0.0 {
            return Ok(());
        }
        let scale = self.scale.max(value);
        self.sum = self.sum * (self.scale / scale).powi(2) + weight * (value / scale).powi(2);
        self.scale = scale;
        Ok(())
    }

    pub(super) fn norm(self) -> Result<f64, String> {
        finite(self.scale * self.sum.sqrt())
    }
}

pub(super) fn finite(value: f64) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("mixed diagnostic arithmetic is nonfinite".into())
    }
}
