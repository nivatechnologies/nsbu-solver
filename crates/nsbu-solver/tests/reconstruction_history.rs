//! Preallocated accepted reconstruction nodes retain exact fields across staged publication.
mod source_contract;
use nsbu_solver::{
    checkpoint::physical::{PhysicalArchive, UnverifiedPhysical},
    diagnostics::reconstruction_history::{
        ReconstructionHistory, ReconstructionNode, StagedSegment,
    },
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        transaction::{commit_candidate, CandidateState},
    },
    lineage::PhysicalImage,
    Complex64, SolverError,
};

const CLOCK_TARGET: usize = 254;
const CLOCK_ELAPSED: usize = 270;
const CLOCK_REMAINING: usize = 286;
const STATE_EPOCH: usize = 302;
const ACCEPTED_STEPS: usize = 318;

fn plan() -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0, 2.0, 3.0], 0.5).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 64,
        },
        1 << 20,
        Epoch(4),
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

fn advance(
    plan: ResourcePlan,
    state: &mut SpectralState,
    candidate: &mut CandidateState,
    workspace: &mut AttemptWorkspace,
) {
    let accepted = workspace
        .try_advance(
            state,
            candidate,
            4,
            Tolerances {
                absolute: [1.0; 2],
                relative: [0.0; 2],
            },
            &mut source(),
        )
        .unwrap()
        .accepted
        .unwrap();
    commit_candidate(plan, state, candidate, accepted).unwrap();
}

fn nodes<'a>(
    start: &'a SpectralState,
    middle: &'a SpectralState,
    end: &'a SpectralState,
    derivative: &'a [Vec<Complex64>; 3],
) -> [ReconstructionNode<'a>; 3] {
    let derivative = std::array::from_fn(|axis| derivative[axis].as_slice());
    [
        ReconstructionNode::new(start, derivative),
        ReconstructionNode::new(middle, derivative),
        ReconstructionNode::new(end, derivative),
    ]
}

fn archived(state: &SpectralState, changes: &[(usize, u128)]) -> SpectralState {
    let size = PhysicalArchive::encoded_len(state).unwrap();
    let mut bytes = vec![0; size];
    PhysicalArchive::write(state, &mut bytes).unwrap();
    for &(offset, value) in changes {
        bytes[offset..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
    PhysicalArchive::read(
        &bytes,
        state.plan(),
        bytes.len(),
        UnverifiedPhysical::reservation(state.plan()).unwrap(),
    )
    .unwrap()
    .into_state()
}

fn derivatives(plan: ResourcePlan) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); plan.domain().layout().half_len()])
}

fn refusal(result: Result<StagedSegment, SolverError>, expected: SolverError) {
    assert_eq!(result.unwrap_err(), expected);
}

fn accepted_states() -> (ResourcePlan, PhysicalImage, PhysicalImage, SpectralState) {
    let plan = plan();
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(7)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(7)).unwrap();
    let mut workspace = AttemptWorkspace::new(plan).unwrap();
    let start = PhysicalImage::capture(&state, usize::MAX).unwrap();
    advance(plan, &mut state, &mut candidate, &mut workspace);
    let middle = PhysicalImage::capture(&state, usize::MAX).unwrap();
    advance(plan, &mut state, &mut candidate, &mut workspace);
    (plan, start, middle, state)
}

#[test]
fn accepted_sequence_reconstructs_each_hermite_node_and_is_independently_owned() {
    let plan = plan();
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(7)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(7)).unwrap();
    let mut workspace = AttemptWorkspace::new(plan).unwrap();
    let start = PhysicalImage::capture(&state, usize::MAX).unwrap();
    advance(plan, &mut state, &mut candidate, &mut workspace);
    let middle = PhysicalImage::capture(&state, usize::MAX).unwrap();
    advance(plan, &mut state, &mut candidate, &mut workspace);
    let zeros =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); plan.domain().layout().half_len()]);
    let mut history = ReconstructionHistory::new(plan, usize::MAX).unwrap();
    let staged = history
        .stage(nodes(start.state(), middle.state(), &state, &zeros))
        .unwrap();
    assert!(!history.has_segment());
    history.publish_after_commit(&state, &staged).unwrap();
    assert_eq!(
        history.clocks(),
        Some([start.state().clock(), middle.state().clock(), state.clock()])
    );
    let n = plan.domain().layout().half_len();
    let mut value = vec![Complex64::new(0.0, 0.0); n];
    let mut derivative = value.clone();
    history
        .reconstruct(middle.state().clock(), 0, &mut value, &mut derivative)
        .unwrap();
    assert_eq!(value, middle.state().component(0).unwrap());
    assert!(derivative
        .iter()
        .all(|value| *value == Complex64::new(0.0, 0.0)));
    assert_ne!(
        value.as_ptr(),
        middle.state().component(0).unwrap().as_ptr()
    );
}

