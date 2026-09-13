//! Standalone, sequential controls for a prospective W3 layout-768 admission.
use nsbu_benchmarks::provider::parallel_reduced::{
    ParallelReducedV2Force, ParallelReducedV2ForceW3,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::{
        forcing::{ForceLimits, ForceWork, PrescribedForce},
        kernel::RightHandSide,
        rhs::SpectralRhs,
    },
    spectral::{FftBackend, FftCatalog, W3FftIdentity, W3FftMode, W3FftPool},
    Complex64, SolverError,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, env};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const RETAINED: usize = 512;
const LAYOUT: usize = 768;
const WORKERS: usize = 32;
const FORWARD_ADDITIONAL: usize = 14_539_902_720;
const BIDIRECTIONAL_ADDITIONAL: usize = 21_787_660_160;

fn main() -> Result<(), String> {
    match env::args().nth(1).as_deref() {
        Some("preflight") => preflight(),
        Some("force") => gated("NSBU_RUN_W3_768_FORCE_CONTROL", force_control),
        Some("rhs") => gated("NSBU_RUN_W3_768_RHS_CONTROL", rhs_control),
        _ => Err("usage: p10-w3-768-controls preflight | force | rhs".into()),
    }
}

fn backend() -> Result<FftBackend, String> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().map_err(debug)?;
    Ok(backend)
}

fn admission() -> Result<bool, String> {
    let backend = backend()?;
    let layout = Layout::new([LAYOUT; 3]).map_err(debug)?;
    match (
        W3FftPool::additional_reservation_with_backend(layout, backend, W3FftMode::Forward),
        W3FftPool::additional_reservation_with_backend(layout, backend, W3FftMode::Bidirectional),
    ) {
        (Ok(forward), Ok(bidirectional))
            if forward == FORWARD_ADDITIONAL && bidirectional == BIDIRECTIONAL_ADDITIONAL =>
        {
            Ok(true)
        }
        (Err(SolverError::InvalidPayload), Err(SolverError::InvalidPayload)) => Ok(false),
        _ => Err("layout768 W3 admission is inconsistent with reviewed formulas".into()),
    }
}

fn preflight() -> Result<(), String> {
    if admission()? {
        println!("status=admitted_preallocation layout=768 forward={FORWARD_ADDITIONAL} bidirectional={BIDIRECTIONAL_ADDITIONAL}");
    } else {
        println!("status=blocked_current_closed_admission layout=768 expected_forward={FORWARD_ADDITIONAL} expected_bidirectional={BIDIRECTIONAL_ADDITIONAL}");
    }
    Ok(())
}

fn gated(name: &str, run: fn() -> Result<(), String>) -> Result<(), String> {
    if env::var(name).as_deref() != Ok("1") {
        return Err(format!(
            "refused: set {name}=1 after reviewed resource admission"
        ));
    }
    if !admission()? {
        return Err("refused: current W3 admission does not include layout768".into());
    }
    run()
}

fn force_control() -> Result<(), String> {
    let backend = backend()?;
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?;
    let samples = Layout::new([LAYOUT; 3]).map_err(debug)?;
    let catalog = catalog(backend)?;
    let serial_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .map_err(debug)?;
    let w3_limits =
        ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .map_err(debug)?;
    if w3_limits
        .storage_bytes
        .checked_sub(serial_limits.storage_bytes)
        != Some(FORWARD_ADDITIONAL)
    {
        return Err("force W3 incremental reservation mismatch".into());
    }
    if !matches!(
        ParallelReducedV2ForceW3::new_with_catalog(
            domain,
            samples,
            WORKERS,
            &catalog,
            w3_limits.storage_bytes - 1
        ),
        Err(SolverError::ResourceLimit)
    ) {
        return Err("force one-byte-under cap did not refuse".into());
    }
    let clock = TickClock::restore(-20, 8192, 4096, 4096).map_err(debug)?;
    let expected = {
        let mut serial = ParallelReducedV2Force::new_with_catalog(
            domain,
            samples,
            WORKERS,
            &catalog,
            serial_limits.storage_bytes,
        )
        .map_err(debug)?;
        let mut output = field(domain);
        let work = serial
            .evaluate(
                clock,
                serial_limits,
                output.each_mut().map(Vec::as_mut_slice),
            )
            .map_err(debug)?;
        (output, work)
    };
    let mut w3 = ParallelReducedV2ForceW3::new_with_catalog(
        domain,
        samples,
        WORKERS,
        &catalog,
        w3_limits.storage_bytes,
    )
    .map_err(debug)?;
    require_identity(
        w3.w3_fft_identity(),
        samples,
        backend,
        W3FftMode::Forward,
        FORWARD_ADDITIONAL,
    )?;
    let mut actual = field(domain);
    let measured = Region::new(GLOBAL);
    let mut work = None;
    for _ in 0..3 {
        work = Some(
            w3.evaluate(clock, w3_limits, actual.each_mut().map(Vec::as_mut_slice))
                .map_err(debug)?,
        );
    }
    let allocations = measured.change();
    same_bits(&actual, &expected.0)?;
    same_work(work.ok_or("missing force work")?, expected.1)?;
    zero_steady(
        allocations.allocations,
        allocations.deallocations,
        allocations.reallocations,
    )?;
    println!("control=force-forward retained=512 layout=768 serial_bytes={} w3_bytes={} additional={} repeated=3 bitwise=true steady_allocations=0", serial_limits.storage_bytes, w3_limits.storage_bytes, FORWARD_ADDITIONAL);
    Ok(())
}

