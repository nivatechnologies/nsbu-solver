//! Exact last-step table identity, checkpoint-cold reconstruction and resource ABI controls.
use nsbu_solver::{
    checkpoint::physical::{PhysicalArchive, UnverifiedPhysical},
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        kernel::{RhsBounds, RightHandSide},
        method::Method,
        transaction::CandidateState,
    },
    Complex64, SolverError,
};

struct NonzeroMean;
impl RightHandSide for NonzeroMean {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for (axis, values) in output.into_iter().enumerate() {
            values.fill(Complex64::new(0.0, 0.0));
            values[0] = Complex64::new((axis + 1) as f64 + time.elapsed() as f64 / 32.0, 0.0);
        }
        Ok(())
    }
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
        2 * 1024 * 1024,
        Epoch(9),
    )
    .unwrap()
}

fn attempt(
    work: &mut AttemptWorkspace,
    state: &SpectralState,
    candidate: &mut CandidateState,
) -> Vec<u64> {
    let result = work
        .try_advance(
            state,
            candidate,
            4,
            Tolerances {
                absolute: [1e6; 2],
                relative: [0.0; 2],
            },
            &mut NonzeroMean,
        )
        .unwrap();
    let proposal = candidate
        .proposal(state, &result.accepted.unwrap())
        .unwrap();
    let words: Vec<_> = (0..3)
        .flat_map(|axis| proposal.component(axis).unwrap())
        .flat_map(|value| [value.re.to_bits(), value.im.to_bits()])
        .collect();
    assert!(words.iter().any(|word| *word != 0));
    words
}

#[test]
fn same_dt_hit_matches_nonzero_forced_checkpoint_cold_rebuild() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let plan = plan(method);
        let clock = TickClock::from_rest(-5, 64).unwrap();
        let state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
        let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
        let mut warm = AttemptWorkspace::new_with_method(plan, method).unwrap();
        let cold_words = attempt(&mut warm, &state, &mut candidate);
        let hit_words = attempt(&mut warm, &state, &mut candidate);
        assert_eq!(hit_words, cold_words);

        let mut bytes = vec![0; PhysicalArchive::encoded_len(&state).unwrap()];
        PhysicalArchive::write(&state, &mut bytes).unwrap();
        let restored = PhysicalArchive::read(
            &bytes,
            plan,
            bytes.len(),
            UnverifiedPhysical::reservation(plan).unwrap(),
        )
        .unwrap()
        .into_state();
        let mut restored_candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
        let mut restored_cold = AttemptWorkspace::new_with_method(plan, method).unwrap();
        assert_eq!(
            attempt(&mut restored_cold, &restored, &mut restored_candidate),
            hit_words
        );
    }
}

#[test]
fn unconditional_default_workspace_resource_abi_is_frozen() {
    assert_eq!(std::mem::size_of::<AttemptWorkspace>(), 800);
    let domain = Domain::new([4; 3], [1.0, 2.0, 3.0], 0.5).unwrap();
    let reservation = AttemptWorkspace::reservation(domain).unwrap();
    assert_eq!(reservation, 22_664);
    assert_eq!(
        AttemptWorkspace::reservation_with_method(domain, Method::HochbruckOstermann).unwrap(),
        45_008
    );
    let short = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: reservation - 1,
            overhead: 4096,
        },
        2 * 1024 * 1024,
        Epoch(9),
    )
    .unwrap();
    assert!(matches!(
        AttemptWorkspace::new(short),
        Err(SolverError::ResourceLimit)
    ));
}
