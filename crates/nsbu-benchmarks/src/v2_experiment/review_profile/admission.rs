//! Fail-closed benchmark admission over the generic frozen verification protocol.
use super::{
    semantics_identity, ObservableDescriptor, ReviewGeometry, OBSERVABLES, OBSERVABLE_COUNT,
    PROBLEM_IDENTITY,
};
use nsbu_solver::{
    domain::TickClock,
    verification::{
        policy::{ObservablePolicy, Policies},
        protocol::{FrozenProtocol, ProtocolInputs},
        review::{required_records, MeasurementReview, ReviewStatus},
        VerificationError,
    },
};

/// Caller-provided identities and numerical policies; no benchmark budget is implied.
#[derive(Debug, Clone, Copy)]
pub struct ProfileInputs<'a> {
    /// Raw mathematical problem digest; must equal the published exact-v2 identity.
    pub problem: [u8; 32],
    /// Canonical typed inventory and mandatory-gap digest.
    pub semantics: [u8; 32],
    /// Caller-owned numerical budgets in exact [`super::OBSERVABLES`] order.
    pub policies: &'a [ObservablePolicy],
}

/// Finite admission and execution caps, separate from scientific semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileCaps {
    /// Maximum duplicate-key comparisons during generic policy admission.
    pub policy_pair_checks: usize,
    /// Maximum canonical generic-protocol bytes.
    pub protocol_bytes: usize,
    /// Maximum required schedule rows.
    pub required_rows: usize,
    /// Bounded malformed-plus-valid record attempts for the later review.
    pub review_attempts: usize,
}

/// Exact resource and traversal requirements of one admitted profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileBounds {
    /// Borrowed policy slice size, excluding caller allocation overhead.
    pub borrowed_policy_bytes: usize,
    /// Exact generic-protocol canonical byte count.
    pub protocol_bytes: usize,
    /// Exact canonical semantics hash traversal byte count.
    pub semantics_bytes: usize,
    /// Complete bounded duplicate-key comparisons.
    pub policy_pair_checks: usize,
    /// Fine-clock by observable Cartesian schedule size.
    pub required_rows: usize,
    /// Admitted later-review attempt allowance.
    pub review_attempts: usize,
    /// Identity-bound mandatory semantics without scalar producers.
    pub mandatory_gaps: usize,
}

/// One required fine-clock-major row; it contains no measurement value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredRow {
    /// Zero-based fine-clock-major schedule position.
    pub index: usize,
    /// Exact required fine-manifest clock.
    pub clock: TickClock,
    /// Typed scalar observable required at this position.
    pub observable: ObservableDescriptor,
}

/// This layer admits only a schedule and semantics, never numerical readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileStatus {
    /// Scalar rows and named mandatory gaps remain unpopulated and unqualified.
    PartialUnpopulatedDiagnostic,
}

/// Benchmark-specific refusal while preserving generic verification causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    /// Mathematical problem digest differs from exact-v2.
    InvalidProblem,
    /// Typed inventory or mandatory-gap digest differs.
    InvalidSemantics,
    /// Policy count, order or stable keys differ.
    InvalidInventory,
    /// Source probe plan does not carry the frozen manifest/branch geometry.
    InvalidGeometry,
    /// A caller cap cannot cover complete admission or review work.
    CapacityExceeded,
    /// Exact-clock construction failed.
    Geometry(nsbu_solver::SolverError),
    /// The unchanged generic protocol refused policy or reconstruction inputs.
    Verification(VerificationError),
}
impl From<nsbu_solver::SolverError> for ProfileError {
    fn from(error: nsbu_solver::SolverError) -> Self {
        Self::Geometry(error)
    }
}
impl From<VerificationError> for ProfileError {
    fn from(error: VerificationError) -> Self {
        Self::Verification(error)
    }
}

