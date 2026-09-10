//! One canonical byte traversal feeds either SHA-256 or admitted caller-owned storage.
use super::{ProtocolInputs, MAGIC};
use crate::{
    domain::TickClock,
    verification::{
        budget::CHANNELS, policy::Policies, reconstruction::ReconstructionSamples,
        refinement::Requirement, times::TestedTimes,
    },
};
use sha2::{Digest, Sha256};

pub(super) fn identity(inputs: ProtocolInputs<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    encode(inputs, |bytes| hash.update(bytes));
    hash.finalize().into()
}
// The caller checks the complete byte reservation before entering this infallible traversal.
pub(super) fn write(inputs: ProtocolInputs<'_>, output: &mut [u8]) -> usize {
    let mut position = 0;
    encode(inputs, |bytes| {
        let end = position + bytes.len();
        output[position..end].copy_from_slice(bytes);
        position = end;
    });
    position
}
fn encode(inputs: ProtocolInputs<'_>, mut put: impl FnMut(&[u8])) {
    put(MAGIC);
    put(&inputs.problem);
    put(&inputs.semantics);
    policies(&mut put, inputs.policies);
    times(&mut put, inputs.time_sets);
    reconstruction(&mut put, inputs.reconstruction);
}
fn policies(put: &mut impl FnMut(&[u8]), policies: Policies<'_>) {
    put(&(policies.as_slice().len() as u128).to_le_bytes());
    for entry in policies.as_slice() {
        put(&entry.key.to_le_bytes());
        put(&entry.budget.total().to_bits().to_le_bytes());
        put(&entry.budget.allocated().to_bits().to_le_bytes());
        for channel in CHANNELS {
            let rule = entry.budget.rule(channel);
            for value in [
                rule.budget(),
                rule.maximum_reduction_ratio(),
                rule.subordinate_floor_budget(),
            ] {
                put(&value.to_bits().to_le_bytes());
            }
            put(&[match rule.requirement() {
                Requirement::Sensitivity => 0,
                Requirement::Refinement => 1,
            }]);
        }
    }
}
fn times(put: &mut impl FnMut(&[u8]), sets: [TestedTimes<'_>; 3]) {
    for times in sets {
        put(&(times.as_slice().len() as u128).to_le_bytes());
        for time in times.as_slice() {
            clock(put, *time);
        }
    }
}
fn reconstruction(put: &mut impl FnMut(&[u8]), samples: ReconstructionSamples<'_>) {
    put(&(samples.as_slice().len() as u128).to_le_bytes());
    for sample in samples.as_slice() {
        for level in sample.levels() {
            for node in level.nodes() {
                clock(put, node);
            }
            clock(put, level.time());
        }
    }
}
fn clock(put: &mut impl FnMut(&[u8]), clock: TickClock) {
    put(&clock.exponent().to_le_bytes());
    put(&clock.target().to_le_bytes());
    put(&clock.elapsed().to_le_bytes());
    put(&clock.remaining().to_le_bytes());
}
