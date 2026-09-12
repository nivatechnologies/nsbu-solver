//! Bounded unqualified diagnostic coordination over independent exact-v2 families.
pub mod export;
mod plan;
mod profile;
mod report;
use crate::v2_experiment::{
    binding::{NodeBindingError, NodeBindingWorkspace},
    physical::PhysicalFamilyWorkspace,
    pressure::PressureFamilyWorkspace,
    probes::{physical::ProbePhysicalWorkspace, pressure::ProbePressureWorkspace, residuals::ResidualFamily, ProbeFamily},
    reference::regional::{RegionalTrackingError, RegionalTrackingWorkspace},
    FamilyError, V2Family,
};
use nsbu_solver::{domain::TickClock, SolverError};
pub use plan::{DiagnosticBounds, DiagnosticPlan, DiagnosticSettings, DiagnosticWork};
pub use profile::StartupProfile;
pub use report::{
    AcceptedDiagnostic, AcceptedEvidence, AcceptedSchedule, DiagnosticEvent, DiagnosticStatus,
    MissingChannel, ResidualEvidence, ResidualSchedule, MISSING_CHANNELS,
};

/// Actual work charged by each attached consumer, including failed requests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiagnosticConsumerWork {
    /// Probe-family reconstruction and comparison work.
    pub probes: crate::v2_experiment::probes::ProbeWork,
    /// Accepted-state physical comparison work.
    pub physical: crate::v2_experiment::physical::PhysicalFamilyWork,
    /// Reconstructed-value physical comparison work at every manifest clock.
    pub probe_physical: crate::v2_experiment::probes::physical::ProbePhysicalWork,
    /// Reconstructed pressure construction and comparison work at every manifest clock.
    pub probe_pressure: crate::v2_experiment::probes::pressure::ProbePressureWork,
    /// Accepted-state pressure construction and comparison work.
    pub pressure: crate::v2_experiment::pressure::PressureFamilyWork,
    /// Analytical reference evaluation and reduction work.
    pub reference: crate::v2_experiment::reference::ReferenceTrackingWork,
    /// Added regional classification and reduction work.
    pub regional: crate::v2_experiment::reference::regional::RegionalTrackingWork,
    /// Off-stage doubled-band residual work.
    pub residual: crate::v2_experiment::probes::residuals::ResidualFamilyWork,
    /// Accepted-node provenance and bitwise comparison work.
    pub binding: crate::v2_experiment::binding::NodeBindingWork,
}

/// Terminal coordinator failures preserve the child origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticError {
    /// Ordinary or probe family and consumer failure.
    Family(FamilyError),
    /// Regional analytical tracking failure.
    Regional(RegionalTrackingError),
    /// Accepted-node provenance or comparison failure.
    Binding(NodeBindingError),
    /// Allocation or coordinator resource failure.
    Numerical(SolverError),
    /// A previous event attempt failed.
    Terminated,
}
impl From<FamilyError> for DiagnosticError {
    fn from(value: FamilyError) -> Self {
        Self::Family(value)
    }
}
impl From<RegionalTrackingError> for DiagnosticError {
    fn from(value: RegionalTrackingError) -> Self {
        Self::Regional(value)
    }
}
impl From<NodeBindingError> for DiagnosticError {
    fn from(value: NodeBindingError) -> Self {
        Self::Binding(value)
    }
}
impl From<SolverError> for DiagnosticError {
    fn from(value: SolverError) -> Self {
        Self::Numerical(value)
    }
}

