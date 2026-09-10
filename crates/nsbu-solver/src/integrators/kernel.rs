//! One CM step with fixed storage and exactly four externally bounded RHS evaluations.
use super::coefficients::CmCoefficients;
use crate::{domain::TickClock, Complex64, SolverError};

/// Component-major vector payload used only as independently owned working storage.
pub type Field = [Vec<Complex64>; 3];

/// Declared storage and per-call work for an independently admitted RHS implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhsBounds {
    /// Complete owned implementation storage, including mutable scratch.
    pub storage_bytes: usize,
    /// Finite provider work units per call; spatial loops are bounded by the plan layout.
    pub work_units: usize,
    /// Scalar 3D transforms per call, including pressure diagnostics.
    pub scalar_transforms: usize,
}

/// Bounded RHS contract. Implementations must honor their predeclared work and storage limits.
/// This interface provides no analytical-reference assignment operation.
pub trait RightHandSide {
    /// Unknown callback costs are allowed only in the standalone research step kernel.
    fn bounds(&self) -> Option<RhsBounds> {
        None
    }

    /// Establish one attempted interval. Research kernels may supply an unbounded RHS; production providers override this budget hook.
    fn begin_attempt(&mut self, _clock: TickClock, _ticks: u128) -> Result<(), SolverError> {
        Ok(())
    }

    /// Start the chosen method's bounded attempt; default callbacks retain their interval hook.
    fn begin_attempt_for_method(
        &mut self,
        clock: TickClock,
        ticks: u128,
        _method: super::method::Method,
    ) -> Result<(), SolverError> {
        self.begin_attempt(clock, ticks)
    }

    /// Fill output for the exact requested clock; no allocation, I/O or hidden retry.
    fn evaluate(
        &mut self,
        state: [&[Complex64]; 3],
        time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError>;
}

/// Five vector scratch slots reused by every step in an attempt.
pub struct CmWorkspace {
    n1: Field,
    na: Field,
    nb: Field,
    nc: Field,
    stage: Field,
}

impl CmWorkspace {
    /// Complete owned array and header reservation, checked before allocation.
    pub fn reservation(length: usize) -> Result<usize, SolverError> {
        if length == 0 {
            return Err(SolverError::InvalidPayload);
        }
        length
            .checked_mul(15 * 16)
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Allocate only after the complete kernel reservation fits the supplied cap.
    pub fn new(length: usize, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(length)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let n1 = field(length)?;
        let na = field(length)?;
        let nb = field(length)?;
        let nc = field(length)?;
        let stage = field(length)?;
        Ok(Self {
            n1,
            na,
            nb,
            nc,
            stage,
        })
    }

    /// Exactly one step. Callers provide precomputed full or half interval coefficients.
    pub fn step(
        &mut self,
        state: [&[Complex64]; 3],
        times: [TickClock; 3],
        dt: f64,
        coefficients: &[CmCoefficients],
        rhs: &mut dyn RightHandSide,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        let n = self.stage[0].len();
        if !dt.is_finite()
            || dt <= 0.0
            || coefficients.len() != n
            || state.iter().any(|v| v.len() != n)
            || output.iter().any(|v| v.len() != n)
        {
            return Err(SolverError::InvalidPayload);
        }
        evaluate_checked(rhs, state, times[0], mutable(&mut self.n1))?;
        stage(state, &self.n1, coefficients, dt, &mut self.stage);
        evaluate_checked(rhs, readonly(&self.stage), times[1], mutable(&mut self.na))?;
        stage(state, &self.na, coefficients, dt, &mut self.stage);
        evaluate_checked(rhs, readonly(&self.stage), times[1], mutable(&mut self.nb))?;
        self.final_stage(state, coefficients, dt);
        evaluate_checked(rhs, readonly(&self.stage), times[2], mutable(&mut self.nc))?;
        self.finish(state, coefficients, dt, output)
    }

    fn final_stage(&mut self, state: [&[Complex64]; 3], coefficients: &[CmCoefficients], dt: f64) {
        for (axis, values) in state.into_iter().enumerate() {
            for (i, (&u, c)) in values.iter().zip(coefficients).enumerate() {
                let a = c.half_exponential * u + dt * c.q * self.n1[axis][i];
                self.stage[axis][i] =
                    c.half_exponential * a + dt * c.q * (2.0 * self.nb[axis][i] - self.n1[axis][i]);
            }
        }
    }

    fn finish(
        &self,
        state: [&[Complex64]; 3],
        coefficients: &[CmCoefficients],
        dt: f64,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for (axis, values) in output.into_iter().enumerate() {
            for (i, (value, c)) in values.iter_mut().zip(coefficients).enumerate() {
                *value = c.exponential * state[axis][i]
                    + dt * (c.weights[0] * self.n1[axis][i]
                        + c.weights[1] * (self.na[axis][i] + self.nb[axis][i])
                        + c.weights[2] * self.nc[axis][i]);
                if !value.re.is_finite() || !value.im.is_finite() {
                    return Err(SolverError::InvalidSpectrum);
                }
            }
        }
        Ok(())
    }
}

fn stage(
    state: [&[Complex64]; 3],
    source: &Field,
    coefficients: &[CmCoefficients],
    dt: f64,
    output: &mut Field,
) {
    for (axis, values) in output.iter_mut().enumerate() {
        for (i, (value, c)) in values.iter_mut().zip(coefficients).enumerate() {
            *value = c.half_exponential * state[axis][i] + dt * c.q * source[axis][i];
        }
    }
}

pub(crate) use crate::storage::field;

pub(crate) fn readonly(field: &Field) -> [&[Complex64]; 3] {
    [&field[0], &field[1], &field[2]]
}
pub(crate) fn mutable(field: &mut Field) -> [&mut [Complex64]; 3] {
    let [a, b, c] = field;
    [a, b, c]
}

pub(crate) fn evaluate_checked(
    rhs: &mut dyn RightHandSide,
    state: [&[Complex64]; 3],
    time: TickClock,
    output: [&mut [Complex64]; 3],
) -> Result<(), SolverError> {
    finite(state)?;
    let [a, b, c] = output;
    rhs.evaluate(state, time, [&mut *a, &mut *b, &mut *c])?;
    finite([a, b, c])
}

pub(crate) fn finite(field: [&[Complex64]; 3]) -> Result<(), SolverError> {
    for values in field {
        if values
            .iter()
            .any(|v| !v.re.is_finite() || !v.im.is_finite())
        {
            return Err(SolverError::InvalidSpectrum);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn field_allocation_refusal_propagates_without_partial_payload_construction() {
        assert_eq!(
            super::field(usize::MAX).unwrap_err(),
            crate::SolverError::AllocationFailed
        );
    }
}
