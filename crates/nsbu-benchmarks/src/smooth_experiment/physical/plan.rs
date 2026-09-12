//! Joint storage and finite work admission for complete actual-state vector comparisons.
use super::{PhysicalFamilyWorkspace, QUANTITIES};
use crate::smooth_experiment::{FamilyError, FamilyPlan};
use nsbu_solver::{
    diagnostics::{
        derivatives::DerivativeWorkspace, local::TensorErrors,
        physical::PhysicalComparisonWorkspace,
    },
    domain::Layout,
    SolverError,
};

/// Worst-case charged work. Each malformed request spends one complete admitted attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhysicalFamilyWork {
    /// Attempts, including failures before sampling.
    pub attempts: usize,
    /// Maximum scalar inverse FFT calls charged for those attempts.
    pub scalar_transforms: usize,
    /// Conservative weighted coefficient/sample visits, excluding FFT internals; not FLOPs.
    pub weighted_visits: usize,
}
/// Resource reservation for one complete physical-refinement consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFamilyBounds {
    /// Diagnostic workspace, fixed metadata and one returned sample record.
    pub storage_bytes: usize,
    /// All six family owners plus this physical consumer; other consumers need their own budget.
    pub joint_storage_bytes: usize,
    /// Finite call allowance, at least the number of required tested times.
    pub maximum_attempts: usize,
    /// Maximum total charged work over the entire allowance.
    pub work: PhysicalFamilyWork,
}
/// Immutable settings borrowed from the already admitted actual-from-rest family.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalFamilyPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) bounds: PhysicalFamilyBounds,
    pub(super) per_attempt: PhysicalFamilyWork,
}
impl<'a> PhysicalFamilyPlan<'a> {
    /// Admit the entire family plus this diagnostic consumer before allocating either.
    /// Floors follow velocity/gradient/Hessian/vorticity order and stay fixed for the run.
    /// Caller input/output artifacts and allocator overhead remain separately budgeted.
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
        let domain = family.branches[2].plan.resources().domain();
        let storage_bytes = PhysicalComparisonWorkspace::reservation(domain, domain, samples)?
            .checked_add(std::mem::size_of::<PhysicalFamilyWorkspace<'_>>())
            .and_then(|n| n.checked_add(std::mem::size_of::<super::PhysicalRefinementSample>()))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = storage_bytes
            .checked_add(family.bounds().storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let per_attempt = work(domain.layout(), samples)?;
        let total = PhysicalFamilyWork {
            attempts: maximum_attempts,
            scalar_transforms: mul(per_attempt.scalar_transforms, maximum_attempts)?,
            weighted_visits: mul(per_attempt.weighted_visits, maximum_attempts)?,
        };
        Ok(Self {
            family,
            samples,
            floors,
            bounds: PhysicalFamilyBounds {
                storage_bytes,
                joint_storage_bytes,
                maximum_attempts,
                work: total,
            },
            per_attempt,
        })
    }
    /// Complete joint storage and worst-case work; acceptance is a separate experiment decision.
    pub fn bounds(self) -> PhysicalFamilyBounds {
        self.bounds
    }
    /// Exact diagnostic grid shared by all twenty quantity/pair comparisons per tested time.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed positive floors in the public quantity order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
}
fn work(source: Layout, samples: Layout) -> Result<PhysicalFamilyWork, SolverError> {
    let transforms = 5 * QUANTITIES
        .into_iter()
        .map(|q| q.scalar_transforms())
        .sum::<usize>();
    // Each scalar sample has validation/differentiation and real output visits. Additional
    // complete component/difference reductions and fixed policy checks are reserved explicitly.
    let weighted_visits = mul(DerivativeWorkspace::coefficient_visits(source)?, transforms)?
        .checked_add(mul(
            samples.real_len(),
            4 * transforms + 10 * 5 * QUANTITIES.len(),
        )?)
        .and_then(|n| n.checked_add(1024))
        .ok_or(SolverError::SizeOverflow)?;
    Ok(PhysicalFamilyWork {
        attempts: 1,
        scalar_transforms: transforms,
        weighted_visits,
    })
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
