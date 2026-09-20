//! Fault-injected publication contracts: every named fault stage must fail
//! loudly, report the genuine kernel error and residual uncertainty, leave
//! either a provably stated residue or provably nothing, and never leak a
//! hidden temporary. Split per fault so each contract reads and measures on
//! its own.
use crate::publication::{self, PublicationFault};
use std::path::{Path, PathBuf};
use std::{fs, io};

fn faults_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-faults")
}

fn faults_directory(name: &str) -> PathBuf {
    let directory = faults_root().join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("fault directory");
    directory
}

fn refused(directory: &Path, name: &str, fault: PublicationFault, pre: Option<&[u8]>) -> (PathBuf, String) {
    let path = directory.join(name);
    if let Some(pre_existing) = pre {
        fs::write(&path, pre_existing).expect("pre-existing write");
    }
    let error = publication::publish_with_fault(&path, b"payload".as_slice(), Some(fault))
        .expect_err("injected fault must fail publication");
    (path, error)
}

/// The final residue contract for one fault: the named file either exists
/// with exactly the expected bytes or not at all, and no hidden temporary is
/// left behind in the directory.
fn assert_residue(directory: &Path, path: &Path, expected: Option<&[u8]>) {
    let residue = match path.symlink_metadata() {
        Ok(_) => Some(fs::read(path).expect("residue read")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => panic!("residue check must succeed: {error}"),
    };
    match expected {
        Some(bytes) => assert_eq!(residue.expect("attached file must remain"), bytes),
        None => assert!(residue.is_none(), "publication must leave nothing at {}", path.display()),
    }
    let leftovers: Vec<_> = fs::read_dir(directory)
        .expect("listing")
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .filter(|name| name.starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "temporary leak: {leftovers:?}");
}

#[test]
fn directory_sync_fault_reports_the_genuine_kernel_error() {
    let directory = faults_directory("sync");
    let (path, error) = refused(&directory, "sync.json", PublicationFault::DirectorySync, None);
    assert!(error.contains("directory-sync"), "{error}");
    assert!(error.contains("uncertainty"), "{error}");
    assert!(error.contains("PermissionDenied"), "real kernel error must surface: {error}");
    assert_residue(&directory, &path, Some(b"payload".as_slice()));
    fs::remove_file(&path).expect("cleanup");
}

#[test]
fn write_failure_cleanup_reports_the_leftover_it_cannot_remove() {
    let directory = faults_directory("cleanup");
    let (path, error) =
        refused(&directory, "cleanup.json", PublicationFault::CleanupAfterWriteFailure, None);
    assert!(error.contains("cleanup"), "{error}");
    assert!(error.contains("uncertainty"), "{error}");
    assert!(!path.exists(), "nothing may be published after a write failure");
    // The injected fault denies the unlink itself: one dot-file genuinely
    // remains and must be named by the refusal, then removed explicitly.
    assert!(error.contains(".tmp"), "the refusal must name the leftover: {error}");
    let leftovers: Vec<_> = fs::read_dir(&directory)
        .expect("listing")
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
        .filter(|name| name.to_string_lossy().starts_with('.'))
        .collect();
    assert_eq!(leftovers.len(), 1, "denied unlink must leave one leftover: {leftovers:?}");
    for leftover in leftovers {
        fs::remove_file(directory.join(leftover)).expect("remove leftover");
    }
}

#[test]
fn truncated_attachment_is_removed_and_reported() {
    let directory = faults_directory("truncate");
    let (path, error) =
        refused(&directory, "truncate.json", PublicationFault::TruncateAttached, None);
    assert!(error.contains("completeness"), "{error}");
    assert!(error.contains("removed"), "{error}");
    assert_residue(&directory, &path, None);
}

#[test]
fn post_attach_temporary_gone_is_reported_as_absent_not_leftover() {
    let directory = faults_directory("attach-cleanup");
    let (path, error) =
        refused(&directory, "attach-cleanup.json", PublicationFault::CleanupAfterAttach, None);
    assert!(error.contains("cleanup"), "{error}");
    // The kernel answers ENOENT: the report must say the temporary is absent,
    // never claim that a duplicate temporary name remains.
    assert!(error.contains("already absent"), "{error}");
    assert!(!error.contains("duplicate temporary name remains"), "{error}");
    assert_residue(&directory, &path, Some(b"payload".as_slice()));
    fs::remove_file(&path).expect("cleanup attached file");
}

#[test]
fn attach_race_leaves_the_competing_file_untouched() {
    let directory = faults_directory("race");
    let (path, error) = refused(&directory, "race.json", PublicationFault::AttachRace, None);
    assert!(error.contains("appeared during publication"), "{error}");
    assert_residue(&directory, &path, Some(b"raced".as_slice()));
    fs::remove_file(&path).expect("cleanup raced file");
}
