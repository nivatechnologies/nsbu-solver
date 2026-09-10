//! Preallocated accepted-node storage for a future transaction-integrated Hermite reconstruction.
use super::hermite::HermiteWeights;
use crate::{
    domain::{validate_spectrum, Epoch, ResourcePlan, SpectralState, TickClock},
    storage::field,
    Complex64, SolverError,
};

const SPECTRUM_TOLERANCE: f64 = 1e-12;

/// One caller-prepared accepted node and its physical-time Fourier derivative.
#[derive(Clone, Copy)]
pub struct ReconstructionNode<'a> {
    state: &'a SpectralState,
    derivative: [&'a [Complex64]; 3],
}

impl<'a> ReconstructionNode<'a> {
    /// Borrow a state and three same-layout derivative components without taking ownership.
    pub fn new(state: &'a SpectralState, derivative: [&'a [Complex64]; 3]) -> Self {
        Self { state, derivative }
    }
}

/// A non-cloneable pending segment identity, valid only for its originating history instance.
#[derive(Debug)]
pub struct StagedSegment {
    owner: usize,
    generation: u128,
}

#[derive(Debug)]
struct Slot {
    clock: Option<TickClock>,
    epoch: Epoch,
    accepted_steps: u128,
    value: [Vec<Complex64>; 3],
    derivative: [Vec<Complex64>; 3],
}

/// Three accepted Hermite nodes plus separately owned pending storage.
///
/// This does not obtain the accepted half-step from an integrator. A transaction adapter must
/// prepare all three actual accepted nodes, stage them before commit, then publish only after
/// the endpoint physical commit succeeds. The caller supplies the exact equally spaced span;
/// consecutive accepted macro endpoints are valid when they meet that geometry.
#[derive(Debug)]
pub struct ReconstructionHistory {
    plan: ResourcePlan,
    accepted: [Slot; 3],
    pending: [Slot; 3],
    has_accepted: bool,
    has_pending: bool,
    owner: usize,
    generation: u128,
}

