//! Standalone, sequential controls for a prospective W3 layout-768 admission.
use nsbu_benchmarks::{
    provider::parallel_reduced::{ParallelReducedV2Force, ParallelReducedV2ForceW3},
    CASE_SHA256,
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
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, env};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const RETAINED: usize = 512;
const LAYOUT: usize = 768;
const WORKERS: usize = 32;
const UNTILED_FORWARD_ADDITIONAL: usize = 14_539_902_720;
const UNTILED_BIDIRECTIONAL_ADDITIONAL: usize = 21_787_660_160;
const TILED_FORWARD_ADDITIONAL: usize = 14_540_099_376;
const TILED_BIDIRECTIONAL_ADDITIONAL: usize = 21_787_856_816;
const CONTROL_OVERHEAD: usize = 64 * 1024;
const FORCE_CAP_BYTES: usize = 64 * 1024 * 1024 * 1024;
const RHS_CAP_BYTES: usize = 96 * 1024 * 1024 * 1024;

#[derive(Clone, Copy)]
struct ControlPlan {
    serial_peak: usize,
    w3_peak: usize,
    cap: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceVariant {
    Untiled,
    Tiled,
}
impl SourceVariant {
    fn label(self) -> &'static str {
        match self {
            Self::Untiled => "untiled",
            Self::Tiled => "tiled",
        }
    }
    fn forward(self) -> usize {
        match self {
            Self::Untiled => UNTILED_FORWARD_ADDITIONAL,
            Self::Tiled => TILED_FORWARD_ADDITIONAL,
        }
    }
    fn bidirectional(self) -> usize {
        match self {
            Self::Untiled => UNTILED_BIDIRECTIONAL_ADDITIONAL,
            Self::Tiled => TILED_BIDIRECTIONAL_ADDITIONAL,
        }
    }
}
impl ControlPlan {
    fn peak(self) -> usize {
        self.serial_peak.max(self.w3_peak)
    }
    fn require(self, name: &str) -> Result<(), String> {
        (self.peak() <= self.cap)
            .then_some(())
            .ok_or_else(|| format!("{name} peak {} exceeds cap {}", self.peak(), self.cap))
    }
}

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

fn admission() -> Result<SourceVariant, String> {
    let backend = backend()?;
    let layout = Layout::new([LAYOUT; 3]).map_err(debug)?;
    match (
        W3FftPool::additional_reservation_with_backend(layout, backend, W3FftMode::Forward),
        W3FftPool::additional_reservation_with_backend(layout, backend, W3FftMode::Bidirectional),
    ) {
        (Ok(UNTILED_FORWARD_ADDITIONAL), Ok(UNTILED_BIDIRECTIONAL_ADDITIONAL)) => {
            Ok(SourceVariant::Untiled)
        }
        (Ok(TILED_FORWARD_ADDITIONAL), Ok(TILED_BIDIRECTIONAL_ADDITIONAL)) => {
            Ok(SourceVariant::Tiled)
        }
        _ => Err("layout768 W3 admission is inconsistent with reviewed formulas".into()),
    }
}

fn preflight() -> Result<(), String> {
    let variant = admission()?;
    let force = force_plan()?;
    let rhs = rhs_plan()?;
    force.require("force")?;
    rhs.require("rhs")?;
    println!("status=admitted_preallocation variant={} layout=768 forward={} bidirectional={} force_serial_peak={} force_w3_peak={} force_peak={} force_cap={} rhs_serial_peak={} rhs_w3_peak={} rhs_peak={} rhs_cap={} source_case_sha256={CASE_SHA256}", variant.label(), variant.forward(), variant.bidirectional(), force.serial_peak, force.w3_peak, force.peak(), force.cap, rhs.serial_peak, rhs.w3_peak, rhs.peak(), rhs.cap);
    Ok(())
}