fn rhs_control() -> Result<(), String> {
    let backend = backend()?;
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?;
    let catalog = catalog(backend)?;
    let limits = FixtureForce::LIMITS;
    let serial_bytes =
        SpectralRhs::<FixtureForce>::reservation_with_fft_backend(domain, limits, backend)
            .map_err(debug)?;
    let w3_bytes =
        SpectralRhs::<FixtureForce>::reservation_with_w3_fft_backend(domain, limits, backend)
            .map_err(debug)?;
    if w3_bytes.checked_sub(serial_bytes) != Some(BIDIRECTIONAL_ADDITIONAL) {
        return Err("RHS W3 incremental reservation mismatch".into());
    }
    if !matches!(
        SpectralRhs::new_with_catalog_w3(domain, FixtureForce, 1.0e9, &catalog, w3_bytes - 1),
        Err(SolverError::ResourceLimit)
    ) {
        return Err("RHS one-byte-under cap did not refuse".into());
    }
    let clock = TickClock::restore(-20, 8192, 4096, 4096).map_err(debug)?;
    let state = fixture_state(domain)?;
    let expected = {
        let mut rhs =
            SpectralRhs::new_with_catalog(domain, FixtureForce, 1.0e9, &catalog, serial_bytes)
                .map_err(debug)?;
        evaluate_rhs(&mut rhs, &state, clock)?
    };
    let mut rhs = SpectralRhs::new_with_catalog_w3(domain, FixtureForce, 1.0e9, &catalog, w3_bytes)
        .map_err(debug)?;
    require_identity(
        rhs.w3_fft_identity().ok_or("missing RHS W3 identity")?,
        domain.padded_layout().map_err(debug)?,
        backend,
        W3FftMode::Bidirectional,
        BIDIRECTIONAL_ADDITIONAL,
    )?;
    let measured = Region::new(GLOBAL);
    let mut actual = None;
    for _ in 0..3 {
        actual = Some(evaluate_rhs(&mut rhs, &state, clock)?);
    }
    let allocations = measured.change();
    let actual = actual.ok_or("missing RHS output")?;
    same_bits(&actual.0, &expected.0)?;
    if actual.1 != expected.1 {
        return Err("RHS consumption differs".into());
    }
    zero_steady(
        allocations.allocations,
        allocations.deallocations,
        allocations.reallocations,
    )?;
    println!("control=rhs-bidirectional retained=512 padded=768 serial_bytes={serial_bytes} w3_bytes={w3_bytes} additional={BIDIRECTIONAL_ADDITIONAL} repeated=3 bitwise=true steady_allocations=0");
    Ok(())
}

