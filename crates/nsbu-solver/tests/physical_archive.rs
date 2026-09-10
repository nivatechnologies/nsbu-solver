//! Bounded physical archives preserve live Fourier bits but never certify a restart.
mod source_contract;
use nsbu_solver::{
    checkpoint::{
        physical::{PhysicalArchive, UnverifiedPhysical},
        CheckpointError,
    },
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        method::Method,
        transaction::{commit_candidate, CandidateState},
    },
    Complex64,
};

fn plan(method: Method) -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0, 2.0, 3.0], 0.5).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
            overhead: 64,
        },
        1 << 20,
        Epoch(42),
    )
    .unwrap()
}

fn source() -> source_contract::Source {
    source_contract::Source::new(
        usize::MAX,
        std::array::from_fn(|axis| {
            [
                Complex64::new((axis + 1) as f64, 0.0),
                Complex64::new(1.0, (axis + 2) as f64),
            ]
        }),
    )
}

fn tolerances(value: f64) -> Tolerances {
    Tolerances {
        absolute: [value; 2],
        relative: [0.0; 2],
    }
}

fn encoded(state: &SpectralState) -> Vec<u8> {
    let size = PhysicalArchive::encoded_len(state).unwrap();
    let mut bytes = vec![0x55; size + 1];
    assert_eq!(
        PhysicalArchive::write(state, &mut bytes[..size - 1]),
        Err(CheckpointError::ResourceLimit)
    );
    assert!(bytes.iter().all(|byte| *byte == 0x55));
    assert_eq!(PhysicalArchive::write(state, &mut bytes), Ok(size));
    assert_eq!(bytes[size], 0x55);
    bytes.truncate(size);
    bytes
}

fn restored(bytes: &[u8], plan: ResourcePlan) -> UnverifiedPhysical {
    PhysicalArchive::read(
        bytes,
        plan,
        bytes.len(),
        UnverifiedPhysical::reservation(plan).unwrap(),
    )
    .unwrap()
}

fn equal(left: &SpectralState, right: &SpectralState) {
    assert_eq!(left.plan(), right.plan());
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.epoch(), right.epoch());
    assert_eq!(left.accepted_steps(), right.accepted_steps());
    for axis in 0..3 {
        for (a, b) in left
            .component(axis)
            .unwrap()
            .iter()
            .zip(right.component(axis).unwrap())
        {
            assert_eq!(a.re.to_bits(), b.re.to_bits());
            assert_eq!(a.im.to_bits(), b.im.to_bits());
        }
    }
}

fn refusal(bytes: &[u8], plan: ResourcePlan) -> CheckpointError {
    PhysicalArchive::read(bytes, plan, bytes.len(), usize::MAX).unwrap_err()
}

