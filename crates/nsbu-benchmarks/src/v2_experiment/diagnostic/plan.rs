//! Joint admission for two families, every consumer and retained events.
use super::{DiagnosticDriver, DiagnosticEvent};
use crate::v2_experiment::{
    binding::{NodeBindingPlan, NodeBindingWork},
    physical::{PhysicalFamilyPlan, PhysicalFamilyWork},
    pressure::{PressureFamilyPlan, PressureFamilyWork},
    probes::{
        physical::{ProbePhysicalPlan, ProbePhysicalWork},
        residuals::{ResidualFamilyPlan, ResidualFamilyWork},
        ProbePlan, ProbeWork,
    },
    reference::{
        regional::{RegionalTrackingPlan, RegionalTrackingWork},
        ReferenceTrackingPlan, ReferenceTrackingWork,
    },
    FamilyError, FamilyPlan,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    SolverError,
};

/// Fixed physical and analytical sampling policy.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticSettings {
    /// Physical-family comparison sample grid.
    pub physical_samples: Layout,
    /// Pressure comparison sample grid.
    pub pressure_samples: Layout,
    /// Analytical-reference sample grid.
    pub reference_samples: Layout,
    /// Relative floors for velocity, gradient, Hessian and vorticity.
    pub physical_floors: [f64; 4],
    /// Relative floors for pressure and pressure gradient.
    pub pressure_floors: [f64; 2],
    /// Relative floors for analytical velocity, gradient, Hessian and vorticity.
    pub reference_floors: [f64; 4],
    /// Regional implicit-root iteration policy; currently exactly 128.
    pub regional_root_budget: usize,
}

/// Coordinator scheduling work, separate from every numerical owner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiagnosticWork {
    /// Complete or failed manifest event attempts.
    pub attempts: usize,
    /// Accepted-state event paths.
    pub accepted_events: usize,
    /// Residual-only event paths.
    pub residual_events: usize,
    /// Manifest membership comparisons.
    pub manifest_comparisons: usize,
}

/// Complete aggregate storage and component work ledgers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticBounds {
    /// Retained events, metadata and two bounded transient event copies.
    pub storage_bytes: usize,
    /// Simultaneous ordinary/probe families, consumers and coordinator storage.
    pub joint_storage_bytes: usize,
    /// Complete coordinator scheduling allowance.
    pub work: DiagnosticWork,
    /// Probe-family work allowance.
    pub probes: ProbeWork,
    /// Physical comparison work allowance.
    pub physical: PhysicalFamilyWork,
    /// Reconstructed-value physical comparison work at every probe clock.
    pub probe_physical: ProbePhysicalWork,
    /// Pressure work allowance.
    pub pressure: PressureFamilyWork,
    /// Analytical tracking work allowance.
    pub reference: ReferenceTrackingWork,
    /// Added regional classification work allowance.
    pub regional: RegionalTrackingWork,
    /// Off-stage residual work allowance.
    pub residual: ResidualFamilyWork,
    /// Accepted-node binding work allowance.
    pub binding: NodeBindingWork,
}

