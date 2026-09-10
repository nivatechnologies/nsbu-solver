//! Physical payload round trips reproduce accepted and rejected attempts with fresh scratch.
mod source_contract;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        method::Method,
        transaction::{commit_candidate, CandidateState},
    },
    lineage::PhysicalImage,
    Complex64, SolverError,
};

fn source() -> source_contract::Source {
    let amplitudes = std::array::from_fn(|axis| {
        [
            Complex64::new((axis + 1) as f64, 0.0),
            Complex64::new(1.0, (axis + 2) as f64),
        ]
    });
    source_contract::Source::new(usize::MAX, amplitudes)
}
fn plan(method: Method) -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0, 2.0, 3.0], 0.5).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
            overhead: 4096,
        },
        1 << 20,
        Epoch(42),
    )
    .unwrap()
}
fn tolerance(value: f64) -> Tolerances {
    Tolerances {
        absolute: [value; 2],
        relative: [0.0; 2],
    }
}
fn compare(left: &SpectralState, right: &SpectralState) {
    assert_eq!(left.plan(), right.plan());
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.epoch(), right.epoch());
    assert_eq!(left.accepted_steps(), right.accepted_steps());
    for axis in 0..3 {
        let a = left.component(axis).unwrap();
        let b = right.component(axis).unwrap();
        assert_ne!(a.as_ptr(), b.as_ptr());
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert_eq!(x.re.to_bits(), y.re.to_bits());
            assert_eq!(x.im.to_bits(), y.im.to_bits());
        }
    }
}

#[test]
fn capture_requires_the_complete_extra_reservation() {
    let plan = plan(Method::CoxMatthews);
    let state =
        SpectralState::from_rest(plan, TickClock::from_rest(-12, 100).unwrap(), Epoch(7)).unwrap();
    let bytes = PhysicalImage::reservation(plan).unwrap();
    assert_eq!(bytes, 3 * 48 * 16 + std::mem::size_of::<PhysicalImage>());
    assert_eq!(
        PhysicalImage::capture(&state, bytes - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    let image = PhysicalImage::capture(&state, bytes).unwrap();
    compare(&state, image.state());
    compare(&state, &image.into_state());
}

#[test]
fn fresh_method_scratch_reproduces_next_accepted_and_rejected_attempts() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        for (budget, accepted) in [(1.0, true), (1e-40, false)] {
            round_trip(method, budget, accepted);
        }
    }
}
fn round_trip(method: Method, budget: f64, accepted: bool) {
    let plan = plan(method);
    let zero = TickClock::from_rest(-12, 100).unwrap();
    let mut state = SpectralState::from_rest(plan, zero, Epoch(7)).unwrap();
    let mut candidate = CandidateState::new(plan, zero, Epoch(7)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
    let first = workspace
        .try_advance(&state, &mut candidate, 4, tolerance(1.0), &mut source())
        .unwrap();
    commit_candidate(plan, &mut state, &mut candidate, first.accepted.unwrap()).unwrap();
    assert_eq!(state.accepted_steps(), 1);
    let image = PhysicalImage::capture(&state, PhysicalImage::reservation(plan).unwrap()).unwrap();
    compare(&state, image.state());
    let mut restored = image.into_state();
    let mut new_candidate = CandidateState::new(plan, zero, Epoch(7)).unwrap();
    let mut new_workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
    let original = workspace
        .try_advance(&state, &mut candidate, 4, tolerance(budget), &mut source())
        .unwrap();
    let resumed = new_workspace
        .try_advance(
            &restored,
            &mut new_candidate,
            4,
            tolerance(budget),
            &mut source(),
        )
        .unwrap();
    assert_eq!(original.ticks, resumed.ticks);
    assert_eq!(original.rhs_calls, resumed.rhs_calls);
    assert_eq!(
        original.indicators.errors.map(f64::to_bits),
        resumed.indicators.errors.map(f64::to_bits)
    );
    assert_eq!(
        original.indicators.ratios.map(f64::to_bits),
        resumed.indicators.ratios.map(f64::to_bits)
    );
    assert_eq!(original.accepted.is_some(), accepted);
    assert_eq!(resumed.accepted.is_some(), accepted);
    if let (Some(a), Some(b)) = (original.accepted, resumed.accepted) {
        commit_candidate(plan, &mut state, &mut candidate, a).unwrap();
        commit_candidate(plan, &mut restored, &mut new_candidate, b).unwrap();
    }
    compare(&state, &restored);
    assert_eq!(restored.accepted_steps(), 1 + u128::from(accepted));
}
