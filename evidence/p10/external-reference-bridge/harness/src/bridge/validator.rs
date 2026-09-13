use super::*;

pub(super) fn validate_bridge(bridge: &BridgeManifest) -> Result<(), String> {
    let identity = [
        bridge.schema == "p10-external-reference-bridge-input-v1",
        hex(&bridge.snapshot_manifest_sha256, 64),
        hex(&bridge.case_sha256, 64),
        hex(&bridge.reference_source_commit, 40),
        bridge.reference_source_commit == REFERENCE_SOURCE_COMMIT,
        hex(&bridge.harness_source_commit, 40),
        hex(&bridge.binary_sha256, 64),
        bridge.case_sha256 == CASE_SHA256,
    ];
    let arithmetic = [
        bridge.reference_evaluator == EVALUATOR,
        bridge.arithmetic == ARITHMETIC,
        bridge.physical_grid == GRID,
        bridge.fft_normalization == NORMALIZATION,
        bridge.crop == CROP,
        bridge.nyquist == NYQUIST,
        bridge.projection == PROJECTION,
        !bridge.coordinate_shift,
        !bridge.mean_alignment,
        bridge.classification == CLASSIFICATION,
        bridge.execution_context == EXECUTION_CONTEXT,
        bridge.execution_cap_bytes > 0,
    ];
    let clock = bridge.clock_exponent == -20
        && bridge.clock_target == 8192
        && bridge.elapsed > 0
        && bridge.elapsed < bridge.clock_target;
    if !identity.into_iter().all(std::convert::identity)
        || !arithmetic.into_iter().all(std::convert::identity)
        || !clock
    {
        return Err("invalid external reference bridge binding".into());
    }
    let (Ok(samples), Ok(retained)) = (
        Layout::new(bridge.sample_dimensions),
        Layout::new(bridge.retained_dimensions),
    ) else {
        return Err("invalid external reference bridge dimensions".into());
    };
    if retained
        .dimensions()
        .into_iter()
        .zip(samples.dimensions())
        .any(|(n, m)| n > m)
    {
        return Err("reference sample grid is smaller than retained grid".into());
    }
    backend(bridge)?;
    validate_sources(bridge)
}

fn validate_sources(bridge: &BridgeManifest) -> Result<(), String> {
    let matched = bridge.reference_sources.len() == SOURCES.len()
        && bridge
            .reference_sources
            .iter()
            .zip(SOURCES)
            .all(|(actual, expected)| actual.role == expected.0 && actual.sha256 == expected.1);
    matched
        .then_some(())
        .ok_or_else(|| "reference source closure mismatch".into())
}

pub(super) fn validate_binding(
    bridge: &BridgeManifest,
    snapshot: &SnapshotManifest,
) -> Result<(), String> {
    let matched = [
        snapshot.dimensions == bridge.retained_dimensions,
        snapshot.evolution.case_sha256 == bridge.case_sha256,
        snapshot.evolution.quantum_exponent == bridge.clock_exponent,
        snapshot.target == bridge.clock_target,
        snapshot.elapsed == bridge.elapsed,
        snapshot.evolution.comparison_endpoint == bridge.elapsed,
        snapshot.evolution.lengths == [1.0; 3],
        snapshot.evolution.viscosity == 1.0,
        snapshot.backend == bridge.fft_backend,
        identity_value(&snapshot.identity, "backend") == Some(bridge.fft_backend.as_str()),
    ];
    if !matched.into_iter().all(std::convert::identity) {
        return Err("snapshot/reference binding mismatch".into());
    }
    TickClock::restore(
        bridge.clock_exponent,
        bridge.clock_target,
        bridge.elapsed,
        bridge.clock_target - bridge.elapsed,
    )
    .map_err(debug)?;
    Ok(())
}

