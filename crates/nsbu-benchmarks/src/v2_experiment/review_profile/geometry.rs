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
    /// Bind caller-supplied fixed-cardinality manifests to one admitted probe plan.
    ///
    /// The three arrays intentionally have the frozen 3/5/7 cardinalities.  The
    /// coarse array must be the complete accepted [`FamilyPlan`](crate::v2_experiment::FamilyPlan)
    /// manifest, the fine array must be the complete [`ProbePlan`] manifest, and
    /// the middle array is a strict intermediate refinement.  The four supplied
    /// refinements must be the ordered off-stage complement of the accepted
    /// clocks, with the actual three branch-node geometries recomputed from the
    /// immutable plan.  This admission stores fixed copies only; later profile
    /// caps remain the bounded policy/review admission point.
    pub fn from_manifests(
        plan: ProbePlan<'_>,
        coarse: [TickClock; 3],
        middle: [TickClock; 5],
        fine: [TickClock; 7],
        probes: [ProbeRefinement; 4],
    ) -> Result<Self, ProfileError> {
        let coarse_times = TestedTimes::new(&coarse, coarse.len())?;
        let middle_times = TestedTimes::new(&middle, middle.len())?;
        let fine_times = TestedTimes::new(&fine, fine.len())?;
        if !middle_times.refines(coarse_times) || !fine_times.refines(middle_times) {
            return Err(InvalidGeometry);
        }

        let family = plan.family_plan();
        if coarse_times.as_slice() != family.times().as_slice()
            || fine_times.as_slice() != plan.tested_times().as_slice()
        {
            return Err(InvalidGeometry);
        }
        let steps = branch_steps(family)?;
        let offstage = offstage_clocks(fine, coarse);
        validate_refinements(probes, offstage, steps)?;

        Ok(Self {
            coarse,
            middle,
            fine,
            probes,
            family_identity: family.identity(),
            probe_identity: plan.identity(),
        })
    }

    /// Bind the known nested manifests and admitted h64/h32/h16 node geometry.
    pub fn startup(plan: ProbePlan<'_>) -> Result<Self, ProfileError> {
        let coarse = clocks([0, 64, 128])?;
        let middle = clocks([0, 63, 64, 127, 128])?;
        let fine = clocks([0, 7, 63, 64, 95, 127, 128])?;
        let family = plan.family_plan();
        let steps = branch_steps(family)?;
        let refinements = [7, 63, 95, 127].map(|elapsed| refinement(clock(elapsed)?, steps));
        let [a, b, c, d] = refinements;
        Self::from_manifests(plan, coarse, middle, fine, [a?, b?, c?, d?])
    }
    /// Three strictly nested exact manifests in coarse-to-fine order.
    pub fn time_sets(&self) -> Result<[TestedTimes<'_>; 3], VerificationError> {
        Ok([
            TestedTimes::new(&self.coarse, self.coarse.len())?,
            TestedTimes::new(&self.middle, self.middle.len())?,
            TestedTimes::new(&self.fine, self.fine.len())?,
        ])
    }
    /// All four admitted reconstruction refinements bound to the fine manifest.
    pub fn reconstruction(&self) -> Result<ReconstructionSamples<'_>, VerificationError> {
        let fine = TestedTimes::new(&self.fine, self.fine.len())?;
        ReconstructionSamples::new(&self.probes, fine, self.probes.len())
    }
    /// Fine coordinator clocks in required row order.
    pub fn fine(&self) -> &[TickClock; 7] {
        &self.fine
    }
    /// Admitted node geometry for probes 7, 63, 95 and 127.
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

fn branch_steps(family: crate::v2_experiment::FamilyPlan<'_>) -> Result<[u128; 3], ProfileError> {
    let steps = [3, 4, 2].map(|index| {
        family
            .branch_plan(index)
            .map(|branch| branch.settings().configuration.limits.step_ticks)
            .ok_or(InvalidGeometry)
    });
    let [a, b, c] = steps;
    Ok([a?, b?, c?])
}

fn offstage_clocks(fine: [TickClock; 7], accepted: [TickClock; 3]) -> [TickClock; 4] {
    let mut result = [fine[0]; 4];
    let mut index = 0;
    for clock in fine {
        if !accepted.contains(&clock) {
            result[index] = clock;
            index += 1;
        }
    }
    debug_assert_eq!(index, result.len());
    result
}

fn validate_refinements(
    probes: [ProbeRefinement; 4],
    offstage: [TickClock; 4],
    steps: [u128; 3],
) -> Result<(), ProfileError> {
    for (supplied, probe) in probes.into_iter().zip(offstage) {
        let expected = refinement(probe, steps)?;
        for (actual, expected) in supplied.levels().into_iter().zip(expected.levels()) {
            if actual.time() != expected.time() || actual.nodes() != expected.nodes() {
                return Err(InvalidGeometry);
            }
        }
    }
    Ok(())
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
