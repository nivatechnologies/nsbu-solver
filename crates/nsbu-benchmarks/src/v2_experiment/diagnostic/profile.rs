//! Owned clocks and fixed numerical policy for the bounded startup diagnostic.
use super::{DiagnosticPlan, DiagnosticSettings};
use crate::{
    runtime_force::ForceSettings,
    v2_experiment::{probes::ProbePlan, FamilyError, FamilyPlan, FamilySettings},
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
    SolverError,
};

/// Owned fixed-array startup profile shared by the example and public CLI.
#[derive(Debug, Clone)]
pub struct StartupProfile {
    accepted: [TickClock; 3],
    manifest: [TickClock; 7],
    residual: [TickClock; 4],
}
impl StartupProfile {
    /// Build the fixed exact clocks without allocating numerical state.
    pub fn new() -> Result<Self, SolverError> {
        let accepted = clocks([0, 64, 128])?;
        let manifest = clocks([0, 7, 63, 64, 95, 127, 128])?;
        Ok(Self {
            accepted,
            manifest,
            residual: [manifest[1], manifest[2], manifest[4], manifest[5]],
        })
    }
    /// Admit the complete fixed profile while borrowing this owner's clock arrays.
    pub fn plan(&self, joint_cap: usize) -> Result<DiagnosticPlan<'_>, FamilyError> {
        self.admit(joint_cap, false)
    }

    /// Admit the same diagnostic profile with caches on trajectory RHS providers only.
    pub fn plan_cached(&self, joint_cap: usize) -> Result<DiagnosticPlan<'_>, FamilyError> {
        self.admit(joint_cap, true)
    }

    fn admit(&self, joint_cap: usize, cached: bool) -> Result<DiagnosticPlan<'_>, FamilyError> {
        let settings = settings()?;
        let times = TestedTimes::new(&self.accepted, self.accepted.len())?;
        let family = if cached {
            FamilyPlan::new_cached(settings, times, joint_cap)?
        } else {
            FamilyPlan::new(settings, times, joint_cap)?
        };
        let probes = ProbePlan::new(
            family,
            TestedTimes::new(&self.manifest, self.manifest.len())?,
            self.manifest.len(),
            joint_cap,
        )?;
        DiagnosticPlan::new(family, probes, &self.residual, policy()?, joint_cap)
    }
    /// Ordinary accepted-clock array `[0,64,128]`.
    pub fn accepted_times(&self) -> &[TickClock; 3] {
        &self.accepted
    }
    /// Complete probe manifest `[0,7,63,64,95,127,128]`.
    pub fn manifest(&self) -> &[TickClock; 7] {
        &self.manifest
    }
    /// Genuine off-stage residual subset `[7,63,95,127]`.
    pub fn residual_times(&self) -> &[TickClock; 4] {
        &self.residual
    }
}

fn clocks<const N: usize>(elapsed: [u128; N]) -> Result<[TickClock; N], SolverError> {
    let values = elapsed.map(|value| TickClock::restore(-20, 8192, value, 8192 - value));
    let mut result = [TickClock::from_rest(-20, 8192)?; N];
    for (slot, value) in result.iter_mut().zip(values) {
        *slot = value?;
    }
    Ok(result)
}
fn settings() -> Result<FamilySettings, SolverError> {
    Ok(FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3])?,
            workers: 0,
        },
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    })
}
fn policy() -> Result<DiagnosticSettings, SolverError> {
    Ok(DiagnosticSettings {
        physical_samples: Layout::new([12; 3])?,
        pressure_samples: Layout::new([24; 3])?,
        reference_samples: Layout::new([12; 3])?,
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
        coverage_panels: [256, 512, 1024],
    })
}