/// Owner of two families, every consumer and immutable retained events.
pub struct DiagnosticDriver<'a> {
    plan: DiagnosticPlan<'a>,
    ordinary: V2Family<'a>,
    probes: ProbeFamily<'a>,
    physical: PhysicalFamilyWorkspace<'a>,
    probe_physical: ProbePhysicalWorkspace<'a>,
    probe_pressure: ProbePressureWorkspace<'a>,
    pressure: PressureFamilyWorkspace<'a>,
    regional: RegionalTrackingWorkspace<'a>,
    residual: ResidualFamily<'a>,
    binding: NodeBindingWorkspace<'a>,
    reports: Vec<DiagnosticEvent>,
    next: usize,
    failed: bool,
    charged: DiagnosticWork,
}
impl<'a> DiagnosticDriver<'a> {
    /// Allocate only after the aggregate plan is admitted.
    pub fn new(plan: DiagnosticPlan<'a>) -> Result<Self, DiagnosticError> {
        let mut reports = Vec::new();
        reports
            .try_reserve_exact(plan.bounds.work.attempts)
            .map_err(|_| SolverError::AllocationFailed)?;
        Ok(Self {
            ordinary: V2Family::new(plan.family)?,
            probes: ProbeFamily::new(plan.probes)?,
            physical: PhysicalFamilyWorkspace::new(plan.physical)?,
            probe_physical: ProbePhysicalWorkspace::new(plan.probe_physical)?,
            probe_pressure: ProbePressureWorkspace::new(plan.probe_pressure)?,
            pressure: PressureFamilyWorkspace::new(plan.pressure)?,
            regional: RegionalTrackingWorkspace::new(plan.regional)?,
            residual: ResidualFamily::new(plan.residual)?,
            binding: NodeBindingWorkspace::new(plan.binding),
            reports,
            plan,
            next: 0,
            failed: false,
            charged: DiagnosticWork::default(),
        })
    }
    /// Immutable complete admission plan.
    pub fn plan(&self) -> DiagnosticPlan<'a> {
        self.plan
    }
    /// All completely published events in manifest order.
    pub fn reports(&self) -> &[DiagnosticEvent] {
        &self.reports
    }
    /// Read-only ordinary family state.
    pub fn ordinary(&self) -> &V2Family<'a> {
        &self.ordinary
    }
    /// Read-only independent probe family state.
    pub fn probes(&self) -> &ProbeFamily<'a> {
        &self.probes
    }
    /// Charged coordinator scheduling work, including a failed attempt.
    pub fn charged_work(&self) -> DiagnosticWork {
        self.charged
    }
    /// Read-only charged work for every attached consumer.
    pub fn consumer_work(&self) -> DiagnosticConsumerWork {
        DiagnosticConsumerWork {
            probes: self.probes.charged_work(),
            physical: self.physical.charged_work(),
            probe_physical: self.probe_physical.charged_work(),
            probe_pressure: self.probe_pressure.charged_work(),
            pressure: self.pressure.charged_work(),
            reference: self.regional.tracking_work(),
            regional: self.regional.regional_work(),
            residual: self.residual.charged_work(),
            binding: self.binding.charged_work(),
        }
    }
    /// Next manifest clock, if any.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// Run one manifest event and publish it only after all scheduled consumers finish.
    pub fn advance(&mut self) -> Result<Option<DiagnosticEvent>, DiagnosticError> {
        if self.failed {
            return Err(DiagnosticError::Terminated);
        }
        let Some(clock) = self.next_time() else {
            return Ok(None);
        };
        let accepted_clock = self.plan.family.times().as_slice().contains(&clock);
        self.charge(accepted_clock);
        match self.compute(clock, accepted_clock) {
            Ok(event) => {
                self.reports.push(event);
                self.next += 1;
                Ok(Some(event))
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn compute(
        &mut self,
        clock: TickClock,
        accepted_clock: bool,
    ) -> Result<DiagnosticEvent, DiagnosticError> {
        let spectral = if accepted_clock {
            Some(self.ordinary.advance()?.ok_or(FamilyError::InvalidFamily)?)
        } else {
            None
        };
        let probe = self.probes.advance()?.ok_or(FamilyError::InvalidFamily)?;
        self.require_probe(probe, clock)?;
        let probe_physical = self.probe_physical.measure(&self.probes, probe)?;
        let probe_pressure = self.probe_pressure.measure(&self.probes, probe)?;
        let (accepted, residual) = if let Some(spectral) = spectral {
            let physical = self.physical.measure(&self.ordinary)?;
            let pressure = self.pressure.measure(&self.ordinary)?;
            let regional_reference = self.regional.measure(&self.ordinary)?;
            let node_binding = self.binding.measure(&self.ordinary, &self.probes)?;
            self.require_accepted(
                clock,
                spectral,
                physical,
                pressure,
                regional_reference,
                node_binding,
            )?;
            (
                AcceptedEvidence::measured(AcceptedDiagnostic {
                    spectral,
                    physical,
                    pressure,
                    regional_reference,
                    node_binding,
                }),
                ResidualEvidence::not_scheduled(),
            )
        } else {
            let residual = self.residual.measure(&self.probes)?;
            if (residual.clock(), residual.reconstruction().identity())
                != (clock, self.plan.probes.identity())
            {
                return Err(FamilyError::InvalidFamily.into());
            }
            (
                AcceptedEvidence::not_scheduled(),
                ResidualEvidence::measured(residual),
            )
        };
        Ok(DiagnosticEvent {
            clock,
            family_identity: self.plan.family.identity(),
            probe_identity: self.plan.probes.identity(),
            probe,
            probe_physical,
            probe_pressure,
            accepted,
            residual,
        })
    }
    fn require_probe(
        &self,
        sample: crate::v2_experiment::probes::ProbeSample,
        clock: TickClock,
    ) -> Result<(), FamilyError> {
        if (sample.clock(), sample.identity()) != (clock, self.plan.probes.identity()) {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(())
    }
    fn require_accepted(
        &self,
        clock: TickClock,
        spectral: crate::v2_experiment::RefinementSample,
        physical: crate::v2_experiment::physical::PhysicalRefinementSample,
        pressure: crate::v2_experiment::pressure::PressureRefinementSample,
        regional: crate::v2_experiment::reference::regional::RegionalTrackingSample,
        binding: crate::v2_experiment::binding::NodeBindingSample,
    ) -> Result<(), FamilyError> {
        let identity = self.plan.family.identity();
        let observed = AcceptedIdentity {
            clocks: [
                spectral.clock(),
                physical.clock(),
                pressure.clock(),
                regional.clock(),
                binding.clock(),
            ],
            families: [
                spectral.identity(),
                physical.identity(),
                pressure.identity(),
                regional.identity(),
                binding.family_identity(),
            ],
            probes: binding.probe_identity(),
        };
        let expected = AcceptedIdentity {
            clocks: [clock; 5],
            families: [identity; 5],
            probes: self.plan.probes.identity(),
        };
        if observed != expected {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(())
    }
    fn charge(&mut self, accepted_clock: bool) {
        self.charged.attempts += 1;
        self.charged.manifest_comparisons += self.plan.per_attempt.manifest_comparisons;
        if accepted_clock {
            self.charged.accepted_events += 1;
        } else {
            self.charged.residual_events += 1;
        }
    }
}

#[derive(PartialEq, Eq)]
struct AcceptedIdentity {
    clocks: [TickClock; 5],
    families: [[u8; 32]; 5],
    probes: [u8; 32],
}
