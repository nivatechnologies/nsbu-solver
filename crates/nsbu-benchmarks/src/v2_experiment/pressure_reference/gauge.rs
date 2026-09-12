//! Exact-clock imports of complete independent empirical pressure-mean artifacts.
use nsbu_solver::domain::TickClock;
use sha2::{Digest, Sha256};

/// One raw high-precision pressure mean and its quadrature/arithmetic settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GaugeEstimate {
    /// Composite-Simpson axial panel count.
    pub axial_panels: usize,
    /// Composite-Simpson radial-squared panel count.
    pub radial_panels: usize,
    /// Decimal arithmetic precision.
    pub precision: usize,
    /// Unrounded decimal result from the independent artifact.
    pub raw_mean: &'static str,
}

/// Empirical changes retained separately; none is promoted to a certified bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GaugeChanges {
    /// Joint 8-to-16 panel change.
    pub coarse_to_middle: &'static str,
    /// Joint 16-to-32 panel change.
    pub middle_to_fine: &'static str,
    /// Axial-only comparison against the finest joint result.
    pub axial_only_to_fine: &'static str,
    /// Radial-only comparison against the finest joint result.
    pub radial_only_to_fine: &'static str,
    /// Same-geometry 80-versus-120-digit differences.
    pub precision_differences: [&'static str; 5],
    /// Maximum arithmetic difference relative to finest grid-change scale.
    pub maximum_precision_to_finest_quadrature_ratio: Option<&'static str>,
    /// Finest 120-digit empirical global mean.
    pub finest_mean: &'static str,
    /// Finest grid-change scale relative to the finest mean.
    pub relative_finest_quadrature_change: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct GaugeData {
    elapsed: u128,
    sha256: [u8; 32],
    estimates: &'static [GaugeEstimate; 10],
    changes: GaugeChanges,
}
include!("gauge_data.rs");

/// Bounded import failure; no partially decoded evidence is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeError {
    /// Serialized artifact exceeds the admitted byte/hash traversal cap.
    CapacityExceeded,
    /// Bytes do not exactly match a frozen independently generated artifact.
    InvalidArtifact,
    /// Frozen decimal mean cannot be represented as finite binary64.
    InvalidMean,
}

/// One exact-clock imported empirical gauge, retaining its authoritative bytes.
#[derive(Debug, Clone, Copy)]
pub struct ImportedGauge<'a> {
    bytes: &'a [u8],
    data: GaugeData,
    means: [f64; 10],
    mean: f64,
}
impl<'a> ImportedGauge<'a> {
    /// Verify actual serialized bytes, identify their exact clock and parse the finest mean.
    pub fn load(bytes: &'a [u8], hash_byte_cap: usize) -> Result<Self, GaugeError> {
        if bytes.len() > hash_byte_cap {
            return Err(GaugeError::CapacityExceeded);
        }
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        let data = DATA
            .iter()
            .copied()
            .find(|candidate| candidate.sha256 == digest)
            .ok_or(GaugeError::InvalidArtifact)?;
        let mut means = [0.0; 10];
        for (output, estimate) in means.iter_mut().zip(data.estimates) {
            *output = estimate
                .raw_mean
                .parse::<f64>()
                .map_err(|_| GaugeError::InvalidMean)?;
            if !output.is_finite() {
                return Err(GaugeError::InvalidMean);
            }
        }
        let mean = means[7];
        Ok(Self {
            bytes,
            data,
            means,
            mean,
        })
    }
    /// Frozen exact solver clock carried by this artifact.
    pub fn clock(self) -> TickClock {
        TickClock::restore(-20, 8192, self.data.elapsed, 8192 - self.data.elapsed)
            .expect("generated gauge clock is valid")
    }
    /// SHA-256 of the exact imported bytes.
    pub fn artifact_sha256(self) -> [u8; 32] {
        self.data.sha256
    }
    /// Authoritative serialized artifact bytes.
    pub fn artifact_bytes(self) -> &'a [u8] {
        self.bytes
    }
    /// Generated typed projection of all ten raw estimates in producer order.
    pub fn estimates(self) -> &'static [GaugeEstimate; 10] {
        self.data.estimates
    }
    /// Binary64 roundings of all ten raw means in matching producer order.
    pub fn binary64_means(self) -> [f64; 10] {
        self.means
    }
    /// Generated projection of separate empirical arithmetic and quadrature changes.
    pub fn changes(self) -> GaugeChanges {
        self.data.changes
    }
    /// Rounded binary64 global mean used by the Rust comparison consumer.
    pub fn mean(self) -> f64 {
        self.mean
    }
}
