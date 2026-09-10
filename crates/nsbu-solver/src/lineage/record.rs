use super::LineageError;
use crate::domain::TickClock;

/// Nonmissing external content identity; constructing it does not verify the content's hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digest([u8; 32]);
impl Digest {
    /// Refuse the reserved missing-identity sentinel.
    pub fn new(bytes: [u8; 32]) -> Result<Self, LineageError> {
        if bytes == [0; 32] {
            return Err(LineageError::MissingIdentity);
        }
        Ok(Self(bytes))
    }
    /// Exact supplied digest for serialization and external content verification.
    pub fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Mathematical identity and separately identified numerical artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// Immutable equations, parameters, physical domain and initial data.
    pub problem: Digest,
    /// Grid, layout, method, arithmetic, controller and execution settings.
    pub execution: Digest,
    /// Prescribed-force implementation, precision and certified coverage manifest.
    pub force: Digest,
}
impl Profile {
    /// Different execution/force artifacts may represent the same mathematical problem.
    pub fn same_problem(self, other: Self) -> bool {
        self.problem == other.problem
    }
}

/// An append-only position scoped to an externally unique registry identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordId {
    pub(super) registry: Digest,
    pub(super) index: usize,
}

/// Declared ancestry; none of these labels constitutes a qualified PDE window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A new branch declared to have begun at exact zero with zero physical state.
    FromRest,
    /// A reference-seeded local test, excluded from the from-rest frontier.
    LocalReference,
    /// A same-profile continuation of an existing branch.
    Continued(RecordId),
    /// A restart transfer with a retained inherited-error report identity.
    Transferred {
        /// Full earlier history, including any invalidated force dependencies.
        parent: RecordId,
        /// Complete error report with observable units and transfer/inherited contributions.
        errors: Digest,
    },
}
impl Origin {
    pub(super) fn parent(self) -> Option<RecordId> {
        match self {
            Self::Continued(id) | Self::Transferred { parent: id, .. } => Some(id),
            _ => None,
        }
    }
}

/// Immutable declaration plus monotone invalidation; no accepted status can be imported here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub(super) profile: Profile,
    pub(super) clock: TickClock,
    pub(super) origin: Origin,
    pub(super) direct: bool,
    pub(super) valid: bool,
}
impl Record {
    /// Declared mathematical and numerical identities.
    pub fn profile(self) -> Profile {
        self.profile
    }
    /// Exact endpoint represented by this declaration.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Immediate ancestry, retaining the transfer error report where present.
    pub fn origin(self) -> Origin {
        self.origin
    }
    /// Direct from-rest ancestry only; actual state provenance still requires validation.
    pub fn is_direct(self) -> bool {
        self.direct
    }
    /// False after this record or an ancestor is invalidated; never numerical acceptance.
    pub fn is_valid(self) -> bool {
        self.valid
    }
}
