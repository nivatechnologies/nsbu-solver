//! Admission of an external reconstruction ring against a separately decoded physical state.
use super::{
    codec::{read_node, Cursor},
    encoded_size, BalanceObserverWork, CheckpointError, Node, ReconstructionObserver,
    ReconstructionSnapshot, ResourcePlan, SolverError, SpectralState, UnverifiedReconstruction,
    HEADER, MAGIC,
};
use nsbu_solver::diagnostics::hermite::HermiteWeights;

pub(super) fn decode(
    bytes: &[u8],
    plan: ResourcePlan,
    samples: usize,
    state: &SpectralState,
    maximum_bytes: usize,
    storage_cap: usize,
) -> Result<UnverifiedReconstruction, CheckpointError> {
    let needed = ReconstructionObserver::snapshot_reservation(plan.domain())
        .map_err(CheckpointError::InvalidHistory)?;
    if needed > storage_cap || bytes.len() > maximum_bytes {
        return Err(CheckpointError::ResourceLimit);
    }
    if state.plan() != plan {
        return Err(CheckpointError::InvalidHistory(SolverError::InvalidPayload));
    }
    let mut c = Cursor(bytes);
    let (count, work, modal_visits) = read_header(&mut c, bytes.len(), plan, samples)?;
    let mut accepted = [node(plan)?, node(plan)?, node(plan)?];
    for item in &mut accepted[..count] {
        read_node(&mut c, plan.domain(), item)?;
    }
    let snapshot = ReconstructionSnapshot {
        plan,
        samples,
        accepted,
        accepted_count: count,
        work,
        modal_visits,
    };
    snapshot
        .validate(plan, samples, state)
        .map_err(CheckpointError::InvalidHistory)?;
    validate_nodes(&snapshot, state)?;
    Ok(UnverifiedReconstruction { snapshot })
}
fn read_header(
    c: &mut Cursor<'_>,
    length: usize,
    plan: ResourcePlan,
    samples: usize,
) -> Result<(usize, BalanceObserverWork, usize), CheckpointError> {
    if length < HEADER {
        return Err(CheckpointError::InvalidEncoding);
    }
    if &c.array::<8>()? != MAGIC || u16::from_le_bytes(c.array()?) != 1 || c.size()? != samples {
        return Err(CheckpointError::InvalidEncoding);
    }
    let count = c.array::<1>()?[0] as usize;
    if length != encoded_size(plan.domain(), count)? {
        return Err(CheckpointError::InvalidEncoding);
    }
    let work = BalanceObserverWork {
        samples: c.size()?,
        work_units: c.size()?,
        scalar_transforms: c.size()?,
    };
    let modal_visits = c.size()?;
    if c.size()? != plan.domain().layout().half_len() {
        return Err(CheckpointError::InvalidEncoding);
    }
    crate::smooth_observer::BalanceObserver::validate_restored(plan, samples, work)
        .map_err(CheckpointError::InvalidHistory)?;
    Ok((count, work, modal_visits))
}

fn node(plan: ResourcePlan) -> Result<Node, CheckpointError> {
    Node::new(plan.domain()).map_err(CheckpointError::InvalidHistory)
}
fn validate_node(
    node: &Node,
    index: usize,
    count: usize,
    state: &SpectralState,
) -> Result<(), CheckpointError> {
    let offset = (count - 1 - index) as u128;
    let expected_steps = state
        .accepted_steps()
        .checked_sub(offset)
        .ok_or(CheckpointError::InvalidEncoding)?;
    let expected_epoch = state
        .epoch()
        .0
        .checked_sub(offset)
        .ok_or(CheckpointError::InvalidEncoding)?;
    let clock = node.clock.ok_or(CheckpointError::InvalidEncoding)?;
    if node.steps != expected_steps
        || node.epoch.0 != expected_epoch
        || clock.exponent() != state.clock().exponent()
        || clock.target() != state.clock().target()
    {
        return Err(CheckpointError::InvalidHistory(SolverError::InvalidPayload));
    }
    if node.steps == 0
        && (clock.elapsed() != 0
            || node
                .value
                .iter()
                .flatten()
                .any(|v| v.re != 0.0 || v.im != 0.0))
    {
        return Err(CheckpointError::InvalidHistory(SolverError::InvalidPayload));
    }
    Ok(())
}

fn validate_nodes(
    snapshot: &ReconstructionSnapshot,
    state: &SpectralState,
) -> Result<(), CheckpointError> {
    for (index, node) in snapshot.accepted[..snapshot.accepted_count]
        .iter()
        .enumerate()
    {
        validate_node(node, index, snapshot.accepted_count, state)?;
    }

    match snapshot.accepted_count {
        3 => {
            let clocks = snapshot
                .accepted
                .each_ref()
                .map(|node| node.clock.expect("decoded clock"));
            HermiteWeights::at(clocks, clocks[1]).map_err(CheckpointError::InvalidHistory)?;
        }
        2 if snapshot.accepted[0].clock.unwrap().elapsed() >= state.clock().elapsed() => {
            return Err(CheckpointError::InvalidHistory(SolverError::InvalidClock))
        }
        _ => {}
    }
    Ok(())
}
