//! Compensated signed reductions; finite floating measurements, not certified bounds.
use super::squares::finite;
use crate::{Complex64, SolverError};

#[derive(Default)]
pub(crate) struct Sum {
    value: f64,
    correction: f64,
}
impl Sum {
    pub(crate) fn dot(
        &mut self,
        left: Complex64,
        right: Complex64,
        weight: f64,
    ) -> Result<(), SolverError> {
        self.add(weight * left.re * right.re)?;
        self.add(weight * left.im * right.im)
    }

    pub(crate) fn add(&mut self, term: f64) -> Result<(), SolverError> {
        let total = finite(self.value + term)?;
        let recovered = total - self.value;
        let error = (self.value - (total - recovered)) + (term - recovered);
        self.correction = finite(self.correction + error)?;
        self.value = total;
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<f64, SolverError> {
        finite(self.value + self.correction)
    }
}
