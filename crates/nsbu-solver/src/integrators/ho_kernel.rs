//! Independent five-stage HO engine with finite work and caller-owned coefficient tables.
use super::{
    ho_coefficients::HoCoefficients,
    kernel::{evaluate_checked, field, finite, mutable, readonly, Field, RightHandSide},
};
use crate::{domain::TickClock, Complex64, SolverError};

/// Five source fields and one stage field, reused without allocation during stepping.
pub struct HoWorkspace {
    sources: [Field; 5],
    stage: Field,
}

impl HoWorkspace {
    /// Reserve all source/stage arrays and their object headers before allocation.
    pub fn reservation(length: usize) -> Result<usize, SolverError> {
        if length == 0 {
            return Err(SolverError::InvalidPayload);
        }
        length
            .checked_mul(18 * 16)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Allocate the exact fixed workspace only when the reservation fits the supplied cap.
    pub fn new(length: usize, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(length)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        Ok(Self {
            sources: [
                field(length)?,
                field(length)?,
                field(length)?,
                field(length)?,
                field(length)?,
            ],
            stage: field(length)?,
        })
    }

    /// One HO step with requested source times start, half, half, end, half.
    pub fn step(
        &mut self,
        state: [&[Complex64]; 3],
        times: [TickClock; 3],
        dt: f64,
        coefficients: &[HoCoefficients],
        rhs: &mut dyn RightHandSide,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        let length = self.stage[0].len();
        if !dt.is_finite()
            || dt <= 0.0
            || coefficients.len() != length
            || state.iter().any(|values| values.len() != length)
            || output.iter().any(|values| values.len() != length)
        {
            return Err(SolverError::InvalidPayload);
        }
        finite(state)?;
        for (index, time_index) in [0, 1, 1, 2, 1].into_iter().enumerate() {
            self.prepare(index, state, coefficients, dt);
            evaluate_checked(
                rhs,
                readonly(&self.stage),
                times[time_index],
                mutable(&mut self.sources[index]),
            )?;
        }
        self.finish(state, coefficients, dt, output)
    }

    fn prepare(
        &mut self,
        stage: usize,
        state: [&[Complex64]; 3],
        tables: &[HoCoefficients],
        dt: f64,
    ) {
        for (axis, values) in state.into_iter().enumerate() {
            for (index, (value, table)) in values.iter().zip(tables).enumerate() {
                let exponential = match stage {
                    0 => 1.0,
                    3 => table.exponential,
                    _ => table.half_exponential,
                };
                let mut source = Complex64::new(0.0, 0.0);
                for earlier in 0..stage {
                    source += table.rows[stage][earlier] * self.sources[earlier][axis][index];
                }
                self.stage[axis][index] = exponential * value + dt * source;
            }
        }
    }

    fn finish(
        &self,
        state: [&[Complex64]; 3],
        tables: &[HoCoefficients],
        dt: f64,
        mut output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for (axis, values) in output.iter_mut().enumerate() {
            for (index, (value, table)) in values.iter_mut().zip(tables).enumerate() {
                let source: Complex64 = self
                    .sources
                    .iter()
                    .zip(table.weights)
                    .map(|(field, weight)| weight * field[axis][index])
                    .sum();
                *value = table.exponential * state[axis][index] + dt * source;
            }
        }
        finite([output[0], output[1], output[2]])
    }
}