/// Borrowed admitted profile backed by the unchanged generic protocol implementation.
#[derive(Debug, Clone, Copy)]
pub struct AdmittedProfile<'a> {
    protocol: FrozenProtocol<'a>,
    bounds: ProfileBounds,
    family_identity: [u8; 32],
    probe_identity: [u8; 32],
}
impl<'a> AdmittedProfile<'a> {
    /// Validate benchmark identities/inventory, then delegate all generic geometry admission.
    pub fn new(
        geometry: &'a ReviewGeometry,
        inputs: ProfileInputs<'a>,
        caps: ProfileCaps,
    ) -> Result<Self, ProfileError> {
        if inputs.problem != PROBLEM_IDENTITY {
            return Err(ProfileError::InvalidProblem);
        }
        if inputs.semantics != semantics_identity() {
            return Err(ProfileError::InvalidSemantics);
        }
        require_inventory(inputs.policies)?;
        let pair_checks = OBSERVABLE_COUNT
            .checked_mul(OBSERVABLE_COUNT - 1)
            .and_then(|value| value.checked_div(2))
            .ok_or(ProfileError::CapacityExceeded)?;
        let rows = required_records(OBSERVABLE_COUNT, geometry.fine().len())?;
        if caps.policy_pair_checks < pair_checks
            || caps.required_rows < rows
            || caps.review_attempts < rows
        {
            return Err(ProfileError::CapacityExceeded);
        }
        let policies = Policies::new(inputs.policies, caps.policy_pair_checks)?;
        let protocol = FrozenProtocol::new(
            ProtocolInputs {
                problem: inputs.problem,
                semantics: inputs.semantics,
                policies,
                time_sets: geometry.time_sets()?,
                reconstruction: geometry.reconstruction()?,
            },
            caps.protocol_bytes,
        )?;
        let bounds = ProfileBounds {
            borrowed_policy_bytes: std::mem::size_of_val(inputs.policies),
            protocol_bytes: protocol.encoded_bytes(),
            semantics_bytes: super::SEMANTICS_BYTES,
            policy_pair_checks: pair_checks,
            required_rows: rows,
            review_attempts: caps.review_attempts,
            mandatory_gaps: super::MANDATORY_GAPS.len(),
        };
        Ok(Self {
            protocol,
            bounds,
            family_identity: geometry.family_identity(),
            probe_identity: geometry.probe_identity(),
        })
    }
    /// Generic frozen-protocol identity including caller-supplied budgets.
    pub fn identity(self) -> [u8; 32] {
        self.protocol.identity()
    }
    /// Canonical typed inventory and explicit mandatory-gap identity.
    pub fn semantics_identity(self) -> [u8; 32] {
        self.protocol.inputs().semantics
    }
    /// Ordinary family identity from which reconstruction geometry was derived.
    pub fn family_identity(self) -> [u8; 32] {
        self.family_identity
    }
    /// Probe plan identity carrying the exact manifest and actual node rules.
    pub fn probe_identity(self) -> [u8; 32] {
        self.probe_identity
    }
    /// Fixed partial admission status; no observation was created.
    pub fn status(self) -> ProfileStatus {
        ProfileStatus::PartialUnpopulatedDiagnostic
    }
    /// Complete fixed-storage and finite traversal declaration.
    pub fn bounds(self) -> ProfileBounds {
        self.bounds
    }
    /// Return one immutable schedule row without a measurement value.
    pub fn required_row(self, index: usize) -> Option<RequiredRow> {
        let policies = self.protocol.inputs().policies.as_slice();
        if index >= self.bounds.required_rows {
            return None;
        }
        let observable = OBSERVABLES[index % policies.len()];
        Some(RequiredRow {
            index,
            clock: self.protocol.inputs().time_sets[2].as_slice()[index / policies.len()],
            observable,
        })
    }
    /// Delegate canonical protocol serialization into caller-owned storage.
    pub fn write_canonical(self, output: &mut [u8]) -> Result<usize, ProfileError> {
        self.protocol.write_canonical(output).map_err(Into::into)
    }
    /// Create the generic empty numerical review with the admitted attempt bound.
    pub fn review(self) -> Result<MeasurementReview<'a>, ProfileError> {
        let review = self.protocol.review(self.bounds.review_attempts)?;
        debug_assert_eq!(review.status(), ReviewStatus::Incomplete);
        Ok(review)
    }
}

fn require_inventory(policies: &[ObservablePolicy]) -> Result<(), ProfileError> {
    if policies.len() != OBSERVABLE_COUNT
        || policies
            .iter()
            .zip(OBSERVABLES)
            .any(|(policy, observable)| policy.key != observable.key)
    {
        return Err(ProfileError::InvalidInventory);
    }
    Ok(())
}