fn field_bytes(domain: Domain) -> Result<usize, String> {
    domain
        .layout()
        .half_len()
        .checked_mul(3 * std::mem::size_of::<Complex64>())
        .ok_or("field byte overflow".into())
}
fn catalog_bytes(backend: FftBackend) -> Result<usize, String> {
    FftCatalog::reservation(backend).map_err(debug)
}
fn force_plan() -> Result<ControlPlan, String> {
    let backend = backend()?;
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?;
    let samples = Layout::new([LAYOUT; 3]).map_err(debug)?;
    let serial =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .map_err(debug)?
            .storage_bytes;
    let w3 =
        ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .map_err(debug)?
            .storage_bytes;
    let output = field_bytes(domain)?;
    let catalog = catalog_bytes(backend)?;
    Ok(ControlPlan {
        serial_peak: catalog + serial + output + CONTROL_OVERHEAD,
        w3_peak: catalog + w3 + 2 * output + CONTROL_OVERHEAD,
        cap: FORCE_CAP_BYTES,
    })
}
fn rhs_plan() -> Result<ControlPlan, String> {
    let backend = backend()?;
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?;
    let limits = FixtureForce::LIMITS;
    let serial = SpectralRhs::<FixtureForce>::reservation_with_fft_backend(domain, limits, backend)
        .map_err(debug)?;
    let w3 = SpectralRhs::<FixtureForce>::reservation_with_w3_fft_backend(domain, limits, backend)
        .map_err(debug)?;
    let field = field_bytes(domain)?;
    let catalog = catalog_bytes(backend)?;
    Ok(ControlPlan {
        serial_peak: catalog + serial + 2 * field + CONTROL_OVERHEAD,
        w3_peak: catalog + w3 + 3 * field + CONTROL_OVERHEAD,
        cap: RHS_CAP_BYTES,
    })
}
fn gated(name: &str, run: fn() -> Result<(), String>) -> Result<(), String> {
    if env::var(name).as_deref() != Ok("1") {
        return Err(format!(
            "refused: set {name}=1 after reviewed resource admission"
        ));
    }
    admission()?;
    run()
}

fn force_control() -> Result<(), String> {
    force_plan()?.require("force")?;
    let variant = admission()?;
    let forward_additional = variant.forward();
    let backend = backend()?;
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?;
    let samples = Layout::new([LAYOUT; 3]).map_err(debug)?;
    let serial_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .map_err(debug)?;
    let w3_limits =
        ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .map_err(debug)?;
    if w3_limits
        .storage_bytes
        .checked_sub(serial_limits.storage_bytes)
        != Some(forward_additional)
    {
        return Err("force W3 incremental reservation mismatch".into());
    }
    let catalog = catalog(backend)?;
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
        forward_additional,
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
    let output_sha256 = field_sha256(&actual);
    println!("control=force-forward variant={} retained=512 layout=768 serial_bytes={} w3_bytes={} additional={} repeated=3 bitwise=true steady_allocations=0 output_sha256={output_sha256:x} source_case_sha256={CASE_SHA256}", variant.label(), serial_limits.storage_bytes, w3_limits.storage_bytes, forward_additional);
    Ok(())
}

