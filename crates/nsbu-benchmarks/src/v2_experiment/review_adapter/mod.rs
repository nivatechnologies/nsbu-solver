//! Fail-closed extraction for the current partial exact-v2 observable inventory.
mod validate;
use self::validate::validate;
use super::{
    diagnostic::{DiagnosticEvent, DiagnosticPlan, MissingChannel, MISSING_CHANNELS},
    probes::residuals::ResidualFamilySample,
};
use crate::CASE_SHA256;
use nsbu_solver::{
    diagnostics::physical::PhysicalQuantity,
    domain::{Layout, TickClock},
    verification::{reconstruction::ProbeRefinement, refinement::Evidence},
};

/// Finest-step CM branch in the source-derived six-branch V2 family layout.
pub const FINEST_CM_BRANCH: usize = 2;
/// Four observables at every clock in the seven-event startup manifest.
pub const REVIEW_RECORDS: usize = 28;

/// Physical units attached to one complete global RMS observable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservableUnits {
    /// Velocity units.
    Velocity,
    /// Velocity divided by length.
    VelocityPerLength,
    /// Velocity divided by squared length.
    VelocityPerLengthSquared,
}

/// Stable partial-inventory identity for one physical quantity and RMS definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalRmsObservable {
    /// Stable extraction key.
    pub key: u32,
    /// Complete vector or ordered tensor quantity.
    pub quantity: PhysicalQuantity,
    /// Quantity units.
    pub units: ObservableUnits,
}

/// Fixed partial inventory in velocity, gradient, ordered-Hessian and vorticity order.
pub const OBSERVABLES: [GlobalRmsObservable; 4] = [
    GlobalRmsObservable {
        key: 0x5632_0101,
        quantity: PhysicalQuantity::Vector,
        units: ObservableUnits::Velocity,
    },
    GlobalRmsObservable {
        key: 0x5632_0102,
        quantity: PhysicalQuantity::Gradient,
        units: ObservableUnits::VelocityPerLength,
    },
    GlobalRmsObservable {
        key: 0x5632_0103,
        quantity: PhysicalQuantity::Hessian,
        units: ObservableUnits::VelocityPerLengthSquared,
    },
    GlobalRmsObservable {
        key: 0x5632_0104,
        quantity: PhysicalQuantity::Vorticity,
        units: ObservableUnits::VelocityPerLength,
    },
];

/// Explicit physical sampling semantics that must match every accepted report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewProfile {
    /// Lattice used for family physical comparisons.
    pub physical_samples: Layout,
    /// Lattice used for analytical tracking.
    pub tracking_samples: Layout,
    /// Shared physical/tracking floors for this fixed startup profile. Extraction
    /// fails closed if the two independently declared report arrays differ.
    pub relative_floors: [f64; 4],
}

/// Whether an accepted-state observable was scheduled at this manifest clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordAvailability {
    /// Actual accepted-state physical and tracking reports supplied the record.
    AcceptedMeasured,
    /// Physical differences came from actual reconstructed values at an off-stage clock.
    OffstagePhysicalMeasured,
}

/// Raw typed evidence retained without applying a numerical policy.
#[derive(Debug, Clone, Copy)]
pub struct AdapterRecord {
    /// Exact manifest clock.
    pub clock: TickClock,
    /// Typed observable and its units.
    pub observable: GlobalRmsObservable,
    /// Accepted or off-stage schedule classification.
    pub availability: RecordAvailability,
    /// Finest-CM sampled analytical discrepancy at this actual event clock.
    pub tracking_error: Option<f64>,
    /// Every generic channel in public `CHANNELS` order; eight remain missing when accepted.
    pub channels: [Evidence; 11],
    /// Actual nested reconstruction geometry on residual clocks only.
    pub reconstruction: Option<ProbeRefinement>,
}

/// This slice is permanently explicit about its incomplete, unqualified inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterStatus {
    /// No numerical-review or PDE-window status is implied.
    PartialUnqualifiedInventory,
}

/// Observable groups outside this deliberately narrow four-global-RMS slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingObservableGroup {
    /// Pressure tracking and gauge-equivalence observables.
    PressureAndGauge,
    /// Off-stage momentum residual as its own quantity and units.
    OffstageResidual,
    /// Region-specific tracking quantities and volume interpretation.
    RegionalTracking,
    /// Balance identities and their quadrature refinements.
    BalanceAndQuadrature,
    /// Peak and relative measures if the frozen benchmark requires them.
    PeakAndRelativeMeasures,
}

/// Known observable groups omitted from this partial extraction.
pub const MISSING_OBSERVABLE_GROUPS: [MissingObservableGroup; 5] = [
    MissingObservableGroup::PressureAndGauge,
    MissingObservableGroup::OffstageResidual,
    MissingObservableGroup::RegionalTracking,
    MissingObservableGroup::BalanceAndQuadrature,
    MissingObservableGroup::PeakAndRelativeMeasures,
];