/// Immutable schedule and complete joint resource admission.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) probes: ProbePlan<'a>,
    pub(super) physical: PhysicalFamilyPlan<'a>,
    pub(super) probe_physical: ProbePhysicalPlan<'a>,
    pub(super) pressure: PressureFamilyPlan<'a>,
    pub(super) regional: RegionalTrackingPlan<'a>,
    pub(super) residual: ResidualFamilyPlan<'a>,
    pub(super) binding: NodeBindingPlan<'a>,
    pub(super) bounds: DiagnosticBounds,
    pub(super) per_attempt: DiagnosticWork,
}
impl<'a> DiagnosticPlan<'a> {
    /// Admit an exact accepted/residual partition and all simultaneous resources.
    pub fn new(
        family: FamilyPlan<'a>,
        probes: ProbePlan<'a>,
        residual_times: &'a [TickClock],
        settings: DiagnosticSettings,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        let (events, accepted, residual_count) = admit_manifest(family, probes, residual_times)?;
        let physical = PhysicalFamilyPlan::new(
            family,
            settings.physical_samples,
            settings.physical_floors,
            accepted,
            joint_cap,
        )?;
        let pressure = PressureFamilyPlan::new(
            family,
            settings.pressure_samples,
            settings.pressure_floors,
            accepted,
            joint_cap,
        )?;
        let probe_physical = ProbePhysicalPlan::new(
            probes,
            settings.physical_samples,
            settings.physical_floors,
            events,
            joint_cap,
        )?;
        let tracking = ReferenceTrackingPlan::new(
            family,
            settings.reference_samples,
            settings.reference_floors,
            accepted,
            joint_cap,
        )?;
        let regional =
            RegionalTrackingPlan::new(tracking, settings.regional_root_budget, joint_cap)?;
        let residual = ResidualFamilyPlan::new(probes, residual_times, residual_count, joint_cap)?;
        let binding = NodeBindingPlan::new(family, probes, accepted, joint_cap)?;
        let storage_bytes = report_storage(events)?;
        let regional_increment = regional
            .bounds()
            .joint_storage_bytes
            .checked_sub(family.bounds().storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        let storage_parts = [
            family.bounds().storage_bytes,
            probes.bounds().joint_storage_bytes,
            physical.bounds().storage_bytes,
            probe_physical.bounds().storage_bytes,
            pressure.bounds().storage_bytes,
            regional_increment,
            residual.bounds().storage_bytes,
            binding.bounds().storage_bytes,
            storage_bytes,
        ];
        let joint_storage_bytes = storage_parts.into_iter().try_fold(0usize, |sum, value| {
            sum.checked_add(value).ok_or(SolverError::SizeOverflow)
        })?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = DiagnosticWork {
            attempts: 1,
            manifest_comparisons: events,
            ..DiagnosticWork::default()
        };
        let work = DiagnosticWork {
            attempts: events,
            accepted_events: accepted,
            residual_events: residual_count,
            manifest_comparisons: events
                .checked_mul(events)
                .ok_or(SolverError::SizeOverflow)?,
        };
        Ok(Self {
            family,
            probes,
            physical,
            probe_physical,
            pressure,
            regional,
            residual,
            binding,
            bounds: DiagnosticBounds {
                storage_bytes,
                joint_storage_bytes,
                work,
                probes: probes.bounds().work,
                physical: physical.bounds().work,
                probe_physical: probe_physical.bounds().work,
                pressure: pressure.bounds().work,
                reference: regional.bounds().tracking_work,
                regional: regional.bounds().regional_work,
                residual: residual.bounds().work,
                binding: binding.bounds().work,
            },
            per_attempt,
        })
    }
    /// Complete simultaneous storage and finite work bounds.
    pub fn bounds(self) -> DiagnosticBounds {
        self.bounds
    }
    /// Ordinary exact-v2 family plan.
    pub fn family_plan(self) -> FamilyPlan<'a> {
        self.family
    }
    /// Independent probe family and complete manifest plan.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
    /// Strict off-stage residual subset.
    pub fn residual_times(self) -> &'a [TickClock] {
        self.residual.tested_times()
    }
    /// Fixed physical and analytical sampling policy bound to this plan.
    pub fn diagnostic_settings(self) -> DiagnosticSettings {
        DiagnosticSettings {
            physical_samples: self.physical.sample_layout(),
            pressure_samples: self.pressure.sample_layout(),
            reference_samples: self.regional.tracking_plan().sample_layout(),
            physical_floors: self.physical.relative_floors(),
            pressure_floors: self.pressure.relative_floors(),
            reference_floors: self.regional.tracking_plan().relative_floors(),
            regional_root_budget: self.regional.root_budget(),
        }
    }
}

fn admit_manifest(
    family: FamilyPlan<'_>,
    probes: ProbePlan<'_>,
    residual: &[TickClock],
) -> Result<(usize, usize, usize), FamilyError> {
    if family.identity() != probes.family_plan().identity() {
        return Err(FamilyError::InvalidFamily);
    }
    let manifest = probes.tested_times().as_slice();
    let accepted = family.times().as_slice();
    if probes.bounds().work.attempts < manifest.len() || manifest.is_empty() {
        return Err(FamilyError::InvalidFamily);
    }
    if accepted.iter().any(|clock| !manifest.contains(clock)) {
        return Err(FamilyError::InvalidFamily);
    }
    for clock in manifest {
        if accepted.contains(clock) == residual.contains(clock) {
            return Err(FamilyError::InvalidFamily);
        }
    }
    if accepted.len().checked_add(residual.len()) != Some(manifest.len()) {
        return Err(FamilyError::InvalidFamily);
    }
    Ok((manifest.len(), accepted.len(), residual.len()))
}

fn report_storage(events: usize) -> Result<usize, SolverError> {
    let event_slots = events.checked_add(2).ok_or(SolverError::SizeOverflow)?;
    std::mem::size_of::<DiagnosticEvent>()
        .checked_mul(event_slots)
        .and_then(|n| n.checked_add(std::mem::size_of::<Vec<DiagnosticEvent>>()))
        .and_then(|n| n.checked_add(std::mem::size_of::<DiagnosticPlan<'_>>()))
        .and_then(|n| n.checked_add(std::mem::size_of::<DiagnosticDriver<'_>>()))
        .and_then(|n| n.checked_add(4096))
        .ok_or(SolverError::SizeOverflow)
}
