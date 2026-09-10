//! Admit reconstructed-field consumers jointly with all six trajectories and interpolation scratch.
use super::{ProbeDiagnostics, ProbePhysicalSample};
use crate::smooth_experiment::{
    physical::{PhysicalFamilyPlan, PhysicalFamilyWork},
    pressure::{PressureFamilyPlan, PressureFamilyWork},
    probes::ProbePlan,
    FamilyError,
};
use nsbu_solver::{domain::Layout, SolverError};

/// Separate complete charges for physical tensors and independently constructed pressure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeDiagnosticWork {
    /// Aggregate attempts, including malformed requests before numerical work.
    pub attempts: usize,
    /// Worst-case complete velocity/gradient/Hessian/vorticity sampling work.
    pub physical: PhysicalFamilyWork,
    /// Worst-case complete prescribed-force/pressure/gradient work.
    pub pressure: PressureFamilyWork,
}
/// Complete simultaneous numerical reservation and finite diagnostic allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeDiagnosticBounds {
    /// Both diagnostic consumers and conservative complete-report/header allowance.
    pub storage_bytes: usize,
    /// All six accepted-history owners, interpolation scratch and both diagnostic consumers.
    pub joint_storage_bytes: usize,
    /// Aggregate and child finite work allowances.
    pub work: ProbeDiagnosticWork,
}
/// Immutable probe identity, exact manifest and six fixed physical relative floors.
#[derive(Debug, Clone, Copy)]
pub struct ProbeDiagnosticsPlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) physical: PhysicalFamilyPlan<'a>,
    pub(super) pressure: PressureFamilyPlan<'a>,
    pub(super) bounds: ProbeDiagnosticBounds,
}
impl<'a> ProbeDiagnosticsPlan<'a> {
    /// Admit all simultaneous storage before constructing any owner or consumer.
    /// Floors are velocity, gradient, Hessian, vorticity, pressure and pressure gradient.
    pub fn new(
        probes: ProbePlan<'a>,
        samples: Layout,
        floors: [f64; 6],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < probes.tested_times().as_slice().len() {
            return Err(SolverError::ResourceLimit.into());
        }
        let physical = PhysicalFamilyPlan::new(
            probes.family_plan(),
            samples,
            [floors[0], floors[1], floors[2], floors[3]],
            maximum_attempts,
            joint_cap,
        )?;
        let pressure = PressureFamilyPlan::new(
            probes.family_plan(),
            samples,
            [floors[4], floors[5]],
            maximum_attempts,
            joint_cap,
        )?;
        let storage_bytes = physical
            .bounds()
            .storage_bytes
            .checked_add(pressure.bounds().storage_bytes)
            .and_then(|n| n.checked_add(std::mem::size_of::<ProbeDiagnostics<'_>>()))
            .and_then(|n| n.checked_add(4 * std::mem::size_of::<ProbePhysicalSample>()))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = storage_bytes
            .checked_add(probes.bounds().joint_storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            probes,
            physical,
            pressure,
            bounds: ProbeDiagnosticBounds {
                storage_bytes,
                joint_storage_bytes,
                work: ProbeDiagnosticWork {
                    attempts: maximum_attempts,
                    physical: physical.bounds().work,
                    pressure: pressure.bounds().work,
                },
            },
        })
    }
    /// Complete joint reservation; caller manifests, retained artifacts and allocator overhead are separate.
    pub fn bounds(self) -> ProbeDiagnosticBounds {
        self.bounds
    }
    /// Common physical lattice retaining the complete doubled pressure band.
    pub fn sample_layout(self) -> Layout {
        self.physical.sample_layout()
    }
    /// Immutable exact physical probe schedule and original from-rest trajectory settings.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
}