fn rhs_control() -> Result<(), String> {
    rhs_plan()?.require("rhs")?;
    let variant = admission()?;
    let bidirectional_additional = variant.bidirectional();
    let backend = backend()?;
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).map_err(debug)?;
    let limits = FixtureForce::LIMITS;
    let serial_bytes =
        SpectralRhs::<FixtureForce>::reservation_with_fft_backend(domain, limits, backend)
            .map_err(debug)?;
    let w3_bytes =
        SpectralRhs::<FixtureForce>::reservation_with_w3_fft_backend(domain, limits, backend)
            .map_err(debug)?;
    if w3_bytes.checked_sub(serial_bytes) != Some(bidirectional_additional) {
        return Err("RHS W3 incremental reservation mismatch".into());
    }
    let catalog = catalog(backend)?;
    if !matches!(
        SpectralRhs::new_with_catalog_w3(
            domain,
            FixtureForce { mean: 0.125 },
            1.0e9,
            &catalog,
            w3_bytes - 1
        ),
        Err(SolverError::ResourceLimit)
    ) {
        return Err("RHS one-byte-under cap did not refuse".into());
    }
    let clock = TickClock::restore(-20, 8192, 4096, 4096).map_err(debug)?;
    let state = fixture_state(domain)?;
    let mut expected_output = field(domain);
    let expected = {
        let mut rhs = SpectralRhs::new_with_catalog(
            domain,
            FixtureForce { mean: 0.125 },
            1.0e9,
            &catalog,
            serial_bytes,
        )
        .map_err(debug)?;
        evaluate_rhs(&mut rhs, &state, clock, &mut expected_output)?
    };
    let mut rhs = SpectralRhs::new_with_catalog_w3(
        domain,
        FixtureForce { mean: 0.125 },
        1.0e9,
        &catalog,
        w3_bytes,
    )
    .map_err(debug)?;
    require_identity(
        rhs.w3_fft_identity().ok_or("missing RHS W3 identity")?,
        domain.padded_layout().map_err(debug)?,
        backend,
        W3FftMode::Bidirectional,
        bidirectional_additional,
    )?;
    let mut actual_output = field(domain);
    let measured = Region::new(GLOBAL);
    let mut actual = None;
    for _ in 0..3 {
        actual = Some(evaluate_rhs(&mut rhs, &state, clock, &mut actual_output)?);
    }
    let allocations = measured.change();
    let actual = actual.ok_or("missing RHS output")?;
    same_bits(&actual_output, &expected_output)?;
    if actual != expected {
        return Err("RHS consumption differs".into());
    }
    zero_steady(
        allocations.allocations,
        allocations.deallocations,
        allocations.reallocations,
    )?;
    let output_sha256 = field_sha256(&actual_output);
    println!("control=rhs-bidirectional variant={} retained=512 padded=768 serial_bytes={serial_bytes} w3_bytes={w3_bytes} additional={bidirectional_additional} repeated=3 bitwise=true steady_allocations=0 output_sha256={output_sha256:x} source_case_sha256={CASE_SHA256}", variant.label());
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
    let put = |state: &mut [Vec<Complex64>; 3],
               axis: usize,
               mode: [isize; 3],
               value: Complex64|
     -> Result<(), String> {
        state[axis][domain.layout().locate(mode).map_err(debug)?.0] = value;
        Ok(())
    };
    // u=(A cos y, B sin z, C cos x): strict Hermitian and divergence-free, with mixed directions/phases.
    for mode in [[0, 1, 0], [0, -1, 0]] {
        put(&mut state, 0, mode, Complex64::new(0.25, 0.0))?;
    }
    put(&mut state, 1, [0, 0, 1], Complex64::new(0.0, -0.125))?;
    for mode in [[1, 0, 0], [-1, 0, 0]] {
        put(&mut state, 2, mode, Complex64::new(0.2, 0.0))?;
    }
    Ok(state)
}
fn evaluate_rhs(
    rhs: &mut SpectralRhs<FixtureForce>,
    state: &[Vec<Complex64>; 3],
    clock: TickClock,
    output: &mut [Vec<Complex64>; 3],
) -> Result<[usize; 3], String> {
    rhs.begin_attempt(clock, 64).map_err(debug)?;
    rhs.evaluate(
        state.each_ref().map(Vec::as_slice),
        clock,
        output.each_mut().map(Vec::as_mut_slice),
    )
    .map_err(debug)?;
    Ok(rhs.consumption())
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
    if actual.iter().zip(expected).any(|(a, b)| a.len() != b.len()) {
        return Err("coefficient component lengths differ".into());
    }
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
fn field_sha256(field: &[Vec<Complex64>; 3]) -> sha2::digest::Output<Sha256> {
    let mut hash = Sha256::new();
    for value in field.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    hash.finalize()
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

struct FixtureForce {
    mean: f64,
}
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
        out[0][0] = Complex64::new(self.mean, 0.0);
        Ok(ForceWork {
            work_units: 1,
            scalar_transforms: 0,
        })
    }
}

#[cfg(test)]
mod tests;
