use nsbu_solver::{diagnostics::norms::Norms, domain::Domain, Complex64};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const SOURCE: &str = "aed49b7d7874a0a720dee88b65ba180c7286fa65";
pub(crate) const W3_SOURCE: &str = "f13c29c9ae91d0b8cf7a790132deb9bd076911c0";
pub(crate) const CASE_SHA256: &str =
    "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";
pub(crate) const PLAN_SHA256: &str =
    "2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a";
pub(crate) const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";
pub(crate) const PROBE: u128 = 1112;
pub(crate) const SUPPORTS: [[u128; 3]; 3] =
    [[896, 1152, 1408], [1024, 1152, 1280], [1088, 1152, 1216]];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProbePlan {
    pub schema: String,
    pub status: String,
    pub source_commit: String,
    pub w3_source_commit: String,
    pub case_sha256: String,
    pub frozen_plan: PathBuf,
    pub frozen_plan_sha256: String,
    pub profile: String,
    pub snapshot_identity: String,
    pub dimensions: [usize; 3],
    pub lengths: [f64; 3],
    pub viscosity: f64,
    pub quantum_exponent: i32,
    pub clock_target: u128,
    pub method: String,
    pub integration_force_dimensions: [usize; 3],
    pub integration_force_workers: usize,
    pub residual_force_dimensions: [usize; 3],
    pub residual_force_workers: usize,
    pub advective_limit: f64,
    pub probe_clock: u128,
    pub supports: [[u128; 3]; 3],
    pub nodes: Vec<NodeBinding>,
    pub claims: Claims,
}

impl ProbePlan {
    pub(crate) fn domain(&self) -> Result<Domain, String> {
        Domain::new(self.dimensions, self.lengths, self.viscosity).map_err(debug)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeBinding {
    pub clock: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
    pub snapshot: PathBuf,
    pub coefficient_sha256: String,
    pub file_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Claims {
    pub runtime_owner_imported: bool,
    pub accepted_interpolation: bool,
    pub acceptance_windows: usize,
    pub arithmetic: String,
    pub purpose: String,
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub coefficients: [Vec<Complex64>; 3],
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct NormOutput {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
    pub divergence_l2: f64,
}

impl From<Norms> for NormOutput {
    fn from(value: Norms) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ReservationOutput {
    pub schema: &'static str,
    pub source_state_bytes: usize,
    pub diagnostic_field_bytes: usize,
    pub fft_catalog_bytes: usize,
    pub integration_rhs_bytes: usize,
    pub integration_force_bytes: usize,
    pub residual_force_bytes: usize,
    pub conservative_workspace_bytes: usize,
    pub reconstruction_peak_bytes: usize,
    pub residual_peak_bytes: usize,
    pub admitted_peak_bytes: usize,
    pub cap_bytes: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct NodeOutput {
    pub clock: u128,
    pub state_file_sha256: String,
    pub coefficient_sha256: String,
    pub physical_derivative_sha256: String,
    pub rhs_calls: usize,
    pub rhs_work_units: usize,
    pub rhs_scalar_transforms: usize,
    pub force_cache_hits: usize,
    pub force_cache_misses: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ScaleOutput {
    pub label: &'static str,
    pub support: [u128; 3],
    pub probe: u128,
    pub residual_acceleration: NormOutput,
    pub residual_force_work_units: usize,
    pub residual_force_scalar_transforms: usize,
    pub reconstructed_value_sha256: String,
    pub reconstructed_derivative_sha256: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DifferenceOutput {
    pub left: &'static str,
    pub right: &'static str,
    pub reconstructed_velocity: NormOutput,
    pub reconstructed_derivative: NormOutput,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunOutput {
    pub schema: &'static str,
    pub status: &'static str,
    pub source_commit: &'static str,
    pub w3_source_commit: &'static str,
    pub case_sha256: &'static str,
    pub frozen_plan_sha256: &'static str,
    pub profile: &'static str,
    pub arithmetic: &'static str,
    pub integration_force: &'static str,
    pub residual_force: &'static str,
    pub retained_grid: [usize; 3],
    pub diagnostic_grid: [usize; 3],
    pub probe_clock: u128,
    pub supports: [[u128; 3]; 3],
    pub support_relationship: &'static str,
    pub reservations: ReservationOutput,
    pub nodes: Vec<NodeOutput>,
    pub scales: Vec<ScaleOutput>,
    pub differences: Vec<DifferenceOutput>,
    pub runtime_owner_imported: bool,
    pub accepted_interpolation: bool,
    pub acceptance_windows: usize,
    pub qualification: &'static str,
}

pub(crate) fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
