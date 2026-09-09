//! Bounded rotational RHS: provider accounting, transform accounting and advective refusal.
use super::{
    forcing::{ForceLimits, PrescribedForce},
    kernel::{field, mutable, readonly, Field, RhsBounds, RightHandSide},
};
use crate::{
    domain::{Domain, TickClock},
    spectral::RotationalWorkspace,
    storage::filled,
    Complex64, SolverError,
};

/// One trajectory's source and spatial operator, with no reference-field interface.
pub struct SpectralRhs<F: PrescribedForce> {
    force: F,
    storage_bytes: usize,
    limits: ForceLimits,
    operator: RotationalWorkspace,
    source: Field,
    pressure: Vec<Complex64>,
    remaining_calls: usize,
    duration: f64,
    advective_limit: f64,
    calls: usize,
    work_units: usize,
    transforms: usize,
}

impl<F: PrescribedForce> SpectralRhs<F> {
    /// Complete declaration for owned operator, force, output and provider storage.
    pub fn reservation(domain: Domain, limits: ForceLimits) -> Result<usize, SolverError> {
        if limits.work_units == 0 || limits.remaining_divisor == 0 {
            return Err(SolverError::UnknownProviderCost);
        }
        limits
            .work_units
            .checked_mul(12)
            .ok_or(SolverError::SizeOverflow)?;
        limits
            .scalar_transforms
            .checked_add(10)
            .and_then(|v| v.checked_mul(12))
            .ok_or(SolverError::SizeOverflow)?;
        let buffers = domain
            .layout()
            .half_len()
            .checked_mul(4 * 16)
            .ok_or(SolverError::SizeOverflow)?;
        RotationalWorkspace::reservation(domain)?
            .checked_add(buffers)
            .and_then(|v| v.checked_add(limits.storage_bytes))
            .and_then(|v| v.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Admit a declared provider and preflight complete storage before allocating operator buffers.
    pub fn new(
        domain: Domain,
        force: F,
        advective_limit: f64,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = force.limits().ok_or(SolverError::UnknownProviderCost)?;
        if Self::reservation(domain, limits)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        if !advective_limit.is_finite() || advective_limit <= 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let n = domain.layout().half_len();
        Ok(Self {
            force,
            storage_bytes: Self::reservation(domain, limits)?,
            limits,
            operator: RotationalWorkspace::new(domain, cap)?,
            source: field(n)?,
            pressure: filled(n, Complex64::new(0.0, 0.0))?,
            remaining_calls: 0,
            duration: 0.0,
            advective_limit,
            calls: 0,
            work_units: 0,
            transforms: 0,
        })
    }

    /// Invocations and charged provider work/transform budgets for the current attempt.
    /// Failed calls retain their full reservation; successful provider reports release unused work.
    pub fn consumption(&self) -> [usize; 3] {
        [self.calls, self.work_units, self.transforms]
    }
}

impl<F: PrescribedForce> RightHandSide for SpectralRhs<F> {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: self.storage_bytes,
            work_units: self.limits.work_units,
            scalar_transforms: self.limits.scalar_transforms + 10,
        })
    }
    fn begin_attempt(&mut self, clock: TickClock, ticks: u128) -> Result<(), SolverError> {
        self.remaining_calls = 0;
        self.calls = 0;
        self.work_units = 0;
        self.transforms = 0;
        if ticks > clock.remaining() / self.limits.remaining_divisor {
            return Err(SolverError::InvalidStep);
        }
        self.duration = super::time::binary_duration(ticks, clock.exponent())?;
        self.remaining_calls = 12;
        Ok(())
    }

    fn evaluate(
        &mut self,
        state: [&[Complex64]; 3],
        time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        if self.remaining_calls == 0 || self.force.limits() != Some(self.limits) {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.remaining_calls -= 1;
        self.calls += 1;
        self.work_units += self.limits.work_units;
        self.transforms += self.limits.scalar_transforms + 10;
        let report = self
            .force
            .evaluate(time, self.limits, mutable(&mut self.source))?;
        if report.work_units > self.limits.work_units
            || report.scalar_transforms > self.limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.work_units -= self.limits.work_units - report.work_units;
        self.transforms -= self.limits.scalar_transforms - report.scalar_transforms;
        self.operator
            .evaluate(state, readonly(&self.source), output, &mut self.pressure)?;
        if self.operator.advective_number(self.duration)? > self.advective_limit {
            return Err(SolverError::AdvectiveLimit);
        }
        Ok(())
    }
}
