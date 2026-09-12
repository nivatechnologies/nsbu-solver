//! Owned nested review manifests derived from one admitted exact-v2 probe plan.
use super::{ProfileError, ProfileError::InvalidGeometry};
use crate::v2_experiment::probes::{nodes, ProbePlan};
use nsbu_solver::{
    domain::TickClock,
    verification::{
        reconstruction::{OffStageProbe, ProbeRefinement, ReconstructionSamples},
        times::TestedTimes,
        VerificationError,
    },
    SolverError,
};

/// Complete fixed startup geometry and immutable source-plan identities.
#[derive(Debug, Clone)]
pub struct ReviewGeometry {
    coarse: [TickClock; 3],
    middle: [TickClock; 5],
    fine: [TickClock; 7],
    probes: [ProbeRefinement; 4],
    family_identity: [u8; 32],
    probe_identity: [u8; 32],
}
impl ReviewGeometry {
    /// Bind the known nested manifests and all actual h64/h32/h16 probe histories.
    pub fn startup(plan: ProbePlan<'_>) -> Result<Self, ProfileError> {
        let coarse = clocks([0, 64, 128])?;
        let middle = clocks([0, 63, 64, 127, 128])?;
        let fine = clocks([0, 7, 63, 64, 95, 127, 128])?;
        if plan.tested_times().as_slice() != fine {
            return Err(InvalidGeometry);
        }
        let family = plan.family_plan();
        let steps = [3, 4, 2].map(|index| {
            family
                .branch_plan(index)
                .map(|branch| branch.settings().configuration.limits.step_ticks)
                .ok_or(InvalidGeometry)
        });
        let [a, b, c] = steps;
        let steps = [a?, b?, c?];
        let refinements = [7, 63, 95, 127].map(|elapsed| refinement(clock(elapsed)?, steps));
        let [a, b, c, d] = refinements;
        Ok(Self {
            coarse,
            middle,
            fine,
            probes: [a?, b?, c?, d?],
            family_identity: family.identity(),
            probe_identity: plan.identity(),
        })
    }
    /// Three strictly nested exact manifests in coarse-to-fine order.
    pub fn time_sets(&self) -> Result<[TestedTimes<'_>; 3], VerificationError> {
        Ok([
            TestedTimes::new(&self.coarse, self.coarse.len())?,
            TestedTimes::new(&self.middle, self.middle.len())?,
            TestedTimes::new(&self.fine, self.fine.len())?,
        ])
    }
    /// All four actual-node reconstruction refinements bound to the fine manifest.
    pub fn reconstruction(&self) -> Result<ReconstructionSamples<'_>, VerificationError> {
        let fine = TestedTimes::new(&self.fine, self.fine.len())?;
        ReconstructionSamples::new(&self.probes, fine, self.probes.len())
    }
    /// Fine coordinator clocks in required row order.
    pub fn fine(&self) -> &[TickClock; 7] {
        &self.fine
    }
    /// Exact histories for probes 7, 63, 95 and 127.
    pub fn probes(&self) -> &[ProbeRefinement; 4] {
        &self.probes
    }
    /// Ordinary family identity from the source probe plan.
    pub fn family_identity(&self) -> [u8; 32] {
        self.family_identity
    }
    /// Complete manifest/owner identity from the source probe plan.
    pub fn probe_identity(&self) -> [u8; 32] {
        self.probe_identity
    }
}
fn refinement(probe: TickClock, steps: [u128; 3]) -> Result<ProbeRefinement, ProfileError> {
    let levels = steps
        .map(|step| OffStageProbe::new(nodes(probe, step)?, probe).map_err(ProfileError::from));
    let [a, b, c] = levels;
    ProbeRefinement::new([a?, b?, c?]).map_err(ProfileError::from)
}
fn clock(elapsed: u128) -> Result<TickClock, SolverError> {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed)
}
fn clocks<const N: usize>(elapsed: [u128; N]) -> Result<[TickClock; N], SolverError> {
    let mut out = [TickClock::from_rest(-20, 8192)?; N];
    for (slot, value) in out.iter_mut().zip(elapsed) {
        *slot = clock(value)?;
    }
    Ok(out)
}