impl ReconstructionHistory {
    /// Reservation for six value/derivative node slots and inline metadata.
    pub fn reservation(plan: ResourcePlan) -> Result<usize, SolverError> {
        plan.domain()
            .layout()
            .half_len()
            .checked_mul(36 * std::mem::size_of::<Complex64>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Allocate all accepted and pending storage before any staging operation.
    pub fn new(plan: ResourcePlan, storage_cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(plan)? > storage_cap {
            return Err(SolverError::ResourceLimit);
        }
        let accepted = slots(plan)?;
        let owner = accepted[0].value[0].as_ptr() as usize;
        Ok(Self {
            plan,
            accepted,
            pending: slots(plan)?,
            has_accepted: false,
            has_pending: false,
            owner,
            generation: 0,
        })
    }

    /// True only after a segment has passed post-physical-commit publication.
    pub fn has_segment(&self) -> bool {
        self.has_accepted
    }

    /// Exact clocks of the published start, midpoint and endpoint, when available.
    pub fn clocks(&self) -> Option<[TickClock; 3]> {
        if !self.has_accepted {
            return None;
        }
        Some([
            self.accepted[0].clock?,
            self.accepted[1].clock?,
            self.accepted[2].clock?,
        ])
    }

    /// Validate and copy a caller-supplied accepted start/midpoint/endpoint segment to pending.
    /// No accepted storage changes on refusal. All buffers were allocated by [`Self::new`].
    pub fn stage(
        &mut self,
        nodes: [ReconstructionNode<'_>; 3],
    ) -> Result<StagedSegment, SolverError> {
        if self.has_pending {
            return Err(SolverError::StaleAttempt);
        }
        validate_nodes(self.plan, nodes)?;
        if self.has_accepted && !matches_slot(&self.accepted[2], nodes[0]) {
            return Err(SolverError::StaleAttempt);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(SolverError::EpochExhausted)?;
        for (destination, source) in self.pending.iter_mut().zip(nodes) {
            copy_slot(destination, source);
        }
        self.has_pending = true;
        self.generation = generation;
        Ok(StagedSegment {
            owner: self.owner,
            generation,
        })
    }

    /// Drop a staged proposal after a rejected or otherwise uncommitted physical attempt.
    pub fn discard(&mut self, staged: &StagedSegment) -> Result<(), SolverError> {
        self.validate_token(staged)?;
        self.has_pending = false;
        Ok(())
    }

    /// Publish only if `committed_endpoint` is bit-identical to the staged endpoint after commit.
    /// The caller must invoke this after the transaction's successful physical exchange. Equality
    /// of state metadata and coefficients is necessary evidence, not a provenance proof; an
    /// integrated transaction hook remains responsible for that atomicity.
    pub fn publish_after_commit(
        &mut self,
        committed_endpoint: &SpectralState,
        staged: &StagedSegment,
    ) -> Result<(), SolverError> {
        self.validate_token(staged)?;
        if !matches_slot(
            &self.pending[2],
            ReconstructionNode::new(
                committed_endpoint,
                [
                    &self.pending[2].derivative[0],
                    &self.pending[2].derivative[1],
                    &self.pending[2].derivative[2],
                ],
            ),
        ) {
            return Err(SolverError::StaleAttempt);
        }
        std::mem::swap(&mut self.accepted, &mut self.pending);
        self.has_accepted = true;
        self.has_pending = false;
        Ok(())
    }

    /// Apply the existing quintic Hermite reconstruction to one published Fourier component.
    pub fn reconstruct(
        &self,
        probe: TickClock,
        axis: usize,
        value: &mut [Complex64],
        derivative: &mut [Complex64],
    ) -> Result<(), SolverError> {
        if !self.has_accepted || axis >= 3 {
            return Err(SolverError::InvalidPayload);
        }
        let clocks = self.clocks().ok_or(SolverError::InvalidPayload)?;
        let weights = HermiteWeights::at(clocks, probe)?;
        weights.apply(
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

    fn validate_token(&self, staged: &StagedSegment) -> Result<(), SolverError> {
        if !self.has_pending || staged.owner != self.owner || staged.generation != self.generation {
            return Err(SolverError::StaleAttempt);
        }
        Ok(())
    }
}

fn slots(plan: ResourcePlan) -> Result<[Slot; 3], SolverError> {
    Ok([slot(plan)?, slot(plan)?, slot(plan)?])
}

fn slot(plan: ResourcePlan) -> Result<Slot, SolverError> {
    Ok(Slot {
        clock: None,
        epoch: Epoch(0),
        accepted_steps: 0,
        value: field(plan.domain().layout().half_len())?,
        derivative: field(plan.domain().layout().half_len())?,
    })
}

fn validate_nodes(
    plan: ResourcePlan,
    nodes: [ReconstructionNode<'_>; 3],
) -> Result<(), SolverError> {
    let clocks = nodes.map(|node| node.state.clock());
    HermiteWeights::at(clocks, clocks[1])?;
    for node in nodes {
        if node.state.plan() != plan
            || node
                .derivative
                .iter()
                .any(|field| field.len() != plan.domain().layout().half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        for axis in 0..3 {
            validate_spectrum(
                plan.domain().layout(),
                node.state.component(axis)?,
                SPECTRUM_TOLERANCE,
            )?;
            validate_spectrum(
                plan.domain().layout(),
                node.derivative[axis],
                SPECTRUM_TOLERANCE,
            )?;
        }
    }
    if !(nodes[0].state.epoch().0 < nodes[1].state.epoch().0
        && nodes[1].state.epoch().0 < nodes[2].state.epoch().0
        && nodes[0].state.accepted_steps() < nodes[1].state.accepted_steps()
        && nodes[1].state.accepted_steps() < nodes[2].state.accepted_steps())
    {
        return Err(SolverError::StaleAttempt);
    }
    Ok(())
}

fn copy_slot(destination: &mut Slot, source: ReconstructionNode<'_>) {
    destination.clock = Some(source.state.clock());
    destination.epoch = source.state.epoch();
    destination.accepted_steps = source.state.accepted_steps();
    for axis in 0..3 {
        destination.value[axis].copy_from_slice(source.state.component(axis).unwrap());
        destination.derivative[axis].copy_from_slice(source.derivative[axis]);
    }
}

fn matches_slot(slot: &Slot, node: ReconstructionNode<'_>) -> bool {
    slot.clock == Some(node.state.clock())
        && slot.epoch == node.state.epoch()
        && slot.accepted_steps == node.state.accepted_steps()
        && (0..3).all(|axis| {
            equal_bits(&slot.value[axis], &node.state.components[axis])
                && equal_bits(&slot.derivative[axis], node.derivative[axis])
        })
}

fn equal_bits(left: &[Complex64], right: &[Complex64]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.re.to_bits() == right.re.to_bits() && left.im.to_bits() == right.im.to_bits()
        })
}