#[test]
fn exact_payload_requires_matching_plan_caps_and_valid_fourier_input() {
    let plan = plan(Method::CoxMatthews);
    let state =
        SpectralState::from_rest(plan, TickClock::from_rest(-12, 100).unwrap(), Epoch(7)).unwrap();
    let bytes = encoded(&state);
    let image = restored(&bytes, plan);
    equal(&state, image.state());
    let changed_plan = ResourcePlan::new(
        Domain::new([4; 3], [1.0, 2.0, 4.0], 0.5).unwrap(),
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 1,
            overhead: 64,
        },
        1 << 20,
        Epoch(42),
    )
    .unwrap();
    assert_eq!(
        PhysicalArchive::read(&bytes, changed_plan, bytes.len(), usize::MAX).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
    assert_eq!(
        PhysicalArchive::read(&bytes, plan, bytes.len() - 1, usize::MAX).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    assert_eq!(
        PhysicalArchive::read(&bytes, plan, bytes.len(), 0).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    let mut nonfinite = bytes.clone();
    let first_value = 350;
    nonfinite[first_value..first_value + 8].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
    assert_eq!(
        PhysicalArchive::read(&nonfinite, plan, nonfinite.len(), usize::MAX).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
    let mut nyquist = bytes.clone();
    let nyquist_value = first_value + 24 * 16;
    nyquist[nyquist_value..nyquist_value + 8].copy_from_slice(&1.0_f64.to_bits().to_le_bytes());
    assert_eq!(
        PhysicalArchive::read(&nyquist, plan, nyquist.len(), usize::MAX).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
    let mut nonhermitian = bytes.clone();
    let k_zero_mode = first_value + 12 * 16;
    nonhermitian[k_zero_mode..k_zero_mode + 8].copy_from_slice(&1.0_f64.to_bits().to_le_bytes());
    assert_eq!(
        PhysicalArchive::read(&nonhermitian, plan, nonhermitian.len(), usize::MAX).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
    let mut bad_count = bytes;
    bad_count[334..350].copy_from_slice(&u128::MAX.to_le_bytes());
    assert_eq!(
        PhysicalArchive::read(&bad_count, plan, bad_count.len(), usize::MAX).unwrap_err(),
        CheckpointError::ResourceLimit
    );
}

#[test]
fn headers_preflight_each_identity_clock_count_and_frame_before_field_allocation() {
    let plan = plan(Method::CoxMatthews);
    let state =
        SpectralState::from_rest(plan, TickClock::from_rest(-12, 100).unwrap(), Epoch(7)).unwrap();
    let bytes = encoded(&state);
    for offset in [
        10, 26, 42, 58, 66, 74, 82, 90, 106, 122, 138, 154, 170, 186, 202, 218, 234,
    ] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert_eq!(refusal(&changed, plan), CheckpointError::InvalidEncoding);
    }
    for (offset, value) in [(254, 99_u128), (270, 1_u128), (286, 0_u128), (334, 0_u128)] {
        let mut changed = bytes.clone();
        changed[offset..offset + 16].copy_from_slice(&value.to_le_bytes());
        assert_eq!(refusal(&changed, plan), CheckpointError::InvalidEncoding);
    }
    let mut oversized = bytes.clone();
    oversized[334..350].copy_from_slice(&u128::MAX.to_le_bytes());
    assert_eq!(refusal(&oversized, plan), CheckpointError::ResourceLimit);
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(refusal(&trailing, plan), CheckpointError::InvalidEncoding);
    for length in [0, 8, 10, 58, 90, 106, 250, 302, 334, 349] {
        assert!(PhysicalArchive::read(&bytes[..length], plan, bytes.len(), usize::MAX).is_err());
    }
}

#[test]
fn both_integrators_resume_the_same_next_attempt_from_unverified_bits() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let plan = plan(method);
        let clock = TickClock::from_rest(-12, 100).unwrap();
        let mut state = SpectralState::from_rest(plan, clock, Epoch(7)).unwrap();
        let mut candidate = CandidateState::new(plan, clock, Epoch(7)).unwrap();
        let mut workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
        let accepted = workspace
            .try_advance(&state, &mut candidate, 4, tolerances(1.0), &mut source())
            .unwrap()
            .accepted
            .unwrap();
        commit_candidate(plan, &mut state, &mut candidate, accepted).unwrap();
        let mut resumed = restored(&encoded(&state), plan).into_state();
        let mut resumed_candidate = CandidateState::new(plan, clock, Epoch(7)).unwrap();
        let mut resumed_workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
        for budget in [1.0, 1e-40] {
            let original = workspace
                .try_advance(&state, &mut candidate, 4, tolerances(budget), &mut source())
                .unwrap();
            let replayed = resumed_workspace
                .try_advance(
                    &resumed,
                    &mut resumed_candidate,
                    4,
                    tolerances(budget),
                    &mut source(),
                )
                .unwrap();
            assert_eq!(original.ticks, replayed.ticks);
            assert_eq!(original.rhs_calls, replayed.rhs_calls);
            assert_eq!(
                original.indicators.errors.map(f64::to_bits),
                replayed.indicators.errors.map(f64::to_bits)
            );
            assert_eq!(original.accepted.is_some(), replayed.accepted.is_some());
            if let (Some(a), Some(b)) = (original.accepted, replayed.accepted) {
                commit_candidate(plan, &mut state, &mut candidate, a).unwrap();
                commit_candidate(plan, &mut resumed, &mut resumed_candidate, b).unwrap();
            }
            equal(&state, &resumed);
        }
    }
}

#[test]
fn conjugacy_roundoff_is_preserved_and_nyquist_remains_exact() {
    let plan = plan(Method::CoxMatthews);
    let clock = TickClock::from_rest(-12, 4096).unwrap();
    let state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut bytes = encoded(&state);
    // An imaginary mean has twice its magnitude as the conjugacy discrepancy.
    // The runtime admits this roundoff; a checkpoint preserves rather than repairs it.
    bytes[358..366].copy_from_slice(&1e-14_f64.to_le_bytes());
    let imported = restored(&bytes, plan);
    assert_eq!(encoded(imported.state()), bytes);
    bytes[358..366].copy_from_slice(&1e-5_f64.to_le_bytes());
    assert!(PhysicalArchive::read(
        &bytes,
        plan,
        bytes.len(),
        UnverifiedPhysical::reservation(plan).unwrap()
    )
    .is_err());
    let mut bytes = encoded(&state);
    let nyquist = plan.domain().layout().index([2, 0, 0]).unwrap();
    let offset = 350 + 16 * nyquist;
    bytes[offset..offset + 8].copy_from_slice(&1e-30_f64.to_le_bytes());
    assert!(PhysicalArchive::read(
        &bytes,
        plan,
        bytes.len(),
        UnverifiedPhysical::reservation(plan).unwrap()
    )
    .is_err());
}
