//! Scaled Euclidean accumulation avoids squaring an otherwise representable large norm.
use crate::{Complex64, SolverError};

#[derive(Default)]
pub(crate) struct Squares {
    scale: f64,
    sum: f64,
}
impl Squares {
    pub(crate) fn complex(&mut self, value: Complex64, weight: f64) -> Result<(), SolverError> {
        self.push(value.re, weight)?;
        self.push(value.im, weight)
    }
    fn push(&mut self, value: f64, weight: f64) -> Result<(), SolverError> {
        let value = value.abs();
        if !value.is_finite() {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if value == 0.0 {
            return Ok(());
        }
        let scale = self.scale.max(value);
        self.sum = self.sum * (self.scale / scale).powi(2) + weight * (value / scale).powi(2);
        self.scale = scale;
        Ok(())
    }
    pub(crate) fn norm(self) -> Result<f64, SolverError> {
        finite(self.scale * self.sum.sqrt())
    }
}

pub(crate) fn finite(value: f64) -> Result<f64, SolverError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SolverError::ArithmeticResolutionLimited)
    }
}
