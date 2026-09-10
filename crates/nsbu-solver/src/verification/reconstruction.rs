//! Exact off-stage geometry; accepted-state provenance remains a separate experiment obligation.
use super::{times::TestedTimes, VerificationError};
use crate::{diagnostics::hermite::coordinates, domain::TickClock};

/// One probe strictly between the full/two-half CM/HO stage clocks of its history.
/// This records sampled geometry only, never a bound over the reconstruction interval.
#[derive(Debug, Clone, Copy)]
pub struct OffStageProbe {
    nodes: [TickClock; 3],
    probe: TickClock,
}

/// Three nested histories at one exact off-stage comparison time.
#[derive(Debug, Clone, Copy)]
pub struct ProbeRefinement {
    levels: [OffStageProbe; 3],
}
impl ProbeRefinement {
    /// Require two strict reconstruction refinements, not shifted or nodal samples.
    pub fn new(levels: [OffStageProbe; 3]) -> Result<Self, VerificationError> {
        if !levels[1].refines(levels[0]) || !levels[2].refines(levels[1]) {
            return Err(VerificationError::InvalidProbe);
        }
        Ok(Self { levels })
    }
    /// Exact geometry on all three levels.
    pub fn levels(self) -> [OffStageProbe; 3] {
        self.levels
    }
}

/// Nonempty bounded off-stage evidence on the declared fine tested-time set.
/// Histories may cover different subintervals; this is not slab-wide coverage or an enclosure.
#[derive(Debug, Clone, Copy)]
pub struct ReconstructionSamples<'a> {
    refinements: &'a [ProbeRefinement],
    times: TestedTimes<'a>,
}
impl<'a> ReconstructionSamples<'a> {
    /// Preserve a strictly ordered probe set, and refuse history beyond the tested window.
    /// Each probe must occur in the exact fine manifest; caller storage remains borrowed.
    pub fn new(
        refinements: &'a [ProbeRefinement],
        times: TestedTimes<'a>,
        maximum_probes: usize,
    ) -> Result<Self, VerificationError> {
        if refinements.len() > maximum_probes {
            return Err(VerificationError::CapacityExceeded);
        }
        if refinements.is_empty() {
            return Err(VerificationError::MissingReconstruction);
        }
        let clocks = times.as_slice();
        let endpoint = clocks[clocks.len() - 1].elapsed();
        let mut previous = 0;
        for refinement in refinements {
            let probe = refinement.levels[0];
            if !clocks.contains(&probe.time()) || probe.time().elapsed() <= previous {
                return Err(VerificationError::InvalidTimes);
            }
            if probe.nodes()[2].elapsed() > endpoint {
                return Err(VerificationError::InvalidProbe);
            }
            previous = probe.time().elapsed();
        }
        Ok(Self { refinements, times })
    }
    /// Exact sampled reconstruction evidence, retained without relabeling it as interval bounds.
    pub fn as_slice(self) -> &'a [ProbeRefinement] {
        self.refinements
    }
    /// Bind a review to the exact manifest against which this evidence was admitted.
    pub fn matches_times(self, times: TestedTimes<'_>) -> bool {
        self.times.as_slice() == times.as_slice()
    }
}
impl OffStageProbe {
    /// Use the exact same time/arithmetic admission as the implemented quintic reconstruction.
    /// History spacing is one accepted macro interval, whose stage set lies on quarter ticks.
    pub fn new(nodes: [TickClock; 3], probe: TickClock) -> Result<Self, VerificationError> {
        coordinates(nodes, probe).map_err(VerificationError::ReconstructionGeometry)?;
        let step = nodes[1].elapsed() - nodes[0].elapsed();
        if !step.is_multiple_of(4)
            || (probe.elapsed() - nodes[0].elapsed()).is_multiple_of(step / 4)
        {
            return Err(VerificationError::InvalidProbe);
        }
        Ok(Self { nodes, probe })
    }

    /// The exact comparison time, retained independently from rounded physical time.
    pub fn time(self) -> TickClock {
        self.probe
    }

    /// Clocks of the accepted-history values required by the experiment.
    pub fn nodes(self) -> [TickClock; 3] {
        self.nodes
    }

    /// Require a smaller, nested history interval and an unchanged exact probe time.
    /// Shrinking intervals do not make maxima into common-interval suprema.
    pub fn refines(self, coarse: Self) -> bool {
        self.probe == coarse.probe
            && self.nodes[0].elapsed() >= coarse.nodes[0].elapsed()
            && self.nodes[2].elapsed() <= coarse.nodes[2].elapsed()
            && self.nodes[1].elapsed() - self.nodes[0].elapsed()
                < coarse.nodes[1].elapsed() - coarse.nodes[0].elapsed()
    }
}
