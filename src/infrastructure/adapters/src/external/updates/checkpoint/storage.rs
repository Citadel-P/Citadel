use super::super::{JournalDirectory, files::open_secure};
use citadel_updates::UpdateError;
use rustix::fs::{Mode, OFlags, mkdirat, openat};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Write, path::Path};
use tokio_util::sync::CancellationToken;

pub(super) fn failed() -> UpdateError {
    UpdateError::Unavailable("Checkpoint preparation failed. Preserve update state for inspection; no replacement was authorized.".into())
}

pub(super) fn cancelled(token: &CancellationToken) -> Result<(), UpdateError> {
    if token.is_cancelled() {
        Err(UpdateError::Cancelled)
    } else {
        Ok(())
    }
}

pub(super) fn create_attempt(
    state: &JournalDirectory,
    name: &str,
) -> Result<JournalDirectory, UpdateError> {
    // Never reuse a partial attempt, follow a symlink or erase crash evidence.
    mkdirat(state.directory(), name, Mode::from_raw_mode(0o700)).map_err(|_| failed())?;
    state.sync()?;
    JournalDirectory::open(state.path().join(name))
}

pub(super) fn write_input(
    stage: &JournalDirectory,
    name: &str,
    bytes: &[u8],
) -> Result<String, UpdateError> {
    let file = File::from(
        openat(
            stage.directory(),
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| failed())?,
    );
    sync_input(file, bytes)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn sync_input(mut file: File, bytes: &[u8]) -> Result<(), UpdateError> {
    file.write_all(bytes).map_err(|_| failed())?;
    file.sync_all().map_err(|_| failed())
}

pub(super) async fn sync_bundle(
    root: &Path,
    files: &[String],
    token: &CancellationToken,
) -> Result<(), UpdateError> {
    let mut directories = std::collections::BTreeSet::new();
    for relative in files {
        cancelled(token)?;
        let path = root.join(relative);
        // Descriptor traversal refuses symlink ancestors, FIFOs and unsafe owners.
        let file = open_secure(&path, false)?;
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata().map_err(|_| failed())?;
        if meta.nlink() != 1 || meta.mode() & 0o077 != 0 {
            return Err(failed());
        }
        tokio::fs::File::from_std(file)
            .sync_all()
            .await
            .map_err(|_| failed())?;
        let mut parent = path.parent();
        while let Some(path) = parent {
            if !path.starts_with(root) {
                break;
            }
            directories.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    // Children precede their parents; then the enclosing attempt is synced by
    // receipt publication. A receipt never precedes its referenced file data.
    for path in directories.iter().rev() {
        cancelled(token)?;
        tokio::fs::File::from_std(open_secure(path, true)?)
            .sync_all()
            .await
            .map_err(|_| failed())?;
    }
    Ok(())
}
