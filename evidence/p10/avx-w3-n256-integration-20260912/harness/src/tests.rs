use super::*;
use nsbu_solver::{
    integrators::{indicator::Indicators, rhs::SpectralRhs},
    Complex64,
};

struct Outcome {
    components: [Vec<Complex64>; 3],
    indicators: Indicators,
    consumption: [usize; 3],
    hit_miss: [usize; 2],
}

fn attempt(w3: bool) -> Outcome {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let force_limits = CachedReducedForce::preflight(domain, samples, 3, backend, w3).unwrap();
    let rhs_bytes = if w3 {
        SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
            domain,
            force_limits,
            backend,
        )
    } else {
        SpectralRhs::<CachedReducedForce>::reservation_with_fft_backend(
            domain,
            force_limits,
            backend,
        )
    }
    .unwrap();
    let attempt_bytes =
        AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews).unwrap();
    let resources = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: catalog_bytes,
            force: rhs_bytes,
            diagnostics: attempt_bytes,
            overhead: OVERHEAD,
        },
        usize::MAX,
        Epoch(0),
    )
    .unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let force =
        CachedReducedForce::new(domain, samples, 3, &catalog, force_limits.storage_bytes, w3)
            .unwrap();
    let mut rhs = if w3 {
        SpectralRhs::new_with_catalog_w3(domain, force, 0.3, &catalog, rhs_bytes)
    } else {
        SpectralRhs::new_with_catalog(domain, force, 0.3, &catalog, rhs_bytes)
    }
    .unwrap();
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let mut state = SpectralState::from_rest(resources, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(resources, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(resources, Method::CoxMatthews).unwrap();
    let result = workspace
        .try_advance(
            &state,
            &mut candidate,
            16,
            Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
            &mut rhs,
        )
        .unwrap();
    let accepted = result.accepted.unwrap();
    commit_candidate(resources, &mut state, &mut candidate, accepted).unwrap();
    Outcome {
        components: std::array::from_fn(|axis| state.component(axis).unwrap().to_vec()),
        indicators: result.indicators,
        consumption: rhs.consumption(),
        hit_miss: rhs.provider().hit_miss(),
    }
}

#[test]
fn fixture_cache_whole_attempt_matches_serial_bits_and_work() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let serial = attempt(false);
    let w3 = attempt(true);
    assert_eq!(w3.components, serial.components);
    assert_eq!(w3.indicators, serial.indicators);
    assert_eq!(w3.consumption, serial.consumption);
    assert_eq!(w3.hit_miss, [7, 5]);
    assert_eq!(w3.hit_miss, serial.hit_miss);
}
