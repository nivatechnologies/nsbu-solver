//! Atomic create-only result publication: results are written exactly once,
//! completely, and never overwrite or replace an existing artifact.
//!
//! The payload is written to a fresh same-directory temporary file, synchronized,
//! and then atomically attached to the final name with `hard_link`, which the
//! kernel refuses when the final name exists (including a raced creation). The
//! parent directory is synchronized after the attach on every completed attach
//! path, including when the post-attach temporary cleanup fails, so a cleanup
//! refusal never suppresses the durability sync of the attached name. Every
//! stage failure is terminal and reported truthfully: cleanup errors state
//! exactly which file may remain — a residue check must confirm existence
//! before anything is said to remain, and the kernel's ENOENT answer is
//! reported as the temporary being absent, never as a leftover; completeness
//! rollback removes the incomplete attachment and synchronizes the parent after
//! the removal (or reports the removal's durability uncertainty explicitly);
//! parent-directory synchronization failures report the uncertainty they leave
//! behind. The directory faults are injected at real operation boundaries:
//! revoking the parent's permissions makes the production open/sync/remove
//! calls fail with the genuine kernel errors, then restores them.
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
};

/// Failure-injection points used by the publication fault tests; the production
/// entry point only ever runs the unfaulted path.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy)]
pub(crate) enum PublicationFault {
    /// Fail the payload write after the temporary file is created.
    Write,
    /// Fail the write and then refuse to remove the temporary file.
    CleanupAfterWriteFailure,
    /// Revoke the parent directory's permissions so the real post-attach
    /// open-and-sync of the parent fails at its own operation boundary.
    DirectorySync,
    /// Create the final name just before the attach to force a raced creation.
    AttachRace,
    /// Truncate the attached file so the completeness check sees a short length.
    TruncateAttached,
    /// Remove the temporary link before the post-attach cleanup sees it, forcing
    /// a genuine not-found cleanup failure at the real unlink boundary.
    CleanupAfterAttach,
    /// Make the parent non-writable just before the post-attach temporary
    /// cleanup so the real unlink fails with a genuine permission refusal; the
    /// parent synchronization must still run afterwards.
    CleanupDeniedAfterAttach,
    /// Truncate the attachment and make the parent non-writable so the
    /// completeness rollback's remove fails at the real unlink boundary.
    RollbackRemoveDenied,
    /// Truncate the attachment, let the rollback remove succeed, then revoke
    /// the parent so the rollback's own parent sync fails at the real boundary.
    RollbackSyncDenied,
}

/// Publish `bytes` at `path` exactly once. Refuses existing files, symlinks and
/// raced creation; verifies byte completeness after the atomic attach; fails
/// (with the residual uncertainty stated) on cleanup, post-attach or
/// parent-directory synchronization failures.
pub(crate) fn publish(path: &Path, bytes: &[u8]) -> Result<(), String> {
    publish_impl(path, bytes, None)
}

/// Test-only failure injection: forces each named stage to fail so the
/// failure-path reporting and cleanup contracts are observable.
#[cfg(test)]
pub(crate) fn publish_with_fault(
    path: &Path,
    bytes: &[u8],
    fault: Option<PublicationFault>,
) -> Result<(), String> {
    publish_impl(path, bytes, fault)
}

fn publish_impl(
    path: &Path,
    bytes: &[u8],
    #[cfg_attr(not(test), allow(unused_variables))] fault: Option<PublicationFault>,
) -> Result<(), String> {
    if path.symlink_metadata().is_ok() {
        return Err(format!(
            "create-only publication refusal: {} already exists",
            path.display()
        ));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = fresh_temporary(parent, path)?;
    if let Err(error) = stage_payload(&temporary, bytes, fault) {
        return Err(match cleanup(&temporary, fault) {
            Ok(()) => error,
            Err(cleanup_error) => format!("{error}; {cleanup_error}"),
        });
    }
    attach_temporary(&temporary, path, fault)?;
    finalize_attached(parent, path, &temporary, bytes.len(), fault)
}

fn fresh_temporary(parent: &Path, path: &Path) -> Result<std::path::PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "create-only publication refusal: path has no usable file name".to_owned())?;
    let temporary = parent.join(format!(".{name}.{:x}.tmp", std::process::id()));
    if temporary.symlink_metadata().is_ok() {
        return Err(format!(
            "create-only publication refusal: stale temporary {} exists",
            temporary.display()
        ));
    }
    Ok(temporary)
}

