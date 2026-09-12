//! Explicitly gated, high-memory arithmetic control for the M512 W3 force owner.
use m512_alloc_trace::{
    begin_measurement, finish_measurement, print_traces, set_phase, TraceAllocator,
};
use nsbu_benchmarks::{
    provider::parallel_reduced::{ParallelReducedV2Force, ParallelReducedV2ForceW3},
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceWork, PrescribedForce},
    spectral::{FftBackend, FftCatalog, W3FftIdentity, W3FftMode},
    Complex64,
};

#[global_allocator]
static GLOBAL: TraceAllocator = TraceAllocator;

const RETAINED: usize = 384;
const SAMPLED: usize = 512;
const WORKERS: usize = 32;
const W3_ADDITIONAL_BYTES: usize = 4_318_334_720;

#[test]
#[ignore = "requires explicit NSBU_RUN_M512_W3_FORCE_CONTROL=1 and about 25 GiB"]
fn m512_w3_force_matches_serial_bits_work_and_steady_allocation() {
    assert_eq!(
        std::env::var("NSBU_RUN_M512_W3_FORCE_CONTROL").as_deref(),
        Ok("1"),
        "set NSBU_RUN_M512_W3_FORCE_CONTROL=1 after admitting the high-memory control"
    );
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().unwrap();
    let domain = Domain::new([RETAINED; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([SAMPLED; 3]).unwrap();
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let serial_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .unwrap();
    let w3_limits =
        ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, WORKERS, backend)
            .unwrap();
    assert_eq!(
        w3_limits.storage_bytes - serial_limits.storage_bytes,
        W3_ADDITIONAL_BYTES
    );

    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    let mut serial = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        WORKERS,
        &catalog,
        serial_limits.storage_bytes,
    )
    .unwrap();
    let mut expected = output(domain);
    let expected_work = serial
        .evaluate(
            clock,
            serial_limits,
            expected.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    drop(serial);

    let mut w3 = ParallelReducedV2ForceW3::new_with_catalog(
        domain,
        samples,
        WORKERS,
        &catalog,
        w3_limits.storage_bytes,
    )
    .unwrap();
    assert_eq!(
        w3.w3_fft_identity(),
        W3FftIdentity {
            layout: samples,
            backend,
            width: 3,
            mode: W3FftMode::Forward,
            additional_bytes: W3_ADDITIONAL_BYTES,
        }
    );
    let mut actual = output(domain);
    begin_measurement();
    let actual_work = repeat(&mut w3, clock, w3_limits, &mut actual);
    let allocations = finish_measurement();
    print_traces();

    assert_bits(&actual, &expected);
    assert_work(actual_work, expected_work);
    assert_eq!(
        (
            allocations.allocations,
            allocations.deallocations,
            allocations.reallocations,
        ),
        (0, 0, 0)
    );
    println!(
        "case_sha256={CASE_SHA256} retained={RETAINED} sampled={SAMPLED} workers={WORKERS} repeated=3 steady_allocations=0"
    );
}

fn repeat(
    provider: &mut ParallelReducedV2ForceW3,
    clock: TickClock,
    limits: nsbu_solver::integrators::forcing::ForceLimits,
    output: &mut [Vec<Complex64>; 3],
) -> ForceWork {
    let mut last = None;
    for phase in 1..=3 {
        set_phase(phase);
        last = Some(
            provider
                .evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))
                .unwrap(),
        );
    }
    last.unwrap()
}

fn output(domain: Domain) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()])
}

fn assert_bits(actual: &[Vec<Complex64>; 3], expected: &[Vec<Complex64>; 3]) {
    for (actual, expected) in actual.iter().flatten().zip(expected.iter().flatten()) {
        assert_eq!(actual.re.to_bits(), expected.re.to_bits());
        assert_eq!(actual.im.to_bits(), expected.im.to_bits());
    }
}

fn assert_work(actual: ForceWork, expected: ForceWork) {
    assert_eq!(actual.work_units, expected.work_units);
    assert_eq!(actual.scalar_transforms, expected.scalar_transforms);
}
