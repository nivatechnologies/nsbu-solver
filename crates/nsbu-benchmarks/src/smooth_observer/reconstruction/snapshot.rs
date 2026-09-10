//! Owned accepted-node snapshots; numerical scratch is rebuilt on restoration.
use super::{field, Node, ReconstructionObserver};
use crate::smooth_observer::{BalanceObserver, BalanceObserverWork};
use nsbu_solver::{
    domain::{Domain, ResourcePlan, SpectralState},
    Complex64, SolverError,
};

/// Trusted snapshot of the accepted reconstruction ring and every charged observation.
///
/// Only an observer can construct this value. It excludes scratch and refuses capture while
/// a proposal is pending. Moving it into a restored observer cannot replenish spent work.
#[derive(Debug)]
pub struct ReconstructionSnapshot {
    pub(super) plan: ResourcePlan,
    pub(super) samples: usize,
    pub(super) accepted: [Node; 3],
    pub(super) accepted_count: usize,
    pub(super) work: BalanceObserverWork,
    pub(super) modal_visits: usize,
}

impl ReconstructionObserver {
    /// Fixed snapshot storage for three owned value/derivative nodes, before allocation.
    pub fn snapshot_reservation(domain: Domain) -> Result<usize, SolverError> {
        domain
            .layout()
            .half_len()
            .checked_mul(18)
            .and_then(|n| n.checked_mul(std::mem::size_of::<Complex64>()))
            .and_then(|n| n.checked_add(std::mem::size_of::<ReconstructionSnapshot>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Copy accepted history under a complete allocation cap. Pending proposals are refused.
    pub fn snapshot(&self, cap: usize) -> Result<ReconstructionSnapshot, SolverError> {
        if self.has_pending {
            return Err(SolverError::StaleAttempt);
        }
        if Self::snapshot_reservation(self.source)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        Ok(ReconstructionSnapshot {
            plan: self.plan,
            samples: self.limits.samples,
            accepted: [
                copy_node(&self.accepted[0])?,
                copy_node(&self.accepted[1])?,
                copy_node(&self.accepted[2])?,
            ],
            accepted_count: self.accepted_count,
            work: self.balance.consumption(),
            modal_visits: self.modal_visits,
        })
    }

    /// Restore trusted accepted nodes with fresh force/FFT scratch and unchanged work counters.
    /// The last node must match the supplied physical state, including every coefficient bit.
    pub fn restore(
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
        snapshot: ReconstructionSnapshot,
    ) -> Result<Self, SolverError> {
        snapshot.validate(plan, samples, state)?;
        let limits = Self::limits(plan.domain(), samples)?;
        if plan.classes()[6] < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        Ok(Self {
            plan,
            source: plan.domain(),
            limits,
            balance: BalanceObserver::restore(plan, samples, snapshot.work)?,
            accepted: snapshot.accepted,
            accepted_count: snapshot.accepted_count,
            pending: Node::new(plan.domain())?,
            has_pending: false,
            modal_visits: snapshot.modal_visits,
        })
    }
}

impl ReconstructionSnapshot {
    pub(super) fn validate(
        &self,
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
    ) -> Result<(), SolverError> {
        if self.plan != plan || self.samples != samples || state.plan() != plan {
            return Err(SolverError::InvalidPayload);
        }
        BalanceObserver::validate_restored(plan, samples, self.work)?;
        let nodes = state
            .accepted_steps()
            .checked_add(1)
            .ok_or(SolverError::EpochExhausted)?;
        if self.accepted_count != nodes.min(3) as usize {
            return Err(SolverError::InvalidPayload);
        }
        validate_endpoint(&self.accepted[self.accepted_count - 1], state)?;
        self.validate_modal_work(plan, nodes)
    }

    fn validate_modal_work(&self, plan: ResourcePlan, nodes: u128) -> Result<(), SolverError> {
        let per_sample = plan
            .domain()
            .layout()
            .half_len()
            .checked_mul(3)
            .ok_or(SolverError::SizeOverflow)?;
        let evaluated = self.modal_visits / per_sample;
        if !self.modal_visits.is_multiple_of(per_sample)
            || (evaluated as u128) < nodes
            || evaluated > self.work.samples
        {
            return Err(SolverError::InvalidPayload);
        }
        Ok(())
    }
}

fn validate_endpoint(node: &Node, state: &SpectralState) -> Result<(), SolverError> {
    if node.clock != Some(state.clock())
        || node.epoch != state.epoch()
        || node.steps != state.accepted_steps()
    {
        return Err(SolverError::InvalidPayload);
    }
    for axis in 0..3 {
        if !node.value[axis]
            .iter()
            .zip(state.component(axis)?)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
        {
            return Err(SolverError::InvalidPayload);
        }
    }
    Ok(())
}

fn copy_node(node: &Node) -> Result<Node, SolverError> {
    let mut value = field(node.value[0].len(), Complex64::new(0.0, 0.0))?;
    let mut derivative = field(node.derivative[0].len(), Complex64::new(0.0, 0.0))?;
    for axis in 0..3 {
        value[axis].copy_from_slice(&node.value[axis]);
        derivative[axis].copy_from_slice(&node.derivative[axis]);
    }
    Ok(Node {
        clock: node.clock,
        epoch: node.epoch,
        steps: node.steps,
        value,
        derivative,
    })
}