#[cfg(test)]
fn fails_write(fault: Option<PublicationFault>) -> bool {
    matches!(
        fault,
        Some(PublicationFault::Write | PublicationFault::CleanupAfterWriteFailure)
    )
}

#[cfg(not(test))]
fn fails_write(_fault: Option<PublicationFault>) -> bool {
    false
}

fn stage_payload(
    temporary: &Path,
    bytes: &[u8],
    #[cfg_attr(not(test), allow(unused_variables))] fault: Option<PublicationFault>,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .map_err(|error| format!("publication stage temporary-create: {error:?}"))?;
    if fails_write(fault) {
        return Err("publication stage write: injected write failure".to_owned());
    }
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("publication stage write-sync: {error:?}"))
}

fn attach_temporary(
    temporary: &Path,
    path: &Path,
    #[cfg_attr(not(test), allow(unused_variables))] fault: Option<PublicationFault>,
) -> Result<(), String> {
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::AttachRace)) {
        std::fs::write(path, b"raced").expect("test injects raced final name");
    }
    if let Err(error) = std::fs::hard_link(temporary, path) {
        cleanup(temporary, fault)?;
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            return Err(format!(
                "create-only publication refusal: {} appeared during publication",
                path.display()
            ));
        }
        return Err(format!("publication stage atomic-attach: {error:?}"));
    }
    #[cfg(test)]
    if matches!(
        fault,
        Some(
            PublicationFault::TruncateAttached
                | PublicationFault::RollbackRemoveDenied
                | PublicationFault::RollbackSyncDenied
        )
    ) {
        std::fs::write(path, b"").expect("test truncates attached file");
    }
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::CleanupAfterAttach)) {
        let _ = std::fs::remove_file(temporary);
    }
    Ok(())
}

fn finalize_attached(
    parent: &Path,
    path: &Path,
    temporary: &Path,
    length: usize,
    #[cfg_attr(not(test), allow(unused_variables))] fault: Option<PublicationFault>,
) -> Result<(), String> {
    let attached = std::fs::metadata(path)
        .ok()
        .is_some_and(|metadata| metadata.len() == length as u64);
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::CleanupDeniedAfterAttach)) {
        revoke_parent_write(parent);
    }
    let cleanup = remove_attached_temporary(temporary);
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::CleanupDeniedAfterAttach)) {
        restore_parent(parent);
    }
    if !attached {
        let mut message = rollback_incomplete(parent, path, fault);
        if let Err(cleanup_error) = cleanup {
            message = format!("{message}; {cleanup_error}");
        }
        return Err(message);
    }
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::DirectorySync)) {
        return Err(faulted_directory_sync(parent, path));
    }
    // Synchronize the parent after the attach on every completed-attach path,
    // including when the temporary cleanup failed: the attached name's entry
    // durability must never be suppressed by an unrelated cleanup refusal.
    let sync = sync_parent(parent, path);
    match cleanup {
        Ok(()) => sync,
        Err(cleanup_error) => match sync {
            Ok(()) => Err(cleanup_error),
            Err(sync_error) => Err(format!("{cleanup_error}; {sync_error}")),
        },
    }
}

fn remove_attached_temporary(temporary: &Path) -> Result<(), String> {
    match std::fs::remove_file(temporary) {
        Ok(()) => Ok(()),
        Err(error) => Err(cleanup_uncertainty(temporary, &error)),
    }
}

/// Truthful post-attach cleanup reporting: the kernel's not-found answer is
/// reported as absent, never as a leftover that "remains"; any other refusal
/// only claims residue once a residue check has actually confirmed it.
fn cleanup_uncertainty(temporary: &Path, error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        return format!(
            "publication stage cleanup: attached-file cleanup of temporary {} failed because \
             the temporary is already absent; nothing of the temporary remains, but the \
             unexpected unlink refusal is reported conservatively ({error:?})",
            temporary.display()
        );
    }
    format!(
        "publication stage cleanup: attached but the temporary link {} could not be removed; \
         residue check: {}; publication uncertainty: retry is blocked until the residue is \
         inspected ({error:?})",
        temporary.display(),
        residue_status(temporary)
    )
}

/// Prove-before-claim: only an existence check that succeeds may say anything
/// remains; a failed check states the uncertainty instead.
pub(crate) fn residue_status(path: &Path) -> String {
    match path.symlink_metadata() {
        Ok(_) => format!("{} remains (existence confirmed)", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            format!("{} is confirmed absent", path.display())
        }
        Err(error) => format!(
            "the presence of {} could not be confirmed ({error:?})",
            path.display()
        ),
    }
}

