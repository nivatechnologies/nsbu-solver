//! Joint residual admission and exact non-stage geometry over a borrowed subset of streamed probes.
use super::{ResidualFamily, ResidualFamilySample};
use crate::smooth_experiment::{
    probes::{plan::nodes, ProbePlan},
    residual::{ResidualWork, ResidualWorkspace},
    FamilyError, PAIRS,
};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, conservative::ConservativeWorkspace},
    domain::{Domain, TickClock},
    verification::reconstruction::{OffStageProbe, ProbeRefinement},
    SolverError,
};

/// Whole attempted-report charges; malformed requests retain the complete conservative allowance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResidualFamilyWork {
    /// Attempted complete six-branch residual reports.
    pub attempts: usize,
    /// Six independent reconstruction/force/product/residual allowances per aggregate attempt.
    pub residual: ResidualWork,
    /// Complete doubled-band coefficient-comparison visits, separate from child residual work.
    pub comparison_work_units: usize,
    /// Worst-case exact-clock equality checks during complete manifest/probe binding.
    pub binding_clock_comparisons: usize,
}
/// Simultaneous numerical storage and finite work, excluding caller manifests and retained artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualFamilyBounds {
    /// Six independent residual consumers, aggregate metadata and conservative report scratch.
    pub storage_bytes: usize,
    /// Complete ProbeFamily owners/interpolants plus this residual consumer.
    pub joint_storage_bytes: usize,
    /// Complete maximum charged report work.
    pub work: ResidualFamilyWork,
    /// Six exact non-stage geometries admitted per declared residual time.
    pub admission_geometry_checks: usize,
}
/// Immutable residual subset and actual from-rest owner profile; no numerical buffers are allocated.
#[derive(Debug, Clone, Copy)]
pub struct ResidualFamilyPlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) times: &'a [TickClock],
    pub(super) sources: [Domain; 6],
    pub(super) bounds: ResidualFamilyBounds,
    pub(super) per_attempt: ResidualFamilyWork,
}
impl<'a> ResidualFamilyPlan<'a> {
    /// Admit a strictly ordered nonempty subset of genuine off-stage clocks from the complete
    /// probe manifest. Temporal histories must form two strict nested refinements at every time.
    pub fn new(
        probes: ProbePlan<'a>,
        times: &'a [TickClock],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if times.is_empty() || maximum_attempts < times.len() {
            return Err(FamilyError::InvalidFamily);
        }
        let family = probes.family_plan();
        let sources = family
            .branches
            .map(|branch| branch.plan.resources().domain());
        admit_times(probes, times)?;
        let (storage_bytes, per_attempt) = reservation(probes, sources)?;
        let joint_storage_bytes = add(storage_bytes, probes.bounds().joint_storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            probes,
            times,
            sources,
            per_attempt,
            bounds: ResidualFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                work: scale(per_attempt, maximum_attempts)?,
                admission_geometry_checks: mul(times.len(), 6)?,
            },
        })
    }
    /// Complete fixed admission; other physical/reference consumers need additional joint storage.
    pub fn bounds(self) -> ResidualFamilyBounds {
        self.bounds
    }
    /// Exact genuine off-stage subset, not a claim of continuous interval control.
    pub fn tested_times(self) -> &'a [TickClock] {
        self.times
    }
    /// Actual owner and full physical probe schedule used by this consumer.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
}
fn admit_times(probes: ProbePlan<'_>, times: &[TickClock]) -> Result<(), FamilyError> {
    let mut previous = None;
    for &time in times {
        if !probes.tested_times().as_slice().contains(&time)
            || previous.is_some_and(|old| time.elapsed() <= old)
        {
            return Err(FamilyError::InvalidFamily);
        }
        let geometries = geometry(probes, time)?;
        ProbeRefinement::new([geometries[3], geometries[4], geometries[2]])?;
        previous = Some(time.elapsed());
    }
    Ok(())
}
fn geometry(probes: ProbePlan<'_>, time: TickClock) -> Result<[OffStageProbe; 6], FamilyError> {
    let [a, b, c, d, e, f] = probes.family_plan().branches.map(|branch| {
        let step = branch.plan.configuration().limits.step_ticks;
        Ok::<_, FamilyError>(OffStageProbe::new(nodes(time, step)?, time)?)
    });
    Ok([a?, b?, c?, d?, e?, f?])
}
fn reservation(
    probes: ProbePlan<'_>,
    sources: [Domain; 6],
) -> Result<(usize, ResidualFamilyWork), SolverError> {
    let mut bytes = add(
        std::mem::size_of::<ResidualFamily<'_>>(),
        mul(std::mem::size_of::<ResidualFamilySample>(), 4)?,
    )?;
    let mut work = ResidualFamilyWork {
        attempts: 1,
        ..ResidualFamilyWork::default()
    };
    for source in sources {
        let child = ResidualWorkspace::reservation(source, 1)?;
        bytes = add(bytes, child.storage_bytes)?;
        work.residual.probes = add(work.residual.probes, child.work.probes)?;
        work.residual.provider_work_units = add(
            work.residual.provider_work_units,
            child.work.provider_work_units,
        )?;
        work.residual.scalar_transforms = add(
            work.residual.scalar_transforms,
            child.work.scalar_transforms,
        )?;
        work.residual.coefficient_work_units = add(
            work.residual.coefficient_work_units,
            child.work.coefficient_work_units,
        )?;
    }
    for (a, b) in PAIRS {
        let plan = ComparisonPlan::new(
            ConservativeWorkspace::diagnostic_domain(sources[a])?,
            ConservativeWorkspace::diagnostic_domain(sources[b])?,
        )?;
        work.comparison_work_units = add(work.comparison_work_units, plan.work_units())?;
    }
    // One search in the full probe list, equality of both complete manifests and the selected clock.
    work.binding_clock_comparisons = add(
        add(
            mul(probes.tested_times().as_slice().len(), 2)?,
            probes.family_plan().times.as_slice().len(),
        )?,
        1,
    )?;
    Ok((bytes, work))
}
fn scale(work: ResidualFamilyWork, count: usize) -> Result<ResidualFamilyWork, SolverError> {
    Ok(ResidualFamilyWork {
        attempts: count,
        residual: ResidualWork {
            probes: mul(work.residual.probes, count)?,
            provider_work_units: mul(work.residual.provider_work_units, count)?,
            scalar_transforms: mul(work.residual.scalar_transforms, count)?,
            coefficient_work_units: mul(work.residual.coefficient_work_units, count)?,
        },
        comparison_work_units: mul(work.comparison_work_units, count)?,
        binding_clock_comparisons: mul(work.binding_clock_comparisons, count)?,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