#[test]
fn preflight_rejection_and_stale_or_uncommitted_tokens_leave_the_ring_unchanged() {
    let plan = plan();
    assert_eq!(
        ReconstructionHistory::new(plan, 0).unwrap_err(),
        SolverError::ResourceLimit
    );
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(7)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(7)).unwrap();
    let mut workspace = AttemptWorkspace::new(plan).unwrap();
    let start = PhysicalImage::capture(&state, usize::MAX).unwrap();
    advance(plan, &mut state, &mut candidate, &mut workspace);
    let middle = PhysicalImage::capture(&state, usize::MAX).unwrap();
    advance(plan, &mut state, &mut candidate, &mut workspace);
    let zeros =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); plan.domain().layout().half_len()]);
    let mut history = ReconstructionHistory::new(plan, usize::MAX).unwrap();
    let staged = history
        .stage(nodes(start.state(), middle.state(), &state, &zeros))
        .unwrap();
    let mut other = ReconstructionHistory::new(plan, usize::MAX).unwrap();
    let other_staged = other
        .stage(nodes(start.state(), middle.state(), &state, &zeros))
        .unwrap();
    assert_eq!(other.discard(&staged), Err(SolverError::StaleAttempt));
    other.discard(&other_staged).unwrap();
    assert_eq!(
        history.publish_after_commit(middle.state(), &staged),
        Err(SolverError::StaleAttempt)
    );
    assert!(!history.has_segment());
    history.discard(&staged).unwrap();
    assert_eq!(history.discard(&staged), Err(SolverError::StaleAttempt));
    assert!(!history.has_segment());
}

#[test]
fn staging_refuses_foreign_plans_and_malformed_derivatives_before_copying() {
    let (plan, start, middle, end) = accepted_states();
    let derivative = derivatives(plan);
    let mut history = ReconstructionHistory::new(plan, usize::MAX).unwrap();
    let foreign_plan = ResourcePlan::new(
        plan.domain(),
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation(plan.domain()).unwrap(),
            overhead: 64,
        },
        1 << 20,
        Epoch(5),
    )
    .unwrap();
    let foreign = SpectralState::from_rest(foreign_plan, start.state().clock(), Epoch(7)).unwrap();
    refusal(
        history.stage(nodes(&foreign, middle.state(), &end, &derivative)),
        SolverError::InvalidPayload,
    );

    let short = vec![Complex64::new(0.0, 0.0); derivative[0].len() - 1];
    let wrong_length = [
        short.as_slice(),
        derivative[1].as_slice(),
        derivative[2].as_slice(),
    ];
    refusal(
        history.stage([
            ReconstructionNode::new(start.state(), wrong_length),
            ReconstructionNode::new(middle.state(), wrong_length),
            ReconstructionNode::new(&end, wrong_length),
        ]),
        SolverError::InvalidPayload,
    );

    let mut bad_spectrum = derivatives(plan);
    let unpaired = plan.domain().layout().index([1, 0, 0]).unwrap();
    bad_spectrum[0][unpaired] = Complex64::new(1.0, 0.0);
    refusal(
        history.stage(nodes(start.state(), middle.state(), &end, &bad_spectrum)),
        SolverError::InvalidSpectrum,
    );
    assert!(!history.has_segment());
}

#[test]
fn staging_refuses_inconsistent_clock_and_accepted_state_metadata() {
    let (plan, start, middle, end) = accepted_states();
    let derivative = derivatives(plan);
    let mut history = ReconstructionHistory::new(plan, usize::MAX).unwrap();
    let other_family = archived(&end, &[(CLOCK_TARGET, 101), (CLOCK_REMAINING, 93)]);
    refusal(
        history.stage(nodes(
            start.state(),
            middle.state(),
            &other_family,
            &derivative,
        )),
        SolverError::InvalidClock,
    );

    let uneven_middle = archived(middle.state(), &[(CLOCK_ELAPSED, 3), (CLOCK_REMAINING, 97)]);
    refusal(
        history.stage(nodes(start.state(), &uneven_middle, &end, &derivative)),
        SolverError::InvalidClock,
    );

    let reversed_start = archived(start.state(), &[(CLOCK_ELAPSED, 8), (CLOCK_REMAINING, 92)]);
    refusal(
        history.stage(nodes(&reversed_start, middle.state(), &end, &derivative)),
        SolverError::InvalidClock,
    );

    let repeated_epoch = archived(middle.state(), &[(STATE_EPOCH, end.epoch().0)]);
    refusal(
        history.stage(nodes(start.state(), &repeated_epoch, &end, &derivative)),
        SolverError::StaleAttempt,
    );

    let repeated_count = archived(middle.state(), &[(ACCEPTED_STEPS, end.accepted_steps())]);
    refusal(
        history.stage(nodes(start.state(), &repeated_count, &end, &derivative)),
        SolverError::StaleAttempt,
    );
    assert!(!history.has_segment());
}

#[test]
fn staged_segment_must_continue_from_the_last_published_endpoint() {
    let (plan, start, middle, end) = accepted_states();
    let derivative = derivatives(plan);
    let mut history = ReconstructionHistory::new(plan, usize::MAX).unwrap();
    let first = history
        .stage(nodes(start.state(), middle.state(), &end, &derivative))
        .unwrap();
    history.publish_after_commit(&end, &first).unwrap();

    let next = archived(
        &end,
        &[
            (CLOCK_ELAPSED, 12),
            (CLOCK_REMAINING, 88),
            (STATE_EPOCH, end.epoch().0 + 1),
            (ACCEPTED_STEPS, end.accepted_steps() + 1),
        ],
    );
    refusal(
        history.stage(nodes(middle.state(), &end, &next, &derivative)),
        SolverError::StaleAttempt,
    );
    assert_eq!(
        history.clocks(),
        Some([start.state().clock(), middle.state().clock(), end.clock()])
    );
}
