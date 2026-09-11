//! Isolated complete reduced-provider resource admission and allocation-free request checks.
use nsbu_benchmarks::provider::reduced::ReducedV2Force;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64,
};
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn fingerprint(values: &[Vec<Complex64>; 3]) -> [u8; 32] {
    let mut hash = Sha256::new();
    for value in values.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    hash.finalize().into()
}

fn main() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let sampled = Layout::new([6, 8, 12]).unwrap();
    let admission = Region::new(GLOBAL);
    let limits = ReducedV2Force::preflight(domain, sampled).unwrap();
    assert!(ReducedV2Force::new(domain, sampled, limits.storage_bytes - 1).is_err());
    let planned = admission.change();
    assert_eq!(
        (
            planned.allocations,
            planned.deallocations,
            planned.reallocations
        ),
        (0, 0, 0)
    );
    let construction = Region::new(GLOBAL);
    let mut provider = ReducedV2Force::new(domain, sampled, limits.storage_bytes).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= limits.storage_bytes);
    let mut values: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let steady = Region::new(GLOBAL);
    for tick in [1, 4, 2, 1, 0] {
        let clock = TickClock::restore(-10, 8, tick, 8 - tick).unwrap();
        let work = provider
            .evaluate(clock, limits, values.each_mut().map(Vec::as_mut_slice))
            .unwrap();
        assert_eq!(
            work.work_units,
            sampled.real_len() + provider.last_root_iterations()
        );
        assert!(work.work_units <= limits.work_units);
        assert_eq!(work.scalar_transforms, 3);
    }
    let before = fingerprint(&values);
    let mut bad = limits;
    bad.scalar_transforms = 2;
    assert!(provider
        .evaluate(
            TickClock::from_rest(-10, 8).unwrap(),
            bad,
            values.each_mut().map(Vec::as_mut_slice)
        )
        .is_err());
    assert!(provider
        .evaluate(
            TickClock::from_rest(-9, 8).unwrap(),
            limits,
            values.each_mut().map(Vec::as_mut_slice)
        )
        .is_err());
    let [a, b, c] = values.each_mut();
    assert!(provider
        .evaluate(
            TickClock::from_rest(-10, 8).unwrap(),
            limits,
            [&mut a[..1], b, c]
        )
        .is_err());
    assert_eq!(fingerprint(&values), before);
    let measured = steady.change();
    assert_eq!(
        (
            measured.allocations,
            measured.deallocations,
            measured.reallocations
        ),
        (0, 0, 0)
    );
    println!("reduced provider admission_allocations=0 constructor_heap_bytes={} declared_bytes={} steady_allocations=0 steady_deallocations=0 steady_reallocations=0 completed_requests=5 refused_requests=3",built.bytes_allocated,limits.storage_bytes);
}
