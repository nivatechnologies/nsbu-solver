//! One exact requested interval, three independently selected exponential steps, and a private fine-result proposal.
use super::{
    indicator::{compare, Indicators, Tolerances},
    kernel::{field, mutable, readonly, Field, RightHandSide},
    method::{Method, MethodWorkspace},
    time::binary_duration,
    transaction::{AcceptedAttempt, CandidateState},
};
use crate::{
    domain::{validate_spectrum, Domain, ResourcePlan, SpectralState},
    SolverError,
};

/// Attempt report and optional single-use commit token. Local acceptance is not PDE convergence.
#[derive(Debug)]
pub struct AttemptResult {
    /// Empirical velocity and vorticity discrepancy channels.
    pub indicators: Indicators,
    /// Exact requested tick count; the core never adjusts it.
    pub ticks: u128,
    /// Number of RHS evaluations in this completed attempt, twelve for CM and fifteen for HO.
    pub rhs_calls: usize,
    /// Present only when both local channels pass.
    pub accepted: Option<AcceptedAttempt>,
}

/// Private, fixed-capacity storage for three steps sharing one committed starting state.
pub struct AttemptWorkspace {
    plan: ResourcePlan,
    kernel: MethodWorkspace,
    coefficients: CoefficientKey,
    coarse: Field,
    midpoint: Field,
}

const INVALID_COEFFICIENT_BITS: u64 = u64::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CoefficientKey {
    dt: u64,
    half_dt: u64,
}
impl CoefficientKey {
    const INVALID: Self = Self {
        dt: INVALID_COEFFICIENT_BITS,
        half_dt: INVALID_COEFFICIENT_BITS,
    };
    const fn is_valid(self) -> bool {
        self.dt != INVALID_COEFFICIENT_BITS && self.half_dt != INVALID_COEFFICIENT_BITS
    }
}

impl AttemptWorkspace {
    /// Additional kernel, comparison and table storage; caller also reserves state and RHS storage.
    pub fn reservation(domain: Domain) -> Result<usize, SolverError> {
        Self::reservation_with_method(domain, Method::CoxMatthews)
    }

    /// Reserve the selected independent method and comparison storage.
    pub fn reservation_with_method(domain: Domain, method: Method) -> Result<usize, SolverError> {
        let n = domain.layout().half_len();
        let arrays = n.checked_mul(6 * 16).ok_or(SolverError::SizeOverflow)?;
        MethodWorkspace::reservation(domain, method)?
            .checked_add(arrays)
            .and_then(|v| v.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Require this reservation in the plan's diagnostics/scratch class before allocating.
    pub fn new(plan: ResourcePlan) -> Result<Self, SolverError> {
        Self::new_with_method(plan, Method::CoxMatthews)
    }

    /// Allocate the selected method only after its complete reservation is admitted.
    pub fn new_with_method(plan: ResourcePlan, method: Method) -> Result<Self, SolverError> {
        if plan.classes()[6] < Self::reservation_with_method(plan.domain(), method)? {
            return Err(SolverError::ResourceLimit);
        }
        let n = plan.domain().layout().half_len();
        let kernel = MethodWorkspace::new(plan.domain(), method, plan.total())?;
        let coarse = field(n)?;
        let midpoint = field(n)?;
        Ok(Self {
            plan,
            kernel,
            coefficients: CoefficientKey::INVALID,
            coarse,
            midpoint,
        })
    }

    /// Method selected at workspace preflight.
    pub fn method(&self) -> Method {
        self.kernel.method()
    }

    /// Attempt exactly the requested interval. Rejection changes no committed field, clock or count.
    /// The RHS must satisfy its separately admitted finite cost and no-allocation contract.
    pub fn try_advance(
        &mut self,
        committed: &SpectralState,
        candidate: &mut CandidateState,
        ticks: u128,
        tolerances: Tolerances,
        rhs: &mut dyn RightHandSide,
    ) -> Result<AttemptResult, SolverError> {
        candidate.invalidate()?;
        self.admit(committed, candidate, tolerances, rhs)?;
        let stages = committed.clock().stages(ticks)?;
        let dt = binary_duration(ticks, committed.clock().exponent())?;
        let half_dt = binary_duration(ticks / 2, committed.clock().exponent())?;
        self.coefficients(dt, half_dt)?;
        rhs.begin_attempt_for_method(committed.clock(), ticks, self.method())?;
        self.kernel.step(
            true,
            readonly(&committed.components),
            [stages[0], stages[2], stages[4]],
            dt,
            rhs,
            mutable(&mut self.coarse),
        )?;
        self.kernel.step(
            false,
            readonly(&committed.components),
            [stages[0], stages[1], stages[2]],
            half_dt,
            rhs,
            mutable(&mut self.midpoint),
        )?;
        self.kernel.step(
            false,
            readonly(&self.midpoint),
            [stages[2], stages[3], stages[4]],
            half_dt,
            rhs,
            mutable(&mut candidate.state.components),
        )?;
        for field in [&self.coarse, &self.midpoint, &candidate.state.components] {
            for values in field {
                validate_spectrum(self.plan.domain().layout(), values, 1e-12)?;
            }
        }
        let indicators = compare(
            self.plan.domain(),
            &self.coarse,
            &candidate.state.components,
            tolerances,
        )?;
        let accepted = if indicators.ratios.iter().all(|value| *value <= 1.0) {
            candidate.state.clock = stages[4];
            candidate.state.epoch = committed.epoch().next()?;
            candidate.state.accepted_steps = committed
                .accepted_steps()
                .checked_add(1)
                .ok_or(SolverError::EpochExhausted)?;
            Some(candidate.accept(committed))
        } else {
            None
        };
        Ok(AttemptResult {
            indicators,
            ticks,
            rhs_calls: self.method().rhs_calls(),
            accepted,
        })
    }

    fn admit(
        &self,
        committed: &SpectralState,
        candidate: &CandidateState,
        tolerances: Tolerances,
        rhs: &dyn RightHandSide,
    ) -> Result<(), SolverError> {
        if committed.plan() != self.plan || candidate.state.plan() != self.plan {
            return Err(SolverError::InvalidPayload);
        }
        tolerances.validate()?;
        let bounds = rhs.bounds().ok_or(SolverError::UnknownProviderCost)?;
        if bounds.storage_bytes > self.plan.classes()[5] || bounds.work_units == 0 {
            return Err(SolverError::ResourceLimit);
        }
        bounds
            .work_units
            .checked_mul(self.method().rhs_calls())
            .ok_or(SolverError::SizeOverflow)?;
        bounds
            .scalar_transforms
            .checked_mul(self.method().rhs_calls())
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }

    fn coefficients(&mut self, dt: f64, half_dt: f64) -> Result<(), SolverError> {
        let requested = CoefficientKey {
            dt: dt.to_bits(),
            half_dt: half_dt.to_bits(),
        };
        if self.coefficients.is_valid() && self.coefficients == requested {
            return Ok(());
        }
        // In-place writes begin only after the old identity is invalidated. A failure or unwind
        // therefore cannot authenticate partially rebuilt tables.
        self.coefficients = CoefficientKey::INVALID;
        self.kernel.coefficients(self.plan.domain(), dt, half_dt)?;
        self.coefficients = requested;
        Ok(())
    }
}

#[cfg(test)]
#[path = "attempt_cache_tests.rs"]
mod cache_tests;
