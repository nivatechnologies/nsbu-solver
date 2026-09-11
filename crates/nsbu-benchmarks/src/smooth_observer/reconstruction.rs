//! Transactional accepted-node Hermite reconstruction for smooth and exact-v2 owners.
//!
//! The endpoint derivative is evaluated from the observer-owned, double-grid conservative
//! product formed for the balance sample.  It therefore neither asks the integrator for a stage
//! derivative nor evaluates the prescribed force a second time.

use super::{field, BalanceObserver, BalanceObserverLimits, BalanceObserverWork, ForceBalance};
use crate::smooth::CyclicSine;
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, hermite::HermiteWeights},
    domain::{validate_spectrum, Domain, Epoch, ResourcePlan, SpectralState, TickClock},
    experiment::observer::{BalanceObserver as BalanceObserverContract, ObserverBounds},
    integrators::forcing::PrescribedForce,
    spectral::{modal, transfer},
    Complex64, SolverError,
};

pub mod archive;
mod snapshot;
pub use snapshot::ReconstructionSnapshot;

type Field = [Vec<Complex64>; 3];

#[derive(Debug)]
struct Node {
    clock: Option<TickClock>,
    epoch: Epoch,
    steps: u128,
    value: Field,
    derivative: Field,
}

/// Crate-private borrowed provenance for one retained, un-interpolated accepted node.
pub(crate) struct AcceptedNodeView<'a> {
    pub(crate) domain: Domain,
    pub(crate) clock: TickClock,
    pub(crate) epoch: Epoch,
    pub(crate) steps: u128,
    pub(crate) value: [&'a [Complex64]; 3],
}

impl Node {
    fn new(domain: Domain) -> Result<Self, SolverError> {
        let zero = Complex64::new(0.0, 0.0);
        Ok(Self {
            clock: None,
            epoch: Epoch(0),
            steps: 0,
            value: field(domain.layout().half_len(), zero)?,
            derivative: field(domain.layout().half_len(), zero)?,
        })
    }

    fn copy_value(&mut self, state: &SpectralState) -> Result<(), SolverError> {
        self.clock = Some(state.clock());
        self.epoch = state.epoch();
        self.steps = state.accepted_steps();
        for axis in 0..3 {
            self.value[axis].copy_from_slice(state.component(axis)?);
        }
        Ok(())
    }
}

/// Complete reservation and finite endpoint budget for [`ReconstructionObserver`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconstructionObserverLimits {
    /// Complete fixed storage, including the independent balance observer.
    pub storage_bytes: usize,
    /// Includes the separate initial-rest derivative evaluation.
    pub samples: usize,
    /// Total admitted force-provider work units.
    pub work_units: usize,
    /// Total admitted independent conservative/force transforms.
    pub scalar_transforms: usize,
    /// Bounded retained-band coefficient visits used to add the viscous RHS term.
    pub modal_visits: usize,
}

/// Independently evaluated RHS data for three accepted endpoints and one private proposal.
///
/// Construction evaluates and retains the actual rest node.  Each later `measure` stages one
/// endpoint; only the runner's `commit_pending`, called after physical and history commits,
/// changes the accepted ring.  Failed or rejected proposals retain their charged work but leave
/// that ring bit-for-bit unchanged.
#[derive(Debug)]
pub struct ReconstructionObserver<F: PrescribedForce = CyclicSine> {
    plan: ResourcePlan,
    source: Domain,
    limits: ReconstructionObserverLimits,
    balance: ForceBalance<F>,
    accepted: [Node; 3],
    accepted_count: usize,
    pending: Node,
    has_pending: bool,
    modal_visits: usize,
}

impl ReconstructionObserver<CyclicSine> {
    /// Declare storage and all work before allocation. `samples` includes the rest node.
    pub fn limits(
        source: Domain,
        samples: usize,
    ) -> Result<ReconstructionObserverLimits, SolverError> {
        if samples == 0 {
            return Err(SolverError::ResourceLimit);
        }
        let balance = BalanceObserver::limits(source, samples)?;
        Self::limits_for(source, samples, balance)
    }

    /// Allocate fixed storage and evaluate the first accepted endpoint from the actual rest
    /// state. The initial evaluation consumes one independent provider budget slot.
    pub fn new(
        plan: ResourcePlan,
        samples: usize,
        from_rest: &SpectralState,
    ) -> Result<Self, SolverError> {
        let limits = Self::limits(plan.domain(), samples)?;
        Self::admit(plan, from_rest, limits)?;
        let balance = BalanceObserver::new(plan, samples)?;
        Self::new_owned(plan, from_rest, limits, balance)
    }
}

