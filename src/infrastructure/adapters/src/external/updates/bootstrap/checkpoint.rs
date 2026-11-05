//! Storage admission only. No backup is created or declared recoverable here.
use super::super::JournalDirectory;
use citadel_updates::UpdateError;
use rustix::fs::{AtFlags, Mode, OFlags, openat, unlinkat};
use std::{fs::File, io::Write, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointStorage {
    pub directory: PathBuf,
    pub available_bytes: u64,
    pub minimum_bytes: u64,
}

pub(super) fn minimum_bytes(database_bytes: u64) -> Result<u64, UpdateError> {
    // Conservative admission budget, not a bound on the eventual dump. Creation
    // must still handle growth, quotas and ENOSPC before any replacement.
    database_bytes
        .checked_mul(2)
        .and_then(|v| v.checked_add(64 * 1024 * 1024))
        .filter(|_| database_bytes > 0)
        .ok_or_else(|| failed("Invalid database size for checkpoint storage assessment."))
}

pub(super) fn check_storage(
    state: &JournalDirectory,
    database_bytes: u64,
) -> Result<CheckpointStorage, UpdateError> {
    state.verify_location()?;
    let directory = state.directory();
    let stat = rustix::fs::fstatvfs(directory)
        .map_err(|_| failed("Cannot inspect checkpoint filesystem capacity."))?;
    let available_bytes = stat
        .f_bavail
        .checked_mul(stat.f_frsize)
        .ok_or_else(|| failed("Invalid checkpoint filesystem capacity."))?;
    let minimum_bytes = minimum_bytes(database_bytes)?;
    if available_bytes < minimum_bytes {
        return Err(failed(
            "Insufficient checkpoint storage: reserve twice the current database size plus 64 MiB before updating.",
        ));
    }
    // Anchor all writes/removals to the protected directory descriptor. A crash
    // leaving this probe is evidence; startup must not silently discard it.
    let name = format!(".checkpoint-probe-{}", Uuid::now_v7());
    let mut file = File::from(
        openat(
            directory,
            name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| failed("Checkpoint storage is not writable by the registered executor."))?,
    );
    let written = (|| -> std::io::Result<()> {
        file.write_all(&[0; 4096])?;
        file.sync_all()?;
        directory.sync_all()
    })();
    drop(file);
    unlinkat(directory, name.as_str(), AtFlags::empty()).map_err(|_| failed(&format!("Cannot remove checkpoint storage probe {name}; preserve update evidence and inspect the registered directory.")))?;
    directory
        .sync_all()
        .map_err(|_| failed("Cannot synchronize checkpoint storage after probe cleanup."))?;
    written.map_err(|_| failed("Checkpoint storage failed its write/synchronization probe."))?;
    state.verify_location()?;
    Ok(CheckpointStorage {
        directory: state.path().into(),
        available_bytes,
        minimum_bytes,
    })
}

fn failed(message: &str) -> UpdateError {
    UpdateError::Unavailable(message.into())
}
