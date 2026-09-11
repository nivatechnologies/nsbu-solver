//! Immutable provenance and bitwise findings for two independently owned families.
use crate::v2_run::Origin;
use nsbu_solver::domain::{Epoch, TickClock};

/// Stored provenance for a retained accepted reconstruction node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedNodeProvenance {
    /// Exact accepted clock stored with the node.
    pub clock: TickClock,
    /// Accepted state epoch stored with the node.
    pub epoch: Epoch,
    /// Accepted macro-step count stored with the node.
    pub accepted_steps: u128,
    /// Private reconstructed-family ownership.
    pub origin: Origin,
    /// Bitwise equality of every velocity coefficient with the ordinary family branch.
    pub coefficients_equal: bool,
}

/// Binding result for one fixed family branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeBindingStatus {
    /// The requested accepted node is absent from the ring, including after eviction.
    MissingRetainedNode,
    /// The node was retained with valid provenance and compared completely.
    Compared(AcceptedNodeProvenance),
}

/// One complete six-branch accepted-node binding record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeBindingSample {
    pub(super) clock: TickClock,
    pub(super) family_identity: [u8; 32],
    pub(super) probe_identity: [u8; 32],
    pub(super) branches: [NodeBindingStatus; 6],
}
impl NodeBindingSample {
    /// Requested ordinary-family accepted clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Identity of the ordinary exact-v2 family.
    pub fn family_identity(self) -> [u8; 32] {
        self.family_identity
    }
    /// Identity of the independently owned reconstructed probe family.
    pub fn probe_identity(self) -> [u8; 32] {
        self.probe_identity
    }
    /// Six fixed branch slots in family order.
    pub fn branches(self) -> [NodeBindingStatus; 6] {
        self.branches
    }
}
