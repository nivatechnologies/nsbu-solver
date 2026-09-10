//! Complete coarse exact-v2 trajectory from rest, compared with independent direct-DFT evolution.
use nsbu_benchmarks::provider::V2Force;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        method::Method,
        rhs::SpectralRhs,
        trajectory::{FixedRun, RunLimits, StopReason},
        transaction::CandidateState,
    },
};

pub fn trajectory(method: Method) -> SpectralState {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let limits = V2Force::preflight(domain, domain.layout()).unwrap();
    let rhs_bytes = SpectralRhs::<V2Force>::reservation(domain, limits).unwrap();
    let cap = 2 * 1024 * 1024;
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: rhs_bytes,
            diagnostics: AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
            overhead: 1024 * 1024,
        },
        cap,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
    let provider = V2Force::new(domain, domain.layout(), limits.storage_bytes).unwrap();
    let mut rhs = SpectralRhs::new(domain, provider, 0.3, rhs_bytes).unwrap();
    let report = FixedRun::new(&mut state, &mut candidate, &mut workspace, &mut rhs)
        .execute(
            RunLimits {
                endpoint: 4096,
                step_ticks: 128,
                maximum_attempts: 32,
            },
            Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        )
        .unwrap();
    assert_eq!(report.reason, StopReason::EndpointReached);
    assert_eq!(report.clock.remaining(), 4096);
    assert_eq!(report.committed, 32);
    assert_eq!(report.attempted, 32);
    assert_eq!(state.accepted_steps(), 32);
    let calls = match method {
        Method::CoxMatthews => 12,
        Method::HochbruckOstermann => 15,
    };
    assert_eq!(rhs.consumption()[0], calls);
    assert_eq!(rhs.consumption()[2], calls * 13);
    let ratios = report.maximum_local_ratios;
    let reserved = plan.total();
    println!("concentrating diagnostic-only method={method:?} n=4 steps=32 fine_steps=64 reference_assignments=0 accepted_pde_windows=0 calls_per_attempt={calls} reserved_bytes={reserved} cap_bytes={cap} maximum_local_ratios={ratios:?}");
    state
}
