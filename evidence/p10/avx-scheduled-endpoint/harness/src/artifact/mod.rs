//! Harness-owned transactional snapshot and record publication; no resume decoder exists.
use nsbu_solver::diagnostics::balances::BalanceSample;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

mod attempt;
mod snapshot;

pub use attempt::{publish_attempt, publish_status};
#[cfg(any(test, not(feature = "n384-prep")))]
pub use attempt::stage_attempt;
#[cfg(not(feature = "n384-prep"))]
pub use snapshot::{disk_preflight, publish_node, stage_node};
#[cfg(feature = "n384-prep")]
pub(crate) use snapshot::{node_json, write_snapshot};

pub const BUFFER_BYTES: usize = 1024 * 1024;
#[cfg(not(feature = "n384-prep"))]
pub const DISK_CAP_BYTES: usize = 4 * 1024 * 1024 * 1024;
#[cfg(feature = "n384-h32")]
pub const DISK_CAP_BYTES: usize = 256 * 1024 * 1024 * 1024;
#[cfg(any(
    feature = "n384-h64",
    feature = "n384-piecewise",
    feature = "n384-piecewise-cadv33",
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
))]
#[cfg(not(feature = "n512-m512-piecewise-cadv33"))]
pub const DISK_CAP_BYTES: usize = 128 * 1024 * 1024 * 1024;
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub const DISK_CAP_BYTES: usize = 512 * 1024 * 1024 * 1024;
#[cfg(feature = "n512-m512-temporal-h32")]
pub const DISK_CAP_BYTES: usize = 512 * 1024 * 1024 * 1024;
#[cfg(feature = "n512-m512-temporal-h16")]
pub const DISK_CAP_BYTES: usize = 768 * 1024 * 1024 * 1024;

pub struct NodeRecord<'a> {
    pub identity: &'a str,
    pub balance: BalanceSample,
    pub observer_seconds: f64,
    pub force_seconds: f64,
    pub conservative_seconds: f64,
    pub transfer_measure_seconds: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationKind {
    Attempt,
    #[cfg(not(feature = "n384-prep"))]
    Node,
    #[cfg(feature = "n384-prep")]
    Step,
}

impl PublicationKind {
    pub fn advances_durable_clock(self) -> bool {
        match self {
            Self::Attempt => false,
            #[cfg(not(feature = "n384-prep"))]
            Self::Node => true,
            #[cfg(feature = "n384-prep")]
            Self::Step => true,
        }
    }
}

pub struct StagedArtifact {
    pub(crate) partial: PathBuf,
    pub(crate) final_path: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) kind: PublicationKind,
    pub(crate) state_hash: Option<String>,
    pub(crate) published: bool,
    pub(crate) preserve_on_failure: bool,
}

impl StagedArtifact {
    pub fn publish(mut self) -> io::Result<PublishedArtifact> {
        self.publish_with_sync(|root| File::open(root)?.sync_all())
    }

    pub(crate) fn publish_with_sync(
        &mut self,
        sync_parent: impl FnOnce(&Path) -> io::Result<()>,
    ) -> io::Result<PublishedArtifact> {
        self.preserve_on_failure = true;
        if let Err(error) = fs::rename(&self.partial, &self.final_path) {
            return Err(self.publication_error(error));
        }
        if let Err(error) = sync_parent(&self.root) {
            return Err(self.publication_error(error));
        }
        self.published = true;
        Ok(PublishedArtifact {
            kind: self.kind,
            state_hash: self.state_hash.take(),
        })
    }

    fn publication_error(&self, cause: io::Error) -> io::Error {
        io::Error::new(
            cause.kind(),
            format!(
                "publication_unconfirmed partial_path={} partial_exists={} final_path={} final_exists={} parent_sync_confirmed=false cause={cause}",
                self.partial.display(),
                self.partial.exists(),
                self.final_path.display(),
                self.final_path.exists(),
            ),
        )
    }
}

impl Drop for StagedArtifact {
    fn drop(&mut self) {
        if !self.published && !self.preserve_on_failure {
            let _ = if self.partial.is_dir() {
                fs::remove_dir_all(&self.partial)
            } else {
                fs::remove_file(&self.partial)
            };
        }
    }
}

#[derive(Debug)]
pub struct PublishedArtifact {
    pub kind: PublicationKind,
    pub state_hash: Option<String>,
}

pub fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value <= '\u{1f}' => {
                use std::fmt::Write as _;
                write!(&mut escaped, "\\u{:04x}", value as u32).expect("String writes cannot fail");
            }
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
}

pub(crate) fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

pub(crate) fn create(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

pub(crate) fn partial_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.partial"))
}

pub(crate) fn refuse_existing(final_path: &Path, partial: &Path) -> io::Result<()> {
    if final_path.exists() || partial.exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "artifact path already exists",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