/// Completed bounded extraction audit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterResult {
    /// Always partial and unqualified for this slice.
    pub status: AdapterStatus,
    /// Public exact-v2 problem definition hash.
    pub case_sha256: &'static str,
    /// Exact layouts and binary64 floor words checked before extraction.
    pub profile: ReviewProfile,
    /// Exact number of transactionally published records.
    pub records: usize,
    /// Ordinary-family identity checked on every event and accepted child.
    pub family_identity: [u8; 32],
    /// Probe-family/manifest identity checked on every event and residual child.
    pub probe_identity: [u8; 32],
    /// Coordinator-declared unavailable channels; this is not an observable inventory.
    pub missing_channels: [MissingChannel; 10],
    /// Observable groups deliberately absent from the four-global-RMS inventory.
    pub missing_observable_groups: [MissingObservableGroup; 5],
}

/// Fixed adapter storage and traversal declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterBounds {
    /// Required caller-owned record-array bytes, excluding call/report copies and process storage.
    pub caller_output_bytes: usize,
    /// Transactional record-array bytes, excluding stack frames, report copies and process storage.
    pub transactional_bytes: usize,
    /// Complete manifest event validations.
    pub event_scans: usize,
    /// Exact typed evidence records.
    pub records: usize,
}
impl AdapterBounds {
    /// Exact fixed bounds for the seven-event, four-observable slice.
    pub const fn fixed() -> Self {
        let bytes = std::mem::size_of::<[Option<AdapterRecord>; REVIEW_RECORDS]>();
        Self {
            caller_output_bytes: bytes,
            transactional_bytes: bytes,
            event_scans: 7,
            records: REVIEW_RECORDS,
        }
    }
}

/// Fail-closed extraction error; caller output remains unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterError {
    /// Profile, identity, schedule or report semantics do not match.
    InvalidInput,
    /// Caller limits cannot cover the fixed extraction.
    CapacityExceeded,
}

/// Validate and extract the complete known-partial inventory without applying tolerances.
pub fn extract(
    plan: DiagnosticPlan<'_>,
    profile: ReviewProfile,
    events: &[DiagnosticEvent],
    maximum_records: usize,
    maximum_transactional_bytes: usize,
    output: &mut [Option<AdapterRecord>; REVIEW_RECORDS],
) -> Result<AdapterResult, AdapterError> {
    let bounds = AdapterBounds::fixed();
    if maximum_records < bounds.records || maximum_transactional_bytes < bounds.transactional_bytes
    {
        return Err(AdapterError::CapacityExceeded);
    }
    validate(plan, profile, events)?;
    let mut pending = [None; REVIEW_RECORDS];
    let mut cursor = 0;
    for event in events {
        for (quantity, observable) in OBSERVABLES.into_iter().enumerate() {
            let (availability, tracking_error, channels, reconstruction) = map(*event, quantity);
            pending[cursor] = Some(AdapterRecord {
                clock: event.clock(),
                observable,
                availability,
                tracking_error,
                channels,
                reconstruction,
            });
            cursor += 1;
        }
    }
    if cursor != REVIEW_RECORDS {
        return Err(AdapterError::InvalidInput);
    }
    *output = pending;
    Ok(AdapterResult {
        status: AdapterStatus::PartialUnqualifiedInventory,
        case_sha256: CASE_SHA256,
        profile,
        records: cursor,
        family_identity: plan.family_plan().identity(),
        probe_identity: plan.probe_plan().identity(),
        missing_channels: MISSING_CHANNELS,
        missing_observable_groups: MISSING_OBSERVABLE_GROUPS,
    })
}

type Mapped = (
    RecordAvailability,
    Option<f64>,
    [Evidence; 11],
    Option<ProbeRefinement>,
);

fn map(event: DiagnosticEvent, quantity: usize) -> Mapped {
    use nsbu_solver::verification::budget::Channel;
    let Some(accepted) = event.accepted().sample() else {
        let residual: ResidualFamilySample = event.residual().sample().expect("validated residual");
        let physical = event.reconstructed_physical().quantities()[quantity];
        let tracking =
            event.reconstructed_reference().branches()[FINEST_CM_BRANCH].quantities[quantity];
        let mut channels = [Evidence::Missing; 11];
        channels[Channel::Space as usize] =
            Evidence::Sequence([physical.pairs[0].rms_error, physical.pairs[1].rms_error]);
        channels[Channel::Time as usize] =
            Evidence::Sequence([physical.pairs[2].rms_error, physical.pairs[3].rms_error]);
        channels[Channel::Method as usize] = Evidence::Pair(physical.pairs[4].rms_error);
        return (
            RecordAvailability::OffstagePhysicalMeasured,
            Some(tracking.error.rms_error),
            channels,
            Some(residual.temporal_geometry()),
        );
    };
    let physical = accepted.physical.quantities()[quantity];
    let tracking = accepted.regional_reference.branches()[FINEST_CM_BRANCH].quantities[quantity];
    let mut channels = [Evidence::Missing; 11];
    channels[Channel::Space as usize] =
        Evidence::Sequence(physical.space.map(|value| value.rms_error));
    channels[Channel::Time as usize] =
        Evidence::Sequence(physical.time.map(|value| value.rms_error));
    channels[Channel::Method as usize] = Evidence::Pair(physical.method.rms_error);
    (
        RecordAvailability::AcceptedMeasured,
        Some(tracking.global.rms_error),
        channels,
        None,
    )
}
