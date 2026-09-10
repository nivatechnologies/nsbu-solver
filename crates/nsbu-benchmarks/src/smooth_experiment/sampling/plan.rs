//! Admit all simultaneous diagnostic owners and finite work before allocating any of them.
use super::{SamplingSample, SamplingWorkspace};
use crate::smooth_experiment::{
    physical::PhysicalFamilyPlan, pressure::PressureFamilyPlan, FamilyError, FamilyPlan,
};
use nsbu_solver::{domain::Layout, SolverError};

/// Worst-case work charged for complete sampling attempts, including failed requests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SamplingWork {
    /// Complete report attempts.
    pub attempts: usize,
    /// Scalar transforms in all six child consumers.
    pub scalar_transforms: usize,
    /// Independent prescribed-force provider work units.
    pub provider_work_units: usize,
    /// Conservative weighted coefficient/sample visits; excludes FFT internals, not FLOPs.
    pub weighted_visits: usize,
}
/// Joint allocation and finite traversal reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingBounds {
    /// All child diagnostics, fixed headers and conservative stack/report scratch.
    pub storage_bytes: usize,
    /// All six actual family owners plus this complete sampling consumer.
    pub joint_storage_bytes: usize,
    /// Maximum work across the entire attempt allowance.
    pub work: SamplingWork,
}
/// Immutable physical sampling policy over an already admitted actual trajectory family.
#[derive(Debug, Clone, Copy)]
pub struct SamplingPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) physical: [PhysicalFamilyPlan<'a>; 3],
    pub(super) pressure: [PressureFamilyPlan<'a>; 3],
    pub(super) bounds: SamplingBounds,
    pub(super) per_attempt: SamplingWork,
}
impl<'a> SamplingPlan<'a> {
    /// Require strict increases on every sample axis; no nesting or monotone peak is assumed.
    /// Floors follow velocity/gradient/Hessian/vorticity/pressure/pressure-gradient order.
    /// No trajectory, provider, FFT or physical output buffer is allocated during admission.
    pub fn new(
        family: FamilyPlan<'a>,
        samples: [Layout; 3],
        floors: [f64; 6],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        for pair in samples.windows(2) {
            if pair[0]
                .dimensions()
                .into_iter()
                .zip(pair[1].dimensions())
                .any(|(a, b)| a >= b)
            {
                return Err(FamilyError::InvalidFamily);
            }
        }
        let physical = samples.map(|grid| {
            PhysicalFamilyPlan::new(
                family,
                grid,
                [floors[0], floors[1], floors[2], floors[3]],
                maximum_attempts,
                joint_cap,
            )
        });
        let [a, b, c] = physical;
        let physical = [a?, b?, c?];
        let pressure = samples.map(|grid| {
            PressureFamilyPlan::new(
                family,
                grid,
                [floors[4], floors[5]],
                maximum_attempts,
                joint_cap,
            )
        });
        let [a, b, c] = pressure;
        let pressure = [a?, b?, c?];
        let bounds = reservation(family, physical, pressure, maximum_attempts)?;
        if bounds.joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            family,
            physical,
            pressure,
            bounds,
            per_attempt: SamplingWork {
                attempts: 1,
                scalar_transforms: bounds.work.scalar_transforms / maximum_attempts,
                provider_work_units: bounds.work.provider_work_units / maximum_attempts,
                weighted_visits: bounds.work.weighted_visits / maximum_attempts,
            },
        })
    }
    /// Includes every simultaneous owner; caller artifact buffers and allocator overhead are separate.
    pub fn bounds(self) -> SamplingBounds {
        self.bounds
    }
    /// Exact admitted sample grids, with no phase/location alignment.
    pub fn sample_layouts(self) -> [Layout; 3] {
        self.physical.map(|plan| plan.sample_layout())
    }
    /// Fixed positive floors shared by every resolution and comparison pair.
    pub fn relative_floors(self) -> [f64; 6] {
        let [a, b, c, d] = self.physical[0].relative_floors();
        let [e, f] = self.pressure[0].relative_floors();
        [a, b, c, d, e, f]
    }
}
fn reservation(
    family: FamilyPlan<'_>,
    physical: [PhysicalFamilyPlan<'_>; 3],
    pressure: [PressureFamilyPlan<'_>; 3],
    attempts: usize,
) -> Result<SamplingBounds, SolverError> {
    // Eight complete records conservatively cover local arrays, return copies and report metadata.
    let mut bytes = add(
        std::mem::size_of::<SamplingWorkspace<'_>>(),
        std::mem::size_of::<SamplingSample>()
            .checked_mul(8)
            .ok_or(SolverError::SizeOverflow)?,
    )?;
    let mut work = SamplingWork {
        attempts,
        ..SamplingWork::default()
    };
    for (a, b) in physical.into_iter().zip(pressure) {
        let a = a.bounds();
        let b = b.bounds();
        bytes = add(bytes, add(a.storage_bytes, b.storage_bytes)?)?;
        work.scalar_transforms = add(
            work.scalar_transforms,
            add(a.work.scalar_transforms, b.work.scalar_transforms)?,
        )?;
        work.provider_work_units = add(work.provider_work_units, b.work.provider_work_units)?;
        work.weighted_visits = add(
            work.weighted_visits,
            add(a.work.weighted_visits, b.work.weighted_visits)?,
        )?;
    }
    // Fixed schedule/merge/report visits are reserved in addition to all numerical children.
    work.weighted_visits = add(
        work.weighted_visits,
        attempts
            .checked_mul(4096)
            .ok_or(SolverError::SizeOverflow)?,
    )?;
    Ok(SamplingBounds {
        storage_bytes: bytes,
        joint_storage_bytes: add(bytes, family.bounds().storage_bytes)?,
        work,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
