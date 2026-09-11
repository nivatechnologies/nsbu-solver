//! Joint admission for exact-v2 residuals over a genuine non-stage probe subset.
use super::{ResidualFamily, ResidualFamilySample, ResidualWorkspace};
use crate::v2_experiment::{
    probes::{plan::nodes, ProbePlan},
    FamilyError, PAIRS,
};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, conservative::ConservativeWorkspace},
    domain::{Domain, Layout, TickClock},
    verification::reconstruction::{OffStageProbe, ProbeRefinement},
    SolverError,
};

/// Work for one or more complete six-branch report attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResidualFamilyWork {
    /// Charged whole-report attempts.
    pub attempts: usize,
    /// Six child residual allowances.
    pub residual: super::ResidualWork,
    /// Complete doubled-band comparison visits.
    pub comparison_work_units: usize,
    /// Exact identity/clock/origin comparisons.
    pub binding_clock_comparisons: usize,
}
/// Consumer and joint probe-family bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualFamilyBounds {
    /// Six residual workspaces and report metadata.
    pub storage_bytes: usize,
    /// Probe owners/scratch plus this consumer.
    pub joint_storage_bytes: usize,
    /// Complete finite report work.
    pub work: ResidualFamilyWork,
    /// Six exact off-stage geometries per declared residual clock.
    pub admission_geometry_checks: usize,
}
/// Immutable residual subset bound to one exact-v2 probe plan.
#[derive(Debug, Clone, Copy)]
pub struct ResidualFamilyPlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) times: &'a [TickClock],
    pub(super) sources: [Domain; 6],
    pub(super) force_samples: Layout,
    pub(super) bounds: ResidualFamilyBounds,
    pub(super) per_attempt: ResidualFamilyWork,
}
impl<'a> ResidualFamilyPlan<'a> {
    /// Admit a strict ordered subset of genuine non-stage clocks and all residual resources.
    pub fn new(
        probes: ProbePlan<'a>,
        times: &'a [TickClock],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if times.is_empty() || maximum_attempts < times.len() {
            return Err(FamilyError::InvalidFamily);
        }
        admit_times(probes, times)?;
        let sources = probes.branches.map(|branch| branch.resources().domain());
        let force_samples = common_force_samples(probes, sources)?;
        let (storage_bytes, per_attempt) = reservation(probes, sources, force_samples)?;
        let joint_storage_bytes = add(storage_bytes, probes.bounds().joint_storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            probes,
            times,
            sources,
            force_samples,
            per_attempt,
            bounds: ResidualFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                work: scale(per_attempt, maximum_attempts)?,
                admission_geometry_checks: mul(times.len(), 6)?,
            },
        })
    }
    /// Complete simultaneous storage and finite work.
    pub fn bounds(self) -> ResidualFamilyBounds {
        self.bounds
    }
    /// Exact genuine non-stage subset.
    pub fn tested_times(self) -> &'a [TickClock] {
        self.times
    }
    /// Underlying exact-v2 reconstruction probe plan.
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
pub(super) fn geometry(
    probes: ProbePlan<'_>,
    time: TickClock,
) -> Result<[OffStageProbe; 6], FamilyError> {
    let values = probes.branches.map(|branch| {
        let step = branch.settings().configuration.limits.step_ticks;
        Ok::<_, FamilyError>(OffStageProbe::new(nodes(time, step)?, time)?)
    });
    let [a, b, c, d, e, f] = values;
    Ok([a?, b?, c?, d?, e?, f?])
}
fn reservation(
    probes: ProbePlan<'_>,
    sources: [Domain; 6],
    force_samples: Layout,
) -> Result<(usize, ResidualFamilyWork), SolverError> {
    let mut bytes = add(
        std::mem::size_of::<ResidualFamily<'_>>(),
        add(std::mem::size_of::<ResidualFamilySample>(), 64 * 16)?,
    )?;
    let mut work = ResidualFamilyWork {
        attempts: 1,
        ..ResidualFamilyWork::default()
    };
    for source in sources {
        let child = ResidualWorkspace::reservation(source, force_samples, 1)?;
        bytes = add(bytes, child.storage_bytes)?;
        work.residual = add_residual(work.residual, child.work)?;
    }
    for (a, b) in PAIRS {
        let comparison = ComparisonPlan::new(
            ConservativeWorkspace::diagnostic_domain(sources[a])?,
            ConservativeWorkspace::diagnostic_domain(sources[b])?,
        )?;
        work.comparison_work_units = add(work.comparison_work_units, comparison.work_units())?;
    }
    work.binding_clock_comparisons = add(mul(probes.tested_times().as_slice().len(), 2)?, 14)?;
    Ok((bytes, work))
}
fn common_force_samples(
    probes: ProbePlan<'_>,
    sources: [Domain; 6],
) -> Result<Layout, SolverError> {
    let samples = probes.branches[0].settings().force.double_grid()?.samples;
    for (branch, source) in probes.branches.iter().zip(sources) {
        if branch.settings().force.double_grid()?.samples != samples {
            return Err(SolverError::InvalidDomain);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        if diagnostic
            .layout()
            .dimensions()
            .iter()
            .zip(samples.dimensions())
            .any(|(required, available)| *required > available)
        {
            return Err(SolverError::InvalidDomain);
        }
    }
    Ok(samples)
}
fn add_residual(
    a: super::ResidualWork,
    b: super::ResidualWork,
) -> Result<super::ResidualWork, SolverError> {
    Ok(super::ResidualWork {
        probes: add(a.probes, b.probes)?,
        provider_work_units: add(a.provider_work_units, b.provider_work_units)?,
        scalar_transforms: add(a.scalar_transforms, b.scalar_transforms)?,
        coefficient_work_units: add(a.coefficient_work_units, b.coefficient_work_units)?,
    })
}
fn scale(work: ResidualFamilyWork, n: usize) -> Result<ResidualFamilyWork, SolverError> {
    Ok(ResidualFamilyWork {
        attempts: n,
        residual: super::ResidualWork {
            probes: mul(work.residual.probes, n)?,
            provider_work_units: mul(work.residual.provider_work_units, n)?,
            scalar_transforms: mul(work.residual.scalar_transforms, n)?,
            coefficient_work_units: mul(work.residual.coefficient_work_units, n)?,
        },
        comparison_work_units: mul(work.comparison_work_units, n)?,
        binding_clock_comparisons: mul(work.binding_clock_comparisons, n)?,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