pub(super) fn validate_snapshot_review(
    bridge: &BridgeManifest,
    snapshot: &SnapshotManifest,
) -> Result<(), String> {
    use crate::model::{ComparisonKind, ProfileBindingKind};
    let profile = snapshot.profile.as_ref();
    let guard = snapshot.admission_guard.as_ref();
    let steps = schedule_steps(snapshot)?;
    let trajectory = trajectory(snapshot);
    let matched = [
        snapshot.comparison_kind == ComparisonKind::MatchedSpatial,
        profile.is_some_and(|value| {
            value.kind == ProfileBindingKind::IdentityProfileField
                && trajectory.is_some_and(|accepted| value.value == accepted.profile)
        }),
        trajectory.is_some_and(|accepted| {
            snapshot.source_commit == accepted.source
                && snapshot.plan_sha256 == accepted.plan_sha256
                && snapshot.evolution.integration_force_dimensions == [accepted.force_samples; 3]
                && identity_value(&snapshot.identity, "profile") == Some(accepted.profile)
                && identity_usize(&snapshot.identity, "force_samples")
                    == Some(accepted.force_samples)
        }),
        identity_value(&snapshot.identity, "case") == Some(bridge.case_sha256.as_str()),
        identity_value(&snapshot.identity, "source") == Some(snapshot.source_commit.as_str()),
        identity_usize(&snapshot.identity, "retained") == Some(384),
        identity_usize(&snapshot.identity, "observer_force_samples") == Some(768),
        identity_usize(&snapshot.identity, "execution_cap") == Some(SNAPSHOT_EXECUTION_CAP),
        identity_usize(&snapshot.identity, "artifact_cap") == Some(SNAPSHOT_ARTIFACT_CAP),
        snapshot.accepted_steps == steps,
        snapshot.epoch == steps,
        guard.is_some_and(|value| {
            value.advective_limit.to_bits() == 3.3_f64.to_bits() && value.maximum_attempts == 48
        }),
        expected_schedule(snapshot),
        admitted_state_hashes(snapshot),
    ];
    matched
        .into_iter()
        .all(std::convert::identity)
        .then_some(())
        .ok_or_else(|| "snapshot reviewed profile/schedule/header/cap binding mismatch".into())
}

#[derive(Clone, Copy)]
struct Trajectory {
    profile: &'static str,
    source: &'static str,
    plan_sha256: &'static str,
    force_samples: usize,
}

fn trajectory(snapshot: &SnapshotManifest) -> Option<Trajectory> {
    let profile = snapshot.profile.as_ref()?.value.as_str();
    match profile {
        PROFILE_M384 => Some(Trajectory {
            profile: PROFILE_M384,
            source: SOURCE_M384,
            plan_sha256: PLAN_M384,
            force_samples: 384,
        }),
        PROFILE_M512 => Some(Trajectory {
            profile: PROFILE_M512,
            source: SOURCE_M512,
            plan_sha256: PLAN_M512,
            force_samples: 512,
        }),
        _ => None,
    }
}

fn schedule_steps(snapshot: &SnapshotManifest) -> Result<u128, String> {
    snapshot
        .evolution
        .schedule
        .iter()
        .try_fold(0_u128, |sum, segment| {
            let steps = (segment.until_exclusive - segment.from_inclusive) / segment.step_ticks;
            sum.checked_add(steps)
                .ok_or_else(|| "schedule step overflow".into())
        })
}

fn expected_schedule(snapshot: &SnapshotManifest) -> bool {
    let segments = &snapshot.evolution.schedule;
    match snapshot.elapsed {
        512 | 1024 | 2048 => segments.len() == 1 && segment(&segments[0], 0, snapshot.elapsed, 64),
        4096 => {
            segments.len() == 2
                && segment(&segments[0], 0, 2048, 64)
                && segment(&segments[1], 2048, 4096, 128)
        }
        _ => false,
    }
}

fn admitted_state_hashes(snapshot: &SnapshotManifest) -> bool {
    if snapshot.source_commit != SOURCE_M512 {
        return true;
    }
    let expected = match snapshot.elapsed {
        512 => (
            "4fbfa9890470ab61dca7ddbb026fd1f93c2f0959d15bf87e713ee0a9111c02af",
            "7a1d8d21e17c85c7f37ea474f5f5e694a91889ebabcec12424427308d020def9",
        ),
        1024 => (
            "bb12be8f266268813ffbeddc3c78659bc84efb2361f14dfd471e5da354fc2324",
            "d62fdf81db2547e6343e21e6be6faabfdf0130a8e686d771785434fe2bd16365",
        ),
        2048 => (
            "461e6f2a95eb578558493bbacebc5456c7e8a3e8f8fb933ac0176a23a3f67cad",
            "25307e71e89cfbdf5ea677efe0c5c8c161e99b4518aa435e431ba553976cdeaa",
        ),
        _ => return false,
    };
    snapshot.coefficient_sha256 == expected.0 && snapshot.file_sha256 == expected.1
}

fn segment(value: &crate::model::ScheduleSegment, from: u128, until: u128, step: u128) -> bool {
    value.from_inclusive == from && value.until_exclusive == until && value.step_ticks == step
}

fn identity_value<'a>(identity: &'a str, key: &str) -> Option<&'a str> {
    let mut matches = identity
        .split(';')
        .filter_map(|field| field.split_once('='))
        .filter(|(name, _)| *name == key)
        .map(|(_, value)| value);
    let value = matches.next()?;
    matches.next().is_none().then_some(value)
}

fn identity_usize(identity: &str, key: &str) -> Option<usize> {
    identity_value(identity, key)?.parse().ok()
}
