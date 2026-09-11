//! Joint storage and finite-work admission for analytical tracking.
use super::{ReferenceTrackingSample, ReferenceTrackingWorkspace, QUANTITIES};
use crate::{
    fields::reference::ReferenceEvaluation,
    v2_experiment::{FamilyError, FamilyPlan},
};
use nsbu_solver::{
    diagnostics::{derivatives::DerivativeWorkspace, local::TensorErrors},
    domain::Layout,
    SolverError,
};

/// Worst-case work charged for complete and refused report attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReferenceTrackingWork {
    /// Complete report attempts.
    pub attempts: usize,
    /// Pointwise analytical reference evaluations.
    pub reference_evaluations: usize,
    /// Conservative analytical scalar-root iterations.
    pub root_iterations: usize,
    /// Actual-state scalar inverse transforms.
    pub scalar_transforms: usize,
    /// Conservative coefficient, sample and reduction visits; excludes FFT internals.
    pub weighted_visits: usize,
}

/// Consumer and joint family resource bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceTrackingBounds {
    /// Consumer-owned workspaces, cached references and report scratch.
    pub storage_bytes: usize,
    /// Six trajectory owners plus this consumer.
    pub joint_storage_bytes: usize,
    /// Complete worst-case work across the attempt allowance.
    pub work: ReferenceTrackingWork,
}

/// Immutable tracking policy borrowed from an admitted exact-v2 family.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceTrackingPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) bounds: ReferenceTrackingBounds,
    pub(super) per_attempt: ReferenceTrackingWork,
}
impl<'a> ReferenceTrackingPlan<'a> {
    /// Admit every simultaneous owner and finite attempt before allocating diagnostics.
    pub fn new(
        family: FamilyPlan<'a>,
        samples: Layout,
        floors: [f64; 4],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < family.times.as_slice().len() {
            return Err(SolverError::ResourceLimit.into());
        }
        for floor in floors {
            TensorErrors::<1>::new(samples.real_len(), floor)?;
        }
        let sources = [0, 1, 2].map(|index| family.branches[index].resources().domain());
        let storage_bytes = reservation(sources, samples)?;
        let joint_storage_bytes = add(storage_bytes, family.bounds.storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = work(sources.map(|source| source.layout()), samples)?;
        let bounds = ReferenceTrackingBounds {
            storage_bytes,
            joint_storage_bytes,
            work: scale(per_attempt, maximum_attempts)?,
        };
        Ok(Self {
            family,
            samples,
            floors,
            bounds,
            per_attempt,
        })
    }
    /// Complete consumer and joint resource bounds.
    pub fn bounds(self) -> ReferenceTrackingBounds {
        self.bounds
    }
    /// Physical sampling lattice, independent of every retained trajectory grid.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Positive velocity/gradient/Hessian/vorticity relative floors.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
}

fn reservation(
    sources: [nsbu_solver::domain::Domain; 3],
    samples: Layout,
) -> Result<usize, SolverError> {
    let arrays = samples
        .real_len()
        .checked_mul(3 * 8 + std::mem::size_of::<ReferenceEvaluation>())
        .ok_or(SolverError::SizeOverflow)?;
    let derivatives = sources.into_iter().try_fold(0usize, |total, source| {
        add(total, DerivativeWorkspace::reservation(source, samples)?)
    })?;
    // Conservative allocator metadata/page allowance for three derivative owners
    // and the four separately allocated actual/error/reference/cache vectors.
    let allocator_allowance = 7usize.checked_mul(4096).ok_or(SolverError::SizeOverflow)?;
    derivatives
        .checked_add(arrays)
        .and_then(|n| n.checked_add(std::mem::size_of::<ReferenceTrackingWorkspace<'_>>()))
        .and_then(|n| n.checked_add(std::mem::size_of::<ReferenceTrackingSample>()))
        .and_then(|n| n.checked_add(allocator_allowance))
        .ok_or(SolverError::SizeOverflow)
}

fn work(sources: [Layout; 3], samples: Layout) -> Result<ReferenceTrackingWork, SolverError> {
    let transforms = 6 * QUANTITIES
        .into_iter()
        .map(|quantity| quantity.scalar_transforms() / 2)
        .sum::<usize>();
    let points = samples.real_len();
    let four_fine = sources[2]
        .half_len()
        .checked_mul(4)
        .ok_or(SolverError::SizeOverflow)?;
    let modal_visits = sources[0]
        .half_len()
        .checked_add(sources[1].half_len())
        .and_then(|n| n.checked_add(four_fine))
        .and_then(|n| n.checked_mul(transforms / 6))
        .ok_or(SolverError::SizeOverflow)?;
    let weighted_visits = Some(modal_visits)
        .and_then(|n| n.checked_add(points.checked_mul(3 * transforms + 6 * 4)?))
        .and_then(|n| n.checked_add(1024))
        .ok_or(SolverError::SizeOverflow)?;
    Ok(ReferenceTrackingWork {
        attempts: 1,
        reference_evaluations: points,
        root_iterations: points.checked_mul(128).ok_or(SolverError::SizeOverflow)?,
        scalar_transforms: transforms,
        weighted_visits,
    })
}

fn scale(work: ReferenceTrackingWork, count: usize) -> Result<ReferenceTrackingWork, SolverError> {
    Ok(ReferenceTrackingWork {
        attempts: count,
        reference_evaluations: mul(work.reference_evaluations, count)?,
        root_iterations: mul(work.root_iterations, count)?,
        scalar_transforms: mul(work.scalar_transforms, count)?,
        weighted_visits: mul(work.weighted_visits, count)?,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
