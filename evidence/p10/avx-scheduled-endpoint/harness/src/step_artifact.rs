//! N384 every-accepted-step bundles; the schema deliberately has no resume decoder.
use crate::artifact::{self, NodeRecord, PublicationKind, StagedArtifact};
use nsbu_solver::{domain::SpectralState, SolverError};
use std::{
    fs::{self, File},
    io,
    path::Path,
};

const SNAPSHOT_HEADER_ALLOWANCE: usize = 4096;
const RECORD_ALLOWANCE: usize = 2 * 4096;
const FILESYSTEM_ALLOWANCE: usize = 64 * 1024;

pub enum Observation<'a> {
    Scheduled(NodeRecord<'a>),
    NotScheduled { identity: &'a str },
}

impl Observation<'_> {
    fn identity(&self) -> &str {
        match self {
            Self::Scheduled(record) => record.identity,
            Self::NotScheduled { identity } => identity,
        }
    }
}

pub fn disk_preflight(state_bytes: usize, steps: usize) -> Result<usize, SolverError> {
    state_bytes
        .checked_add(SNAPSHOT_HEADER_ALLOWANCE + RECORD_ALLOWANCE + FILESYSTEM_ALLOWANCE)
        .and_then(|bytes| bytes.checked_mul(steps))
        .filter(|&bytes| bytes <= artifact::DISK_CAP_BYTES)
        .ok_or(SolverError::ResourceLimit)
}

pub fn publish_rest(root: &Path, identity: &str) -> io::Result<()> {
    let json = format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-n384-rest-v1\",\n",
            "  \"identity\": {},\n  \"clock\": 0,\n",
            "  \"state_payload\": false,\n",
            "  \"observation_status\": \"RestExact\",\n",
            "  \"balance\": \"REST\"\n}}\n"
        ),
        artifact::json_string(identity),
    );
    artifact::publish_status(root, "rest.json", &json)
}

pub fn stage_step(
    root: &Path,
    index: usize,
    state: &SpectralState,
    observation: Observation<'_>,
    attempt_json: &str,
) -> io::Result<StagedArtifact> {
    let clock = state.clock().elapsed();
    let name = format!("step-{index:03}-clock-{clock:04}");
    let final_path = root.join(&name);
    let partial = root.join(format!("{name}.partial"));
    refuse_existing(&final_path, &partial)?;
    fs::create_dir(&partial)?;
    let staged = stage_contents(&partial, state, observation, attempt_json);
    let hash = finish_staging(&partial, staged)?;
    Ok(StagedArtifact {
        partial,
        final_path,
        root: root.to_owned(),
        kind: PublicationKind::Step,
        state_hash: Some(hash),
        published: false,
        preserve_on_failure: false,
    })
}

fn stage_contents(
    partial: &Path,
    state: &SpectralState,
    observation: Observation<'_>,
    attempt_json: &str,
) -> io::Result<String> {
    let (hash, bytes) =
        artifact::write_snapshot(&partial.join("state.bin"), state, observation.identity())?;
    let record = record_json(state, observation, &hash, bytes);
    artifact::write_file(&partial.join("record.json"), record.as_bytes())?;
    artifact::write_file(&partial.join("attempt.json"), attempt_json.as_bytes())?;
    Ok(hash)
}

fn finish_staging(partial: &Path, staged: io::Result<String>) -> io::Result<String> {
    let result = staged.and_then(|hash| {
        File::open(partial)?.sync_all()?;
        Ok(hash)
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(partial);
    }
    result
}

fn record_json(
    state: &SpectralState,
    observation: Observation<'_>,
    hash: &str,
    coefficient_bytes: usize,
) -> String {
    match observation {
        Observation::Scheduled(record) => {
            artifact::node_json(state, record, hash, coefficient_bytes).replacen(
                "\"schema\": \"p10-avx-scheduled-node-v1\",",
                "\"schema\": \"p10-avx-n384-step-v1\",\n  \"observation_status\": \"Scheduled\",",
                1,
            )
        }
        Observation::NotScheduled { identity } => {
            not_scheduled_json(state, identity, hash, coefficient_bytes)
        }
    }
}

fn not_scheduled_json(
    state: &SpectralState,
    identity: &str,
    hash: &str,
    coefficient_bytes: usize,
) -> String {
    format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-n384-step-v1\",\n",
            "  \"identity\": {},\n  \"resumable\": false,\n",
            "  \"clock\": {},\n  \"epoch\": {},\n  \"accepted_steps\": {},\n",
            "  \"coefficient_bytes\": {},\n  \"state_sha256\": \"{}\",\n",
            "  \"observation_status\": \"NotScheduled\"\n}}\n"
        ),
        artifact::json_string(identity),
        state.clock().elapsed(),
        state.epoch().0,
        state.accepted_steps(),
        coefficient_bytes,
        hash,
    )
}