/// Completeness rollback: remove the incomplete attachment, then synchronize
/// the parent so the removal is durable; report the removal's durability
/// uncertainty when that synchronization fails, and only state that an
/// attachment remains once a residue check has confirmed its existence.
fn rollback_incomplete(
    parent: &Path,
    path: &Path,
    #[cfg_attr(not(test), allow(unused_variables))] fault: Option<PublicationFault>,
) -> String {
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::RollbackRemoveDenied)) {
        revoke_parent_write(parent);
    }
    let removal = std::fs::remove_file(path);
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::RollbackRemoveDenied)) {
        restore_parent(parent);
    }
    match removal {
        Ok(()) => {
            #[cfg(test)]
            if matches!(fault, Some(PublicationFault::RollbackSyncDenied)) {
                deny_parent(parent);
            }
            let sync = std::fs::File::open(parent).and_then(|directory| directory.sync_all());
            #[cfg(test)]
            if matches!(fault, Some(PublicationFault::RollbackSyncDenied)) {
                restore_parent(parent);
            }
            match sync {
                Ok(()) => "publication stage completeness: attached length differs; the attached \
                           file was removed and the parent directory was synchronized"
                    .to_owned(),
                Err(error) => format!(
                    "publication stage completeness: attached length differs; the attached file was \
                     removed; publication uncertainty: the parent {} durability sync of the removal \
                     failed, so the removal may not be durable ({error:?})",
                    parent.display()
                ),
            }
        }
        Err(error) => format!(
            "publication stage completeness: attached length differs; the incomplete attached {} \
             could not be removed; residue check: {}; publication uncertainty: an incomplete \
             attachment must be inspected before any retry ({error:?})",
            path.display(),
            residue_status(path)
        ),
    }
}

fn sync_parent(parent: &Path, path: &Path) -> Result<(), String> {
    let directory = OpenOptions::new()
        .read(true)
        .open(parent)
        .map_err(|error| {
            format!(
                "publication stage directory-sync: parent {} cannot be reopened; publication \
                 uncertainty: attached {} exists but its directory entry may not be durable \
                 ({error:?})",
                parent.display(),
                path.display()
            )
        })?;
    directory.sync_all().map_err(|error| {
        format!(
            "publication stage directory-sync: parent {} durability sync failed; publication \
             uncertainty: attached {} exists but its directory entry may not be durable \
             ({error:?})",
            parent.display(),
            path.display()
        )
    })
}

/// Drive the production open-and-sync through a genuine kernel refusal: revoke
/// the parent's permissions, run the real `sync_parent`, then restore them.
/// The message is always the real production uncertainty message carrying the
/// kernel's own error.
#[cfg(test)]
fn faulted_directory_sync(parent: &Path, path: &Path) -> String {
    deny_parent(parent);
    let outcome = sync_parent(parent, path);
    restore_parent(parent);
    outcome.expect_err(
        "the injected parent-permission fault must make the real directory sync fail; \
         publication must not report success",
    )
}

#[cfg(test)]
fn revoke_parent_write(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;
    // Read+execute stays so existence checks and read-open still succeed; the
    // write-bit revocation makes unlink fail with the genuine EACCES.
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o500))
        .expect("test revokes parent write permission");
}

/// Revoke every parent permission: the read-open of the real post-attach
/// directory sync then fails with the genuine kernel refusal.
#[cfg(test)]
fn deny_parent(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o0))
        .expect("test revokes all parent permissions");
}

#[cfg(test)]
fn restore_parent(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o755));
}

#[cfg_attr(not(test), allow(unused_variables))]
fn cleanup(temporary: &Path, fault: Option<PublicationFault>) -> Result<(), String> {
    #[cfg(test)]
    if matches!(fault, Some(PublicationFault::CleanupAfterWriteFailure)) {
        return Err(format!(
            "publication stage cleanup: injected cleanup failure; publication uncertainty: \
             temporary {} may remain and must be removed before any retry",
            temporary.display()
        ));
    }
    let Err(error) = std::fs::remove_file(temporary) else {
        return Ok(());
    };
    if error.kind() == std::io::ErrorKind::NotFound {
        return Ok(());
    }
    Err(format!(
        "publication stage cleanup: temporary {} could not be removed; residue check: {}; \
         publication uncertainty: retry is blocked until the residue is inspected ({error:?})",
        temporary.display(),
        residue_status(temporary)
    ))
}
