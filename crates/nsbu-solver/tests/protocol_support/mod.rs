//! Small owned fixture; protocol construction borrows its preexisting arrays.
use nsbu_solver::{
    domain::TickClock,
    verification::{
        budget::Budget,
        policy::{ObservablePolicy, Policies},
        protocol::ProtocolInputs,
        reconstruction::{OffStageProbe, ProbeRefinement, ReconstructionSamples},
        refinement::{Requirement, Rule},
        times::TestedTimes,
    },
};

pub fn clock(tick: u128) -> TickClock {
    TickClock::restore(-16, 512, tick, 512 - tick).unwrap()
}
pub fn probes(finest: [u128; 3], probe: u128) -> ProbeRefinement {
    ProbeRefinement::new(
        [[0, 64, 128], [64, 96, 128], finest]
            .map(|nodes| OffStageProbe::new(nodes.map(clock), clock(probe)).unwrap()),
    )
    .unwrap()
}
pub struct Fixture {
    pub policies: Vec<ObservablePolicy>,
    pub times: [Vec<TickClock>; 3],
    pub probes: Vec<ProbeRefinement>,
}
impl Fixture {
    pub fn new() -> Self {
        let rule = Rule::new(0.01, 0.5, 0.1, Requirement::Refinement).unwrap();
        Self {
            policies: vec![ObservablePolicy {
                key: 41,
                budget: Budget::new(1.0, [rule; 11]).unwrap(),
            }],
            times: [vec![0, 128], vec![0, 64, 128], vec![0, 64, 127, 128]]
                .map(|times| times.into_iter().map(clock).collect()),
            probes: vec![probes([96, 112, 128], 127)],
        }
    }
    pub fn inputs(&self) -> ProtocolInputs<'_> {
        let times = self
            .times
            .each_ref()
            .map(|times| TestedTimes::new(times, times.len()).unwrap());
        ProtocolInputs {
            problem: [1; 32],
            semantics: [2; 32],
            policies: Policies::new(&self.policies, self.policies.len() * self.policies.len())
                .unwrap(),
            time_sets: times,
            reconstruction: ReconstructionSamples::new(&self.probes, times[2], self.probes.len())
                .unwrap(),
        }
    }
}