impl<F: PrescribedForce> ReconstructionObserver<F> {
    pub(super) fn admit(
        plan: ResourcePlan,
        from_rest: &SpectralState,
        limits: ReconstructionObserverLimits,
    ) -> Result<(), SolverError> {
        validate_rest(plan, from_rest)?;
        if plan.classes()[6] < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        Ok(())
    }

    pub(super) fn limits_for(
        source: Domain,
        samples: usize,
        balance: BalanceObserverLimits,
    ) -> Result<ReconstructionObserverLimits, SolverError> {
        if samples == 0 || balance.samples != samples {
            return Err(SolverError::ResourceLimit);
        }
        let fields = source
            .layout()
            .half_len()
            .checked_mul(24)
            .and_then(|n| n.checked_mul(std::mem::size_of::<Complex64>()))
            .ok_or(SolverError::SizeOverflow)?;
        let storage_bytes = balance
            .storage_bytes
            .checked_add(fields)
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)?;
        Ok(ReconstructionObserverLimits {
            storage_bytes,
            samples,
            work_units: balance.work_units,
            scalar_transforms: balance.scalar_transforms,
            modal_visits: source
                .layout()
                .half_len()
                .checked_mul(3)
                .and_then(|n| n.checked_mul(samples))
                .ok_or(SolverError::SizeOverflow)?,
        })
    }

    pub(super) fn new_owned(
        plan: ResourcePlan,
        from_rest: &SpectralState,
        limits: ReconstructionObserverLimits,
        balance: ForceBalance<F>,
    ) -> Result<Self, SolverError> {
        Self::admit(plan, from_rest, limits)?;
        let source = plan.domain();
        let mut result = Self {
            plan,
            source,
            limits,
            balance,
            accepted: [Node::new(source)?, Node::new(source)?, Node::new(source)?],
            accepted_count: 0,
            pending: Node::new(source)?,
            has_pending: false,
            modal_visits: 0,
        };
        result.capture_initial(from_rest)?;
        Ok(result)
    }

    /// Complete constructor admission.
    pub fn limits_declared(&self) -> ReconstructionObserverLimits {
        self.limits
    }
    /// Force/product work charged to attempted observations, including the rest sample.
    pub fn consumption(&self) -> BalanceObserverWork {
        self.balance.consumption()
    }
    /// Charged retained coefficient visits, including a failed derivative computation.
    pub fn modal_visits(&self) -> usize {
        self.modal_visits
    }
    /// Remaining finite observation capacity; rejection does not replenish spent work.
    pub fn remaining_samples(&self) -> usize {
        self.limits.samples - self.balance.consumption().samples
    }

    /// Clocks for the last three accepted macro endpoints, once the ring is full.
    pub fn last_accepted_clocks(&self) -> Option<[TickClock; 3]> {
        if self.accepted_count != 3 {
            return None;
        }
        Some(
            self.accepted
                .each_ref()
                .map(|node| node.clock.expect("accepted node clock")),
        )
    }

    /// Find an exact retained node without reconstructing or exposing mutable storage.
    pub(crate) fn accepted_node(&self, clock: TickClock) -> Option<AcceptedNodeView<'_>> {
        let node = self.accepted[..self.accepted_count]
            .iter()
            .find(|node| node.clock == Some(clock))?;
        Some(AcceptedNodeView {
            domain: self.source,
            clock,
            epoch: node.epoch,
            steps: node.steps,
            value: node.value.each_ref().map(Vec::as_slice),
        })
    }

    /// Reconstruct one component into caller-owned scratch. This allocates no storage.
    pub fn reconstruct(
        &self,
        probe: TickClock,
        axis: usize,
        value: &mut [Complex64],
        derivative: &mut [Complex64],
    ) -> Result<(), SolverError> {
        if axis >= 3 {
            return Err(SolverError::InvalidPayload);
        }
        let clocks = self
            .last_accepted_clocks()
            .ok_or(SolverError::StaleAttempt)?;
        HermiteWeights::at(clocks, probe)?.apply(
            [
                &self.accepted[0].value[axis],
                &self.accepted[1].value[axis],
                &self.accepted[2].value[axis],
                &self.accepted[0].derivative[axis],
                &self.accepted[1].derivative[axis],
                &self.accepted[2].derivative[axis],
            ],
            value,
            derivative,
        )
    }

    fn capture_initial(&mut self, state: &SpectralState) -> Result<(), SolverError> {
        self.balance.sample(state)?;
        self.accepted[0].copy_value(state)?;
        self.charge_modal()?;
        rhs_from_last_balance(&self.balance, state, &mut self.accepted[0].derivative)?;
        self.accepted_count = 1;
        Ok(())
    }

    fn charge_modal(&mut self) -> Result<(), SolverError> {
        let charge = self
            .source
            .layout()
            .half_len()
            .checked_mul(3)
            .ok_or(SolverError::SizeOverflow)?;
        let next = self
            .modal_visits
            .checked_add(charge)
            .ok_or(SolverError::SizeOverflow)?;
        if next > self.limits.modal_visits {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.modal_visits = next;
        Ok(())
    }

    fn admit_endpoint(&self, state: &SpectralState) -> Result<(), SolverError> {
        if self.has_pending || state.plan() != self.plan {
            return Err(SolverError::StaleAttempt);
        }
        let previous = &self.accepted[self.accepted_count - 1];
        let last = previous.clock.ok_or(SolverError::InvalidClock)?;
        if state.epoch() != previous.epoch.next()?
            || state.accepted_steps()
                != previous
                    .steps
                    .checked_add(1)
                    .ok_or(SolverError::EpochExhausted)?
        {
            return Err(SolverError::StaleAttempt);
        }
        let now = state.clock();
        if now.exponent() != last.exponent()
            || now.target() != last.target()
            || now.elapsed() <= last.elapsed()
        {
            return Err(SolverError::InvalidClock);
        }
        if self.accepted_count >= 2 {
            let earlier = self.accepted[self.accepted_count - 2]
                .clock
                .ok_or(SolverError::InvalidClock)?;
            HermiteWeights::at([earlier, last, now], last)?;
        }
        Ok(())
    }
}

