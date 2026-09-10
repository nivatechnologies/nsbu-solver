//! Explicit method selection with independently constructed coefficient tables and kernels.
use super::{
    coefficients::CmCoefficients,
    ho_coefficients::HoCoefficients,
    ho_kernel::HoWorkspace,
    kernel::{CmWorkspace, RightHandSide},
};
use crate::{
    domain::{Domain, TickClock},
    spectral::modal,
    storage::filled,
    Complex64, SolverError,
};

/// Independent fourth-order exponential Runge–Kutta methods supported by bounded attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Four-stage Cox–Matthews method.
    CoxMatthews,
    /// Five-stage Hochbruck–Ostermann method with nonmonotone stage times.
    HochbruckOstermann,
}
impl Method {
    /// Exact source-call budget for one full/two-half attempt.
    pub const fn rhs_calls(self) -> usize {
        match self {
            Self::CoxMatthews => 12,
            Self::HochbruckOstermann => 15,
        }
    }
}

pub(crate) enum MethodWorkspace {
    Cm(CmWorkspace, Vec<CmCoefficients>, Vec<CmCoefficients>),
    Ho(HoWorkspace, Vec<HoCoefficients>, Vec<HoCoefficients>),
}
impl MethodWorkspace {
    pub(crate) fn reservation(domain: Domain, method: Method) -> Result<usize, SolverError> {
        let n = domain.layout().half_len();
        let (kernel, coefficient) = match method {
            Method::CoxMatthews => (
                CmWorkspace::reservation(n)?,
                std::mem::size_of::<CmCoefficients>(),
            ),
            Method::HochbruckOstermann => (
                HoWorkspace::reservation(n)?,
                std::mem::size_of::<HoCoefficients>(),
            ),
        };
        n.checked_mul(2 * coefficient)
            .and_then(|bytes| bytes.checked_add(kernel))
            .ok_or(SolverError::SizeOverflow)
    }
    pub(crate) fn new(domain: Domain, method: Method, cap: usize) -> Result<Self, SolverError> {
        let n = domain.layout().half_len();
        Ok(match method {
            Method::CoxMatthews => Self::Cm(
                CmWorkspace::new(n, cap)?,
                filled(n, CmCoefficients::new(0.0)?)?,
                filled(n, CmCoefficients::new(0.0)?)?,
            ),
            Method::HochbruckOstermann => Self::Ho(
                HoWorkspace::new(n, cap)?,
                filled(n, HoCoefficients::new(0.0)?)?,
                filled(n, HoCoefficients::new(0.0)?)?,
            ),
        })
    }
    pub(crate) fn method(&self) -> Method {
        match self {
            Self::Cm(..) => Method::CoxMatthews,
            Self::Ho(..) => Method::HochbruckOstermann,
        }
    }
    pub(crate) fn coefficients(
        &mut self,
        domain: Domain,
        dt: f64,
        half_dt: f64,
    ) -> Result<(), SolverError> {
        let layout = domain.layout();
        for index in 0..layout.half_len() {
            let position = layout.position(index)?;
            let decay = if layout.is_nyquist(position)? {
                0.0
            } else {
                let k = modal::wavevector(domain, layout.mode(position)?)?;
                -domain.viscosity() * k.iter().map(|value| value * value).sum::<f64>()
            };
            let full = dt * decay;
            if !full.is_finite() {
                return Err(SolverError::ArithmeticResolutionLimited);
            }
            self.store(index, full, half_dt * decay)?;
        }
        Ok(())
    }
    fn store(&mut self, index: usize, z: f64, half_z: f64) -> Result<(), SolverError> {
        match self {
            Self::Cm(_, full, half) => {
                full[index] = CmCoefficients::new(z)?;
                half[index] = CmCoefficients::new(half_z)?;
            }
            Self::Ho(_, full, half) => {
                full[index] = HoCoefficients::new(z)?;
                half[index] = HoCoefficients::new(half_z)?;
            }
        }
        Ok(())
    }
    pub(crate) fn step(
        &mut self,
        full_step: bool,
        state: [&[Complex64]; 3],
        times: [TickClock; 3],
        dt: f64,
        rhs: &mut dyn RightHandSide,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        match self {
            Self::Cm(kernel, full, half) => kernel.step(
                state,
                times,
                dt,
                if full_step { full } else { half },
                rhs,
                output,
            ),
            Self::Ho(kernel, full, half) => kernel.step(
                state,
                times,
                dt,
                if full_step { full } else { half },
                rhs,
                output,
            ),
        }
    }
}
