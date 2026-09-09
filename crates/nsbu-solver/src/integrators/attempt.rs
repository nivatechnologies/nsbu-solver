//! One exact requested interval, three CM steps, and a private fine-result proposal.
use super::{
    coefficients::CmCoefficients,
    indicator::{compare, Indicators, Tolerances},
    kernel::{field, mutable, readonly, CmWorkspace, Field, RightHandSide},
    time::binary_duration,
    transaction::{AcceptedAttempt, CandidateState},
};
use crate::{
    domain::{validate_spectrum, Domain, ResourcePlan, SpectralState},
    spectral::modal,
    storage::filled,
    SolverError,
};

/// Attempt report and optional single-use commit token. Local acceptance is not PDE convergence.
#[derive(Debug)]
pub struct AttemptResult {
    /// Empirical velocity and vorticity discrepancy channels.
    pub indicators: Indicators,
    /// Exact requested tick count; the core never adjusts it.
    pub ticks: u128,
    /// Number of RHS evaluations in this completed attempt, always twelve.
    pub rhs_calls: usize,
    /// Present only when both local channels pass.
    pub accepted: Option<AcceptedAttempt>,
}

/// Private, fixed-capacity storage for three steps sharing one committed starting state.
pub struct AttemptWorkspace {
    plan: ResourcePlan,
    kernel: CmWorkspace,
    coarse: Field,
    midpoint: Field,
    full: Vec<CmCoefficients>,
    half: Vec<CmCoefficients>,
}

impl AttemptWorkspace {
    /// Additional kernel, comparison and table storage; caller also reserves state and RHS storage.
    pub fn reservation(domain: Domain) -> Result<usize, SolverError> {
        let n = domain.layout().half_len();
        let arrays = n
            .checked_mul(6 * 16 + 2 * std::mem::size_of::<CmCoefficients>())
            .ok_or(SolverError::SizeOverflow)?;
        CmWorkspace::reservation(n)?
            .checked_add(arrays)
            .and_then(|v| v.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Require this reservation in the plan's diagnostics/scratch class before allocating.
    pub fn new(plan: ResourcePlan) -> Result<Self, SolverError> {
        if plan.classes()[6] < Self::reservation(plan.domain())? {
            return Err(SolverError::ResourceLimit);
        }
        let n = plan.domain().layout().half_len();
        let zero = CmCoefficients::new(0.0)?;
        let kernel = CmWorkspace::new(n, plan.total())?;
        let coarse = field(n)?;
        let midpoint = field(n)?;
        let full = filled(n, zero)?;
        let half = filled(n, zero)?;
        Ok(Self {
            plan,
            kernel,
            coarse,
            midpoint,
            full,
            half,
        })
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
        rhs.begin_attempt(committed.clock(), ticks)?;
        self.kernel.step(
            readonly(&committed.components),
            [stages[0], stages[2], stages[4]],
            dt,
            &self.full,
            rhs,
            mutable(&mut self.coarse),
        )?;
        self.kernel.step(
            readonly(&committed.components),
            [stages[0], stages[1], stages[2]],
            half_dt,
            &self.half,
            rhs,
            mutable(&mut self.midpoint),
        )?;
        self.kernel.step(
            readonly(&self.midpoint),
            [stages[2], stages[3], stages[4]],
            half_dt,
            &self.half,
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
            rhs_calls: 12,
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
            .checked_mul(12)
            .ok_or(SolverError::SizeOverflow)?;
        bounds
            .scalar_transforms
            .checked_mul(12)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }

    fn coefficients(&mut self, dt: f64, half_dt: f64) -> Result<(), SolverError> {
        let domain = self.plan.domain();
        let layout = domain.layout();
        for index in 0..layout.half_len() {
            let position = layout.position(index)?;
            let decay = if layout.is_nyquist(position)? {
                0.0
            } else {
                let k = modal::wavevector(domain, layout.mode(position)?)?;
                -domain.viscosity() * k.iter().map(|value| value * value).sum::<f64>()
            };
            let full_argument = dt * decay;
            if !full_argument.is_finite() {
                return Err(SolverError::ArithmeticResolutionLimited);
            }
            self.full[index] = CmCoefficients::new(full_argument)?;
            self.half[index] = CmCoefficients::new(half_dt * decay)?;
        }
        Ok(())
    }
}