impl<F: PrescribedForce> BalanceObserverContract for ReconstructionObserver<F> {
    fn bounds(&self) -> Option<ObserverBounds> {
        Some(ObserverBounds {
            storage_bytes: self.limits.storage_bytes,
            work_units: self.balance.limits_declared().work_units / self.limits.samples,
        })
    }

    fn measure(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError> {
        self.admit_endpoint(state)?;
        let sample = self.balance.sample(state)?;
        self.pending.copy_value(state)?;
        self.charge_modal()?;
        rhs_from_last_balance(&self.balance, state, &mut self.pending.derivative)?;
        self.has_pending = true;
        Ok(sample)
    }

    fn commit_pending(&mut self) {
        if !self.has_pending {
            return;
        }
        if self.accepted_count < 3 {
            std::mem::swap(&mut self.accepted[self.accepted_count], &mut self.pending);
            self.accepted_count += 1;
        } else {
            self.accepted.swap(0, 1);
            self.accepted.swap(1, 2);
            std::mem::swap(&mut self.accepted[2], &mut self.pending);
        }
        self.has_pending = false;
    }

    fn discard_pending(&mut self) {
        self.has_pending = false;
    }
}

fn validate_rest(plan: ResourcePlan, state: &SpectralState) -> Result<(), SolverError> {
    if state.plan() != plan || state.accepted_steps() != 0 || state.clock().elapsed() != 0 {
        return Err(SolverError::InvalidPayload);
    }
    for axis in 0..3 {
        if state
            .component(axis)?
            .iter()
            .any(|value| *value != Complex64::new(0.0, 0.0))
        {
            return Err(SolverError::InvalidPayload);
        }
    }
    Ok(())
}

fn rhs_from_last_balance(
    balance: &ForceBalance<impl PrescribedForce>,
    state: &SpectralState,
    output: &mut Field,
) -> Result<(), SolverError> {
    let source = state.plan().domain();
    for (axis, field) in output.iter_mut().enumerate() {
        transfer(
            balance.diagnostic.layout(),
            source.layout(),
            &balance.conservative[axis],
            field,
        )?;
        add_viscosity(source, state.component(axis)?, field)?;
        validate_spectrum(source.layout(), field, 1e-12)?;
    }
    Ok(())
}

fn add_viscosity(
    source: Domain,
    velocity: &[Complex64],
    output: &mut [Complex64],
) -> Result<(), SolverError> {
    for (index, value) in output.iter_mut().enumerate() {
        let position = source.layout().position(index)?;
        if source.layout().is_nyquist(position)? {
            *value = Complex64::new(0.0, 0.0);
            continue;
        }
        let wave = modal::wavevector(source, source.layout().mode(position)?)?;
        let mut acceleration = -*value;
        for component in wave {
            acceleration -= source.viscosity() * component * (component * velocity[index]);
        }
        if !acceleration.re.is_finite() || !acceleration.im.is_finite() {
            return Err(SolverError::InvalidSpectrum);
        }
        *value = acceleration;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
