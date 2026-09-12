use nsbu_solver::{domain::Domain, Complex64};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub schema: String,
    pub snapshot: PathBuf,
    pub plan: PathBuf,
    pub identity: String,
    pub source_commit: String,
    pub plan_sha256: String,
    pub coefficient_sha256: String,
    pub file_sha256: String,
    pub backend: String,
    pub execution: String,
    pub dimensions: [usize; 3],
    pub evolution: Evolution,
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
}

impl Manifest {
    pub(crate) fn domain(&self) -> Result<Domain, String> {
        Domain::new(
            self.dimensions,
            self.evolution.lengths,
            self.evolution.viscosity,
        )
        .map_err(debug)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evolution {
    pub case_sha256: String,
    pub quantum_exponent: i32,
    pub clock_target: u128,
    pub comparison_endpoint: u128,
    pub lengths: [f64; 3],
    pub viscosity: f64,
    pub method: String,
    pub integration_force_dimensions: [usize; 3],
    pub schedule: Vec<ScheduleSegment>,
    pub absolute_tolerances: [f64; 2],
    pub relative_tolerances: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScheduleSegment {
    pub from_inclusive: u128,
    pub until_exclusive: u128,
    pub step_ticks: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClockHeader {
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
}

impl ClockHeader {
    pub(crate) fn from_manifest(manifest: &Manifest) -> Self {
        Self {
            elapsed: manifest.elapsed,
            target: manifest.target,
            epoch: manifest.epoch,
            accepted_steps: manifest.accepted_steps,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub coefficients: [Vec<Complex64>; 3],
    pub coefficient_sha256: String,
    pub file_sha256: String,
    pub clock: ClockHeader,
}

#[derive(Debug, Serialize)]
pub(crate) struct Hashes<'a> {
    pub coefficient_sha256: &'a str,
    pub file_sha256: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClockOutput {
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub left_accepted_steps: u128,
    pub right_accepted_steps: u128,
}

#[derive(Debug, Serialize)]
pub(crate) struct Output<'a> {
    pub schema: &'static str,
    pub evolution: &'a Evolution,
    pub left_identity: &'a str,
    pub left_backend: &'a str,
    pub left_execution: &'a str,
    pub left_source_commit: &'a str,
    pub left_plan_sha256: &'a str,
    pub right_identity: &'a str,
    pub right_backend: &'a str,
    pub right_execution: &'a str,
    pub right_source_commit: &'a str,
    pub right_plan_sha256: &'a str,
    pub left_hashes: Hashes<'a>,
    pub right_hashes: Hashes<'a>,
    pub clock: ClockOutput,
    pub full: NormOutput,
    pub common: NormOutput,
    pub newly_resolved: NormOutput,
    pub fine_absolute: NormOutput,
    pub mean_error: [f64; 3],
    pub admitted_bytes: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct NormOutput {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
    pub divergence_l2: f64,
}

impl From<nsbu_solver::diagnostics::norms::Norms> for NormOutput {
    fn from(value: nsbu_solver::diagnostics::norms::Norms) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

pub(crate) fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
