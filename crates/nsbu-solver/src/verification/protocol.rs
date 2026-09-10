//! Complete numerical-policy/time fingerprints, separate from physical provenance and acceptance.
use super::{
    policy::Policies,
    reconstruction::ReconstructionSamples,
    review::{required_records, MeasurementReview},
    times::TestedTimes,
    VerificationError,
};
mod encoding;

const MAGIC: &[u8; 16] = b"NSBUPROTOCOL0001";
const CLOCK_BYTES: usize = 52;
const POLICY_BYTES: usize = 4 + 8 + 8 + 11 * (3 * 8 + 1);

/// Borrowed immutable protocol inputs. The benchmark must independently validate its
/// problem identity and complete observable semantics before using this generic layer.
#[derive(Debug, Clone, Copy)]
pub struct ProtocolInputs<'a> {
    /// Exact mathematical problem identifier; zero is forbidden.
    pub problem: [u8; 32],
    /// Versioned observable inventory, units, masks, floors and interpretation identifier.
    pub semantics: [u8; 32],
    /// Ordered, unique observable keys and every numerical channel rule.
    pub policies: Policies<'a>,
    /// Three strictly nested exact tested-time manifests on one from-rest window.
    pub time_sets: [TestedTimes<'a>; 3],
    /// Exact off-stage histories and probe refinements on the finest manifest.
    pub reconstruction: ReconstructionSamples<'a>,
}

/// An immutable fingerprint of validated numerical settings and sampled geometry.
/// It cannot authenticate the supplied problem/semantics identifiers, measurements
/// or actual-state origin, and it never issues a PDE-window acceptance status.
#[derive(Debug, Clone, Copy)]
pub struct FrozenProtocol<'a> {
    inputs: ProtocolInputs<'a>,
    identity: [u8; 32],
    encoded_bytes: usize,
}
impl<'a> FrozenProtocol<'a> {
    /// Bound the complete hash traversal before performing it. Input arrays remain
    /// borrowed; SHA-256 uses fixed storage and no serialization buffer is allocated.
    pub fn new(
        inputs: ProtocolInputs<'a>,
        maximum_encoded_bytes: usize,
    ) -> Result<Self, VerificationError> {
        let encoded_bytes = reservation(inputs)?;
        if encoded_bytes > maximum_encoded_bytes {
            return Err(VerificationError::CapacityExceeded);
        }
        if inputs.problem == [0; 32] || inputs.semantics == [0; 32] {
            return Err(VerificationError::MissingProtocolIdentity);
        }
        let required = required_records(
            inputs.policies.as_slice().len(),
            inputs.time_sets[2].as_slice().len(),
        )?;
        MeasurementReview::new(
            inputs.policies,
            inputs.time_sets,
            inputs.reconstruction,
            required,
        )?;
        Ok(Self {
            inputs,
            identity: encoding::identity(inputs),
            encoded_bytes,
        })
    }

    /// SHA-256 of the versioned canonical byte sequence, not an authenticity assertion.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Complete finite hash-input byte count, including every field and repeated clock.
    pub fn encoded_bytes(self) -> usize {
        self.encoded_bytes
    }
    /// Write the complete canonical policy artifact into caller-owned storage. Short
    /// buffers are refused before modification; trailing caller bytes remain unchanged.
    pub fn write_canonical(self, output: &mut [u8]) -> Result<usize, VerificationError> {
        if output.len() < self.encoded_bytes {
            return Err(VerificationError::CapacityExceeded);
        }
        Ok(encoding::write(self.inputs, output))
    }
    /// Borrowed settings cannot change while the frozen protocol or a derived review lives.
    pub fn inputs(self) -> ProtocolInputs<'a> {
        self.inputs
    }
    /// Start a bounded numerical review of exactly the fingerprinted settings.
    /// Attempt caps are execution limits, separately recorded from scientific protocol identity.
    pub fn review(
        self,
        maximum_attempts: usize,
    ) -> Result<MeasurementReview<'a>, VerificationError> {
        MeasurementReview::new(
            self.inputs.policies,
            self.inputs.time_sets,
            self.inputs.reconstruction,
            maximum_attempts,
        )
    }
}

/// Exact serialization-work reservation, with checked arithmetic and no allocation.
/// Caller-owned manifests and upstream admission work have their own resource budgets.
pub fn reservation(inputs: ProtocolInputs<'_>) -> Result<usize, VerificationError> {
    let policies = inputs.policies.as_slice().len().checked_mul(POLICY_BYTES);
    let clocks = inputs
        .time_sets
        .into_iter()
        .try_fold(0usize, |sum, times| sum.checked_add(times.as_slice().len()));
    let probes = inputs
        .reconstruction
        .as_slice()
        .len()
        .checked_mul(12 * CLOCK_BYTES);
    // Magic + two identities + policy count + three time counts + reconstruction count.
    let header = MAGIC.len() + 64 + 5 * 16;
    policies
        .and_then(|p| {
            clocks
                .and_then(|c| c.checked_mul(CLOCK_BYTES))
                .and_then(|c| p.checked_add(c))
        })
        .and_then(|n| probes.and_then(|p| n.checked_add(p)))
        .and_then(|n| n.checked_add(header))
        .ok_or(VerificationError::CapacityExceeded)
}
