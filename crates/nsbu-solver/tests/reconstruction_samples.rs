//! A declared probe table must really refine, stay inside the window, and match exact tested times.
use nsbu_solver::{
    domain::TickClock,
    verification::{
        reconstruction::{OffStageProbe, ProbeRefinement, ReconstructionSamples},
        times::TestedTimes,
        VerificationError,
    },
};

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-10, 128, elapsed, 128 - elapsed).unwrap()
}
fn levels(time: u128) -> [OffStageProbe; 3] {
    [32, 16, 8].map(|step| OffStageProbe::new([0, step, 2 * step].map(clock), clock(time)).unwrap())
}

#[test]
fn three_nested_histories_are_required_at_each_unchanged_probe() {
    let [a, b, c] = levels(3);
    for wrong in [[a, a, c], [a, b, b], [a, b, levels(5)[2]]] {
        assert_eq!(
            ProbeRefinement::new(wrong).unwrap_err(),
            VerificationError::InvalidProbe
        );
    }
    let probes = [3, 5].map(|time| ProbeRefinement::new(levels(time)).unwrap());
    let clocks = [0, 3, 5, 16, 64].map(clock);
    let times = TestedTimes::new(&clocks, 5).unwrap();
    let samples = ReconstructionSamples::new(&probes, times, 2).unwrap();
    assert_eq!(samples.as_slice().len(), 2);
    assert_eq!(samples.as_slice()[1].levels()[2].time(), clock(5));
    assert!(samples.matches_times(times));
    let extra = [0, 3, 5, 8, 16, 64].map(clock);
    assert!(!samples.matches_times(TestedTimes::new(&extra, 6).unwrap()));
}

#[test]
fn missing_unsorted_and_duplicate_off_stage_samples_never_pass_as_a_table() {
    let probes = [3, 5].map(|time| ProbeRefinement::new(levels(time)).unwrap());
    let clocks = [0, 3, 5, 16, 64].map(clock);
    let times = TestedTimes::new(&clocks, 5).unwrap();
    assert_eq!(
        ReconstructionSamples::new(&[], times, 0).unwrap_err(),
        VerificationError::MissingReconstruction
    );
    assert_eq!(
        ReconstructionSamples::new(&probes, times, 1).unwrap_err(),
        VerificationError::CapacityExceeded
    );
    for wrong in [[probes[1], probes[0]], [probes[0], probes[0]]] {
        assert_eq!(
            ReconstructionSamples::new(&wrong, times, 2).unwrap_err(),
            VerificationError::InvalidTimes
        );
    }
}

#[test]
fn untested_probe_times_future_history_and_changed_quantum_are_refused() {
    let probes = [ProbeRefinement::new(levels(3)).unwrap()];
    let absent = [0, 5, 16, 64].map(clock);
    assert_eq!(
        ReconstructionSamples::new(&probes, TestedTimes::new(&absent, 4).unwrap(), 1).unwrap_err(),
        VerificationError::InvalidTimes
    );
    let shortened = [0, 3, 16, 32].map(clock);
    assert_eq!(
        ReconstructionSamples::new(&probes, TestedTimes::new(&shortened, 4).unwrap(), 1)
            .unwrap_err(),
        VerificationError::InvalidProbe
    );
    let changed =
        [0, 3, 16, 64].map(|elapsed| TickClock::restore(-11, 128, elapsed, 128 - elapsed).unwrap());
    assert_eq!(
        ReconstructionSamples::new(&probes, TestedTimes::new(&changed, 4).unwrap(), 1).unwrap_err(),
        VerificationError::InvalidTimes
    );
}
