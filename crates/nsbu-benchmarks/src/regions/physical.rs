//! Complete physical tensor samples feed unchanged v2 geometric partitions.
use super::{RegionalError, RegionalReport, RegionalTensorErrors};
use crate::BenchmarkError;
use nsbu_solver::{diagnostics::physical::PhysicalComparison, domain::TickClock};

/// Apply v2 masks to every sample in a complete scalar/vector/tensor comparison.
/// This validates unit-cube/viscosity-one geometry and the v2 clock, not the supplied
/// spectra's mathematical identity, actual-state origin or synchronization. The owning
/// experiment must bind those independently before using these empirical measurements.
/// Root work, attempt count and relative floor are explicit; no heap storage is allocated.
pub fn measure(
    input: &PhysicalComparison<'_>,
    clock: TickClock,
    root_budget: usize,
    maximum_attempts: usize,
) -> Result<RegionalReport, RegionalError> {
    if input
        .domains()
        .into_iter()
        .any(|domain| domain.lengths() != [1.0; 3] || domain.viscosity() != 1.0)
    {
        return Err(BenchmarkError::InvalidInput.into());
    }
    match input.quantity().components() {
        1 => collect::<1>(input, clock, root_budget, maximum_attempts),
        3 => collect::<3>(input, clock, root_budget, maximum_attempts),
        9 => collect::<9>(input, clock, root_budget, maximum_attempts),
        27 => collect::<27>(input, clock, root_budget, maximum_attempts),
        _ => Err(BenchmarkError::InvalidInput.into()),
    }
}
fn collect<const C: usize>(
    input: &PhysicalComparison<'_>,
    clock: TickClock,
    root_budget: usize,
    maximum_attempts: usize,
) -> Result<RegionalReport, RegionalError> {
    let mut errors = RegionalTensorErrors::<C>::new(
        clock,
        input.sample_layout(),
        root_budget,
        maximum_attempts,
        input.global().relative_floor,
    )?;
    for (&difference, &reference) in input
        .error_magnitudes()
        .iter()
        .zip(input.reference_magnitudes())
    {
        errors.push_magnitudes(difference, reference)?;
    }
    Ok(errors.report()?)
}
