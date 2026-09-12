//! Harness-owner transaction state: a proposal is staged before commit and published after it.
use crate::artifact::{PublicationKind, PublishedArtifact};
use std::io;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Frontiers {
    pub attempted: usize,
    pub in_memory_clock: u128,
    pub durable_clock: u128,
    pub durable_attempt: usize,
    pub provisional_clock: Option<u128>,
}

pub fn commit_staged<S, E, C, P>(
    frontiers: &mut Frontiers,
    attempt: usize,
    clock: u128,
    staged: Result<S, E>,
    commit: C,
    publish: P,
) -> Result<PublishedArtifact, E>
where
    E: From<io::Error>,
    C: FnOnce(),
    P: FnOnce(S) -> io::Result<PublishedArtifact>,
{
    let staged = staged?;
    frontiers.provisional_clock = Some(clock);
    commit();
    frontiers.in_memory_clock = clock;
    let published = publish(staged)?;
    frontiers.durable_attempt = attempt;
    if published.kind == PublicationKind::Node {
        frontiers.durable_clock = clock;
    }
    frontiers.provisional_clock = None;
    Ok(published)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{self, StagedArtifact};
    use std::{
        cell::Cell,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "p10-endpoint-owner-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn observer_failure_does_not_commit_or_move_any_frontier() {
        let committed = Cell::new(false);
        let mut frontiers = Frontiers {
            attempted: 1,
            ..Frontiers::default()
        };
        let staged: Result<StagedArtifact, io::Error> = Err(io::Error::other("observer failed"));
        let result = commit_staged(
            &mut frontiers,
            1,
            32,
            staged,
            || committed.set(true),
            StagedArtifact::publish,
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
        assert!(!committed.get());
        assert_eq!(frontiers.in_memory_clock, 0);
        assert_eq!(frontiers.durable_clock, 0);
        assert_eq!(frontiers.durable_attempt, 0);
        assert_eq!(frontiers.provisional_clock, None);
    }

    #[test]
    fn staged_write_failure_does_not_commit_or_move_any_frontier() {
        let committed = Cell::new(false);
        let mut frontiers = Frontiers {
            attempted: 16,
            ..Frontiers::default()
        };
        let staged: Result<StagedArtifact, io::Error> = Err(io::Error::other("write failed"));
        let result = commit_staged(
            &mut frontiers,
            16,
            512,
            staged,
            || committed.set(true),
            StagedArtifact::publish,
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
        assert!(!committed.get());
        assert_eq!(frontiers.durable_attempt, 0);
        assert_eq!(frontiers.durable_clock, 0);
    }

    #[test]
    fn publish_failure_discloses_in_memory_and_provisional_frontiers() {
        let root = root("publish-fail");
        let staged = artifact::stage_attempt(&root, 1, "{}\n").unwrap();
        fs::create_dir(root.join("attempt-001.json")).unwrap();
        let committed = Cell::new(false);
        let mut frontiers = Frontiers {
            attempted: 1,
            ..Frontiers::default()
        };
        let result = commit_staged(
            &mut frontiers,
            1,
            32,
            Ok::<_, io::Error>(staged),
            || committed.set(true),
            StagedArtifact::publish,
        );
        assert!(result.is_err());
        assert!(committed.get());
        assert_eq!(frontiers.in_memory_clock, 32);
        assert_eq!(frontiers.durable_clock, 0);
        assert_eq!(frontiers.durable_attempt, 0);
        assert_eq!(frontiers.provisional_clock, Some(32));
        assert!(root.join("attempt-001.json.partial").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_attempt_publication_moves_attempt_frontier_only() {
        let root = root("success");
        let staged = artifact::stage_attempt(&root, 1, "{}\n").unwrap();
        let mut frontiers = Frontiers {
            attempted: 1,
            ..Frontiers::default()
        };
        commit_staged(
            &mut frontiers,
            1,
            32,
            Ok::<_, io::Error>(staged),
            || {},
            StagedArtifact::publish,
        )
        .unwrap();
        assert_eq!(frontiers.in_memory_clock, 32);
        assert_eq!(frontiers.durable_clock, 0);
        assert_eq!(frontiers.durable_attempt, 1);
        assert_eq!(frontiers.provisional_clock, None);
        fs::remove_dir_all(root).unwrap();
    }
}
