//! Joint admission for pressure construction, imported gauges and analytical sampling.
use super::{ImportedGauge, PressureReferenceBounds, PressureReferenceWork};
use crate::v2_experiment::{pressure::PressureFamilyPlan, FamilyError, FamilyPlan};
use nsbu_solver::{
    diagnostics::{conservative::ConservativeWorkspace, derivatives::DerivativeWorkspace},
    domain::Layout,
    SolverError,
};

/// Immutable all-six analytical pressure-tracking admission.
#[derive(Debug, Clone, Copy)]
pub struct PressureReferencePlan<'a> {
    pub(super) pressure: PressureFamilyPlan<'a>,
    pub(super) gauges: [ImportedGauge<'a>; 3],
    pub(super) bounds: PressureReferenceBounds,
    pub(super) per_attempt: PressureReferenceWork,
}
impl<'a> PressureReferencePlan<'a> {
    /// Admit exact gauge clocks, pressure construction, sampling storage and finite work.
    pub fn new(
        family: FamilyPlan<'a>,
        gauges: [ImportedGauge<'a>; 3],
        samples: Layout,
        floors: [f64; 2],
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        require_gauges(family, gauges)?;
        let pressure = PressureFamilyPlan::new(family, samples, floors, 3, joint_cap)?;
        let points = samples.real_len();
        let storage_bytes = reservation(pressure)?;
        let joint_storage_bytes = family
            .bounds()
            .storage_bytes
            .checked_add(storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let imported_gauge_hash_bytes = gauges.iter().try_fold(0usize, |total, gauge| {
            total
                .checked_add(gauge.artifact_bytes().len())
                .ok_or(SolverError::SizeOverflow)
        })?;
        // The existing pressure visit envelope covers ten constructions plus all
        // pair comparisons, conservatively exceeding this consumer's six.
        let pressure_work = pressure.per_attempt_work();
        let provider = pressure.provider_limits();
        let per_attempt = PressureReferenceWork {
            attempts: 1,
            reference_evaluations: points,
            root_iterations: points.checked_mul(128).ok_or(SolverError::SizeOverflow)?,
            provider_work_units: provider.work_units,
            scalar_transforms: provider
                .scalar_transforms
                .checked_add(6 * 9)
                .and_then(|n| n.checked_add(6 * 4))
                .ok_or(SolverError::SizeOverflow)?,
            weighted_visits: pressure_work
                .weighted_visits
                .checked_add(
                    points
                        .checked_mul(6 * 16 + 64)
                        .ok_or(SolverError::SizeOverflow)?,
                )
                .ok_or(SolverError::SizeOverflow)?,
        };
        let work = per_attempt.scale(3)?;
        Ok(Self {
            pressure,
            gauges,
            bounds: PressureReferenceBounds {
                storage_bytes,
                joint_storage_bytes,
                work,
                imported_gauge_hash_bytes,
            },
            per_attempt,
        })
    }
    /// Complete conservative owner, scratch and finite-work bounds.
    pub fn bounds(self) -> PressureReferenceBounds {
        self.bounds
    }
    /// Physical sample lattice shared by numerical and analytical pressure.
    pub fn sample_layout(self) -> Layout {
        self.pressure.sample_layout()
    }
    /// Full doubled pressure and force sampling lattice.
    pub fn force_layout(self) -> Layout {
        self.pressure.force_layout()
    }
    /// Imported gauge evidence in exact accepted-clock order.
    pub fn gauges(self) -> [ImportedGauge<'a>; 3] {
        self.gauges
    }
}
fn reservation(pressure: PressureFamilyPlan<'_>) -> Result<usize, SolverError> {
    let source = pressure.source_domain();
    let diagnostic = pressure.diagnostic_domain();
    let samples = pressure.sample_layout();
    let complex_elements = source
        .layout()
        .half_len()
        .checked_mul(3)
        .and_then(|n| {
            diagnostic
                .layout()
                .half_len()
                .checked_mul(7)
                .and_then(|m| n.checked_add(m))
        })
        .ok_or(SolverError::SizeOverflow)?;
    let owned_arrays = complex_elements
        .checked_mul(std::mem::size_of::<nsbu_solver::Complex64>())
        .and_then(|n| {
            samples
                .real_len()
                .checked_mul(
                    3 * std::mem::size_of::<f64>()
                        + std::mem::size_of::<crate::fields::reference::ReferenceEvaluation>(),
                )
                .and_then(|m| n.checked_add(m))
        })
        .ok_or(SolverError::SizeOverflow)?;
    let workspaces = ConservativeWorkspace::reservation(source)?
        .checked_add(DerivativeWorkspace::reservation(diagnostic, samples)?)
        .and_then(|n| n.checked_add(pressure.provider_limits().storage_bytes))
        .ok_or(SolverError::SizeOverflow)?;
    // Fourteen explicitly owned Vec buffers plus bounded object/report metadata.
    owned_arrays
        .checked_add(workspaces)
        .and_then(|n| n.checked_add(14 * 4096))
        .and_then(|n| n.checked_add(std::mem::size_of::<super::PressureReferenceWorkspace<'_>>()))
        .and_then(|n| n.checked_add(std::mem::size_of::<super::PressureReferenceSample<'_>>()))
        .ok_or(SolverError::SizeOverflow)
}

fn require_gauges(
    family: FamilyPlan<'_>,
    gauges: [ImportedGauge<'_>; 3],
) -> Result<(), FamilyError> {
    let times = family.times().as_slice();
    if times.len() != gauges.len()
        || times
            .iter()
            .zip(gauges)
            .any(|(clock, gauge)| *clock != gauge.clock())
    {
        return Err(SolverError::InvalidPayload.into());
    }
    Ok(())
}
