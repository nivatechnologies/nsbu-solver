//! Explicit AVX observer parity and closed-layout admission checks.
use super::*;

/// The closed AVX length set admits no doubled diagnostic below 96, so the smallest
/// source grid the AVX path can observe is N64 (diagnostic 128). On that minimum
/// useful configuration the AVX offline balances must match the reviewed
/// `ReducedObserver` on the same AVX backend, bit-for-bit, on an actual committed
/// CM state (integrated once on an owned-radix RHS; the observer only reads state).
#[test]
fn avx_backend_matches_the_control_observer_on_a_committed_n64_state() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().expect(
        "explicit AVX outcome: this host must admit rustfft-6.4.1-avx-avx2-fma for this evidence",
    );
    let (domain, state) = committed_state([64; 3]);
    let samples = Layout::new([128; 3]).expect("avx samples");
    let catalog = nsbu_solver::spectral::FftCatalog::new(
        backend,
        nsbu_solver::spectral::FftCatalog::reservation(backend).expect("avx reservation"),
    )
    .expect("avx catalog");
    let ledger = OfflineObserver::preflight(domain, samples, 2, backend, 0)
        .expect("source N64 with doubled N128 diagnostic and M128 samples is AVX-admitted");
    assert!(
        ledger.total_bytes <= 2 * 1024 * 1024 * 1024,
        "the bounded N64 check must fit the 2 GiB conservative budget: {}",
        ledger.total_bytes
    );
    assert_eq!(
        OfflineObserver::new(
            domain,
            samples,
            2,
            &catalog,
            0,
            &ledger,
            ledger.total_bytes - 1,
        )
        .err(),
        Some(SolverError::ResourceLimit)
    );
    let control_bytes =
        crate::control::ReducedObserver::preflight(domain, samples, 2, backend).unwrap();
    let mut control =
        crate::control::ReducedObserver::new(domain, samples, 2, &catalog, control_bytes).unwrap();
    let mut offline =
        OfflineObserver::new(domain, samples, 2, &catalog, 0, &ledger, ledger.total_bytes)
            .expect("avx offline observer");
    let before = state_hash(&state);
    let expected = control.sample(&state).expect("avx control sample").balance;
    let observed = offline
        .observe(
            state.clock(),
            [
                state.component(0).unwrap(),
                state.component(1).unwrap(),
                state.component(2).unwrap(),
            ],
        )
        .expect("avx offline observe");
    assert_eq!(expected, observed);
    assert!(
        observed.norms.l2.is_finite()
            && observed.energy.is_finite()
            && observed.enstrophy.is_finite(),
        "AVX balances must be finite on the actual committed state"
    );
    assert_eq!(state_hash(&state), before);
}

/// Honest scope record: the closed length set still refuses the previous N<=16
/// grids; lifting that constraint was only done for the bounded N64 check above.
#[test]
fn avx_backend_still_refuses_the_tiny_n8_grids() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new(N, [1.0; 3], 1.0).expect("domain");
    let samples = Layout::new([16; 3]).expect("samples");
    assert_eq!(
        OfflineObserver::preflight(domain, samples, 2, backend, 0).err(),
        Some(SolverError::InvalidDomain),
        "N8 domain / diagnostic N16 / M16 must stay refused by the closed AVX set"
    );
}