fn catalog(backend: FftBackend) -> Result<FftCatalog, String> {
    let bytes = FftCatalog::reservation(backend).map_err(debug)?;
    FftCatalog::new(backend, bytes).map_err(debug)
}
fn field(domain: Domain) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()])
}
fn fixture_state(domain: Domain) -> Result<[Vec<Complex64>; 3], String> {
    let mut state = field(domain);
    for (axis, value) in [(1, 0.25), (2, -0.125)] {
        for mode in [[1, 0, 0], [-1, 0, 0]] {
            let index = domain.layout().locate(mode).map_err(debug)?.0;
            state[axis][index] = Complex64::new(value, 0.0);
        }
    }
    Ok(state)
}
fn evaluate_rhs(
    rhs: &mut SpectralRhs<FixtureForce>,
    state: &[Vec<Complex64>; 3],
    clock: TickClock,
) -> Result<([Vec<Complex64>; 3], [usize; 3]), String> {
    rhs.begin_attempt(clock, 64).map_err(debug)?;
    let mut output = field(Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?);
    rhs.evaluate(
        state.each_ref().map(Vec::as_slice),
        clock,
        output.each_mut().map(Vec::as_mut_slice),
    )
    .map_err(debug)?;
    Ok((output, rhs.consumption()))
}
fn require_identity(
    actual: W3FftIdentity,
    layout: Layout,
    backend: FftBackend,
    mode: W3FftMode,
    bytes: usize,
) -> Result<(), String> {
    if actual
        == (W3FftIdentity {
            layout,
            backend,
            width: 3,
            mode,
            additional_bytes: bytes,
        })
    {
        Ok(())
    } else {
        Err("W3 identity mismatch".into())
    }
}
fn same_bits(actual: &[Vec<Complex64>; 3], expected: &[Vec<Complex64>; 3]) -> Result<(), String> {
    if actual
        .iter()
        .flatten()
        .zip(expected.iter().flatten())
        .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
    {
        Ok(())
    } else {
        Err("coefficient bits differ".into())
    }
}
fn same_work(actual: ForceWork, expected: ForceWork) -> Result<(), String> {
    if actual.work_units == expected.work_units
        && actual.scalar_transforms == expected.scalar_transforms
    {
        Ok(())
    } else {
        Err("force work differs".into())
    }
}
fn zero_steady(a: usize, d: usize, r: usize) -> Result<(), String> {
    if (a, d, r) == (0, 0, 0) {
        Ok(())
    } else {
        Err(format!("steady allocation change ({a},{d},{r})"))
    }
}
fn debug(error: SolverError) -> String {
    format!("{error:?}")
}

struct FixtureForce;
impl FixtureForce {
    const LIMITS: ForceLimits = ForceLimits {
        storage_bytes: 0,
        work_units: 1,
        scalar_transforms: 0,
        remaining_divisor: 20,
    };
}
impl PrescribedForce for FixtureForce {
    fn limits(&self) -> Option<ForceLimits> {
        Some(Self::LIMITS)
    }
    fn evaluate(
        &mut self,
        _: TickClock,
        limit: ForceLimits,
        mut out: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limit != Self::LIMITS {
            return Err(SolverError::ProviderBudgetExceeded);
        };
        for values in &mut out {
            values.fill(Complex64::new(0.0, 0.0));
        }
        out[0][0] = Complex64::new(0.125, 0.0);
        Ok(ForceWork {
            work_units: 1,
            scalar_transforms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_768_admission_matches_formulas_and_1024_remains_closed() {
        assert_eq!(FORWARD_ADDITIONAL, 14_539_902_720);
        assert_eq!(BIDIRECTIONAL_ADDITIONAL, 21_787_660_160);
        let b = FftBackend::RustFft6_4_1AvxFma;
        if b.ensure_available().is_ok() {
            let admitted = Layout::new([LAYOUT; 3]).unwrap();
            assert_eq!(
                W3FftPool::additional_reservation_with_backend(admitted, b, W3FftMode::Forward),
                Ok(FORWARD_ADDITIONAL)
            );
            assert_eq!(
                W3FftPool::additional_reservation_with_backend(
                    admitted,
                    b,
                    W3FftMode::Bidirectional
                ),
                Ok(BIDIRECTIONAL_ADDITIONAL)
            );
            let excluded = Layout::new([1024; 3]).unwrap();
            assert_eq!(
                W3FftPool::additional_reservation_with_backend(excluded, b, W3FftMode::Forward),
                Err(SolverError::InvalidPayload)
            );
            assert_eq!(
                W3FftPool::additional_reservation_with_backend(
                    excluded,
                    b,
                    W3FftMode::Bidirectional
                ),
                Err(SolverError::InvalidPayload)
            );
        }
    }
}