fn refuse_existing(final_path: &Path, partial: &Path) -> io::Result<()> {
    if final_path.exists() || partial.exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "step bundle path already exists",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_solver::{
        diagnostics::balances::BalanceSample,
        domain::{Domain, Epoch, ExtraStorage, ResourcePlan, TickClock},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exact_n384_artifact_profiles_are_bounded() {
        let state_bytes = 1_366_032_384;
        assert_eq!(disk_preflight(state_bytes, 64).unwrap(), 87_431_053_312);
        #[cfg(any(feature = "n384-piecewise", feature = "n384-piecewise-cadv33"))]
        assert_eq!(disk_preflight(state_bytes, 48).unwrap(), 65_573_289_984);
        #[cfg(feature = "n384-h32")]
        assert_eq!(disk_preflight(state_bytes, 128).unwrap(), 174_862_106_624);
    }

    #[test]
    fn rest_is_metadata_only_and_unscheduled_step_has_no_balance() {
        let root = root("unscheduled");
        publish_rest(&root, "identity").unwrap();
        assert!(!root.join("state.bin").exists());
        let state = state();
        stage_step(
            &root,
            1,
            &state,
            Observation::NotScheduled {
                identity: "identity",
            },
            "{\"outcome\":\"committed\"}\n",
        )
        .unwrap()
        .publish()
        .unwrap();
        let bundle = root.join("step-001-clock-0000");
        let record = fs::read_to_string(bundle.join("record.json")).unwrap();
        assert!(record.contains("\"observation_status\": \"NotScheduled\""));
        assert!(!record.contains("\"balance\""));
        assert!(bundle.join("state.bin").exists());
        assert!(bundle.join("attempt.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scheduled_step_carries_actual_balance_in_same_bundle() {
        let root = root("scheduled");
        let state = state();
        let record = NodeRecord {
            identity: "identity",
            balance: BalanceSample::REST,
            observer_seconds: 1.0,
            force_seconds: 0.5,
            conservative_seconds: 0.25,
            transfer_measure_seconds: 0.25,
        };
        stage_step(&root, 16, &state, Observation::Scheduled(record), "{}\n")
            .unwrap()
            .publish()
            .unwrap();
        let bundle = root.join("step-016-clock-0000");
        let json = fs::read_to_string(bundle.join("record.json")).unwrap();
        assert!(json.contains("\"observation_status\": \"Scheduled\""));
        assert!(json.contains("\"balance\""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parent_sync_failure_reports_unconfirmed_final_location() {
        let root = root("sync-failure");
        let state = state();
        let mut staged = stage_step(
            &root,
            1,
            &state,
            Observation::NotScheduled {
                identity: "identity",
            },
            "{}\n",
        )
        .unwrap();
        let error = staged
            .publish_with_sync(|_| Err(io::Error::other("injected parent sync")))
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("partial_exists=false"));
        assert!(message.contains("final_exists=true"));
        assert!(message.contains("parent_sync_confirmed=false"));
        assert!(root.join("step-001-clock-0000").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn state() -> SpectralState {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let resources = ResourcePlan::new(
            domain,
            ExtraStorage {
                fft: 0,
                force: 0,
                diagnostics: 0,
                overhead: 0,
            },
            1024 * 1024,
            Epoch(0),
        )
        .unwrap();
        SpectralState::from_rest(
            resources,
            TickClock::from_rest(-20, 8192).unwrap(),
            Epoch(0),
        )
        .unwrap()
    }

    fn root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "p10-n384-step-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        path
    }
}
