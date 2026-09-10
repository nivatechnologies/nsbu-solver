//! A transferred coarse history retains its missing high mode against direct fine evolution.
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace,
    indicator::Tolerances,
    kernel::{RhsBounds, RightHandSide},
    method::Method,
    transaction::{commit_candidate, CandidateState},
};
use nsbu_solver::{lineage::ProlongedState, Complex64, SolverError};

struct Shear {
    position: Option<usize>,
}
impl RightHandSide for Shear {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }
    fn evaluate(
        &mut self,
        _: [&[Complex64]; 3],
        _: TickClock,
        mut output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for field in output.iter_mut() {
            field.fill(Complex64::new(0.0, 0.0));
        }
        if let Some(index) = self.position {
            output[0][index] = Complex64::new(0.5, 0.0);
        }
        Ok(())
    }
}
fn plan(dimensions: [usize; 3], lengths: [f64; 3], viscosity: f64, method: Method) -> ResourcePlan {
    let domain = Domain::new(dimensions, lengths, viscosity).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
            overhead: 4096,
        },
        1 << 24,
        Epoch(9),
    )
    .unwrap()
}
fn standard(n: usize, method: Method) -> ResourcePlan {
    plan([n; 3], [std::f64::consts::TAU; 3], 0.125, method)
}
fn initial(plan: ResourcePlan, epoch: Epoch) -> SpectralState {
    SpectralState::from_rest(plan, TickClock::from_rest(-12, 1000).unwrap(), epoch).unwrap()
}
fn evolve(state: &mut SpectralState, method: Method, mode: isize, steps: usize) {
    let mut source = Shear {
        position: state
            .plan()
            .domain()
            .layout()
            .locate([0, 0, mode])
            .ok()
            .map(|p| p.0),
    };
    let mut scratch = AttemptWorkspace::new_with_method(state.plan(), method).unwrap();
    // Fresh scratch starts empty; the committed physical clock and coefficients are untouched.
    let scratch_clock =
        TickClock::from_rest(state.clock().exponent(), state.clock().target()).unwrap();
    let mut candidate = CandidateState::new(state.plan(), scratch_clock, state.epoch()).unwrap();
    for _ in 0..steps {
        let result = scratch
            .try_advance(
                state,
                &mut candidate,
                4,
                Tolerances {
                    absolute: [1e-10; 2],
                    relative: [1e-10; 2],
                },
                &mut source,
            )
            .unwrap();
        commit_candidate(
            state.plan(),
            state,
            &mut candidate,
            result.accepted.unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn missing_earlier_high_mode_survives_transfer_and_later_fine_integration() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut coarse = initial(standard(4, method), Epoch(0));
        let mut direct = initial(standard(8, method), Epoch(0));
        evolve(&mut coarse, method, 3, 4);
        evolve(&mut direct, method, 3, 4);
        let transfer = ProlongedState::from_state(&coarse, direct.plan(), usize::MAX).unwrap();
        assert_eq!(transfer.source_plan(), coarse.plan());
        assert_eq!(transfer.state().clock(), coarse.clock());
        assert_eq!(transfer.state().accepted_steps(), 4);
        assert_eq!(transfer.state().epoch(), Epoch(5));
        let mut continued = transfer.into_state();
        evolve(&mut continued, method, 3, 4);
        evolve(&mut direct, method, 3, 4);
        assert_eq!(continued.clock(), direct.clock());
        assert_eq!(continued.accepted_steps(), 8);
        let layout = direct.plan().domain().layout();
        let high = layout.locate([0, 0, 3]).unwrap().0;
        let direct_value = direct.component(0).unwrap()[high].re;
        let transferred_value = continued.component(0).unwrap()[high].re;
        let rate: f64 = 0.125 * 9.0;
        let half_time = 16.0 / 4096.0;
        let expected = -(-rate * half_time).exp_m1() / (2.0 * rate);
        assert!((direct_value + (-2.0 * rate * half_time).exp_m1() / (2.0 * rate)).abs() < 1e-16);
        assert!((transferred_value - expected).abs() < 1e-16);
        assert!(
            (direct_value - transferred_value - expected * (-rate * half_time).exp()).abs() < 1e-16
        );
        assert!(direct_value - transferred_value > 0.001);
        let common = layout.locate([0, 0, 1]).unwrap().0;
        assert_eq!(
            direct.component(0).unwrap()[common],
            continued.component(0).unwrap()[common]
        );
    }
}

#[test]
fn retained_modes_keep_their_bits_and_new_modes_are_zero_without_rescaling() {
    let method = Method::CoxMatthews;
    let mut source = initial(standard(4, method), Epoch(11));
    evolve(&mut source, method, 1, 2);
    let target = standard(8, method);
    let directional = plan([4, 8, 4], source.plan().domain().lengths(), 0.125, method);
    let directional_image = ProlongedState::from_state(&source, directional, usize::MAX).unwrap();
    assert_eq!(directional_image.state().plan(), directional);
    let cap = ProlongedState::reservation(target).unwrap();
    assert_eq!(
        cap,
        3 * target.domain().layout().half_len() * 16 + std::mem::size_of::<ProlongedState>()
    );
    assert_eq!(
        ProlongedState::from_state(&source, target, cap - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    let transferred = ProlongedState::from_state(&source, target, cap).unwrap();
    let old = source.plan().domain().layout().locate([0, 0, 1]).unwrap().0;
    let new = target.domain().layout().locate([0, 0, 1]).unwrap().0;
    assert_eq!(
        transferred.state().component(0).unwrap()[new].re.to_bits(),
        source.component(0).unwrap()[old].re.to_bits()
    );
    assert_eq!(
        transferred.state().component(0).unwrap()[new].im.to_bits(),
        source.component(0).unwrap()[old].im.to_bits()
    );
    assert_eq!(transferred.state().epoch(), Epoch(14));
    let high = target.domain().layout().locate([0, 0, 3]).unwrap().0;
    assert_eq!(
        transferred.state().component(0).unwrap()[high],
        Complex64::new(0.0, 0.0)
    );
    assert_eq!(source.accepted_steps(), 2);
}

#[test]
fn changed_problem_coarsened_axes_and_exhausted_epoch_are_refused() {
    let method = Method::HochbruckOstermann;
    let source = initial(standard(8, method), Epoch(0));
    let lengths = [std::f64::consts::TAU; 3];
    let targets = [
        standard(8, method),
        standard(4, method),
        plan([12, 4, 12], lengths, 0.125, method),
        plan([12; 3], [1.0; 3], 0.125, method),
        plan([12; 3], lengths, 0.25, method),
    ];
    for target in targets {
        assert_eq!(
            ProlongedState::from_state(&source, target, usize::MAX).unwrap_err(),
            SolverError::InvalidDomain
        );
    }
    let exhausted = initial(standard(4, method), Epoch(u128::MAX));
    assert_eq!(
        ProlongedState::from_state(&exhausted, standard(8, method), usize::MAX).unwrap_err(),
        SolverError::EpochExhausted
    );
    assert_eq!(source.clock().elapsed(), 0);
}
