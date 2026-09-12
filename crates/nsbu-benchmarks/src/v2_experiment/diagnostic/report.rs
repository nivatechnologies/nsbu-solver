//! Retained unqualified events with explicit scheduled and missing channels.
use crate::v2_experiment::{
    binding::NodeBindingSample,
    physical::PhysicalRefinementSample,
    pressure::PressureRefinementSample,
    probes::{
        physical::ProbePhysicalSample, pressure::ProbePressureSample,
        reference::ProbeReferenceSample, residuals::ResidualFamilySample, ProbeSample,
    },
    reference::regional::RegionalTrackingSample,
    RefinementSample,
};
use nsbu_solver::domain::TickClock;

/// Evidence channels not supplied by this coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingChannel {
    /// No independently varied force grid is attached.
    ForceResolution,
    /// No higher-precision force evaluation is attached.
    ForcePrecision,
    /// Binary64 arithmetic is not independently varied.
    Arithmetic,
    /// The analytical reference is evaluated only in binary64 here.
    ReferencePrecision,
    /// Pressure has no analytical reference comparison.
    PressureReference,
    /// No pressure-gauge equivalence analysis is attached.
    PressureGauge,
    /// Spectral transfer resolution is not independently varied.
    Transfer,
    /// Physical sampling resolution is not independently varied.
    SamplingResolution,
    /// Quadrature resolution is not independently varied.
    QuadratureResolution,
    /// Sample counts do not establish physical region volumes.
    RegionVolumeCoverage,
}

/// Complete fixed list of evidence channels absent from every event.
pub const MISSING_CHANNELS: [MissingChannel; 10] = [
    MissingChannel::ForceResolution,
    MissingChannel::ForcePrecision,
    MissingChannel::Arithmetic,
    MissingChannel::ReferencePrecision,
    MissingChannel::PressureReference,
    MissingChannel::PressureGauge,
    MissingChannel::Transfer,
    MissingChannel::SamplingResolution,
    MissingChannel::QuadratureResolution,
    MissingChannel::RegionVolumeCoverage,
];

/// Scientific status carried by every event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStatus {
    /// Measurements are retained without a qualification decision.
    UnqualifiedDiagnostic,
}

/// All consumers scheduled only at an ordinary accepted clock.
#[derive(Debug, Clone, Copy)]
pub struct AcceptedDiagnostic {
    /// Full-band velocity and derivative family differences.
    pub spectral: RefinementSample,
    /// Velocity, gradient, Hessian and vorticity physical differences.
    pub physical: PhysicalRefinementSample,
    /// Pressure and pressure-gradient physical differences.
    pub pressure: PressureRefinementSample,
    /// Global and geometric regional analytical-reference errors.
    pub regional_reference: RegionalTrackingSample,
    /// Exact retained-node provenance and bitwise comparisons.
    pub node_binding: NodeBindingSample,
}

/// Accepted-state path selected for a manifest clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptedSchedule {
    /// This manifest clock is scheduled only for an off-stage residual.
    NotScheduledAtResidualClock,
    /// All accepted-state consumers completed.
    Measured,
}

/// Accepted evidence with an explicit schedule marker.
#[derive(Debug, Clone, Copy)]
pub struct AcceptedEvidence {
    schedule: AcceptedSchedule,
    sample: Option<AcceptedDiagnostic>,
}
impl AcceptedEvidence {
    pub(super) fn not_scheduled() -> Self {
        Self {
            schedule: AcceptedSchedule::NotScheduledAtResidualClock,
            sample: None,
        }
    }
    pub(super) fn measured(sample: AcceptedDiagnostic) -> Self {
        Self {
            schedule: AcceptedSchedule::Measured,
            sample: Some(sample),
        }
    }
    /// Explicit accepted-path schedule marker.
    pub fn schedule(self) -> AcceptedSchedule {
        self.schedule
    }
    /// Complete sample only when the accepted path was scheduled.
    pub fn sample(self) -> Option<AcceptedDiagnostic> {
        self.sample
    }
}

/// Residual path selected for a manifest clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualSchedule {
    /// This manifest clock is scheduled only for accepted-state consumers.
    NotScheduledAtAcceptedClock,
    /// The six off-stage residuals completed.
    Measured,
}

/// Residual evidence with an explicit schedule marker.
#[derive(Debug, Clone, Copy)]
pub struct ResidualEvidence {
    schedule: ResidualSchedule,
    sample: Option<ResidualFamilySample>,
}
impl ResidualEvidence {
    pub(super) fn not_scheduled() -> Self {
        Self {
            schedule: ResidualSchedule::NotScheduledAtAcceptedClock,
            sample: None,
        }
    }
    pub(super) fn measured(sample: ResidualFamilySample) -> Self {
        Self {
            schedule: ResidualSchedule::Measured,
            sample: Some(sample),
        }
    }
    /// Explicit residual-path schedule marker.
    pub fn schedule(self) -> ResidualSchedule {
        self.schedule
    }
    /// Complete sample only when the residual path was scheduled.
    pub fn sample(self) -> Option<ResidualFamilySample> {
        self.sample
    }
}

/// One transactionally retained manifest event.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticEvent {
    pub(super) clock: TickClock,
    pub(super) family_identity: [u8; 32],
    pub(super) probe_identity: [u8; 32],
    pub(super) probe: ProbeSample,
    pub(super) probe_physical: ProbePhysicalSample,
    pub(super) probe_pressure: ProbePressureSample,
    pub(super) probe_reference: ProbeReferenceSample,
    pub(super) accepted: AcceptedEvidence,
    pub(super) residual: ResidualEvidence,
}
impl DiagnosticEvent {
    /// Exact requested manifest clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Immutable ordinary-family identity.
    pub fn family_identity(self) -> [u8; 32] {
        self.family_identity
    }
    /// Immutable probe-family and complete-manifest identity.
    pub fn probe_identity(self) -> [u8; 32] {
        self.probe_identity
    }
    /// Reconstructed value and derivative comparisons at this clock.
    pub fn probe(self) -> ProbeSample {
        self.probe
    }
    /// Physical comparisons of the six reconstructed value fields at this clock.
    pub fn reconstructed_physical(self) -> ProbePhysicalSample {
        self.probe_physical
    }
    /// Reconstructed pressure and pressure-gradient comparisons at this exact clock.
    pub fn reconstructed_pressure(self) -> ProbePressureSample {
        self.probe_pressure
    }
    /// Analytical tracking of all six reconstructed value fields at this exact clock.
    pub fn reconstructed_reference(self) -> ProbeReferenceSample {
        self.probe_reference
    }
    /// Accepted-state evidence or its schedule marker.
    pub fn accepted(self) -> AcceptedEvidence {
        self.accepted
    }
    /// Residual evidence or its schedule marker.
    pub fn residual(self) -> ResidualEvidence {
        self.residual
    }
    /// This driver never assigns a qualification outcome.
    pub fn status(self) -> DiagnosticStatus {
        DiagnosticStatus::UnqualifiedDiagnostic
    }
    /// Fixed unresolved evidence channels.
    pub fn missing_channels(self) -> &'static [MissingChannel; 10] {
        &MISSING_CHANNELS
    }
}
