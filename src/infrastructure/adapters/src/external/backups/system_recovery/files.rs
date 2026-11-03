use citadel_backups::BackupError;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fmt::Write,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;
pub(super) const MAX_RECOVERY_METADATA_BYTES: u64 = 64 * 1024;
pub(super) const MAX_RECOVERY_KEY_BYTES: u64 = 16 * 1024;
pub(super) async fn checked_bundle_file(
    root: &Path,
    relative: &str,
) -> Result<PathBuf, BackupError> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(BackupError::Validation(
            "Recovery bundle contains an unsafe path.".into(),
        ));
    }
    let path = root.join(relative);
    let metadata = tokio::fs::symlink_metadata(&path).await.map_err(storage)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(BackupError::Validation(format!(
            "Recovery bundle entry '{}' is not a regular file.",
            relative.display()
        )));
    }
    let canonical = path.canonicalize().map_err(storage)?;
    if !canonical.starts_with(root) {
        return Err(BackupError::Validation(
            "Recovery bundle path escaped its root.".into(),
        ));
    }
    Ok(canonical)
}

pub(super) async fn read_recovery_file(
    path: &Path,
    limit: u64,
) -> Result<Zeroizing<Vec<u8>>, BackupError> {
    // Bound the read itself: a file can grow after a metadata size check. Keys
    // must also be cleared when a partial read or size validation fails.
    let mut bytes = Zeroizing::new(Vec::new());
    tokio::fs::File::open(path)
        .await
        .map_err(storage)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(storage)?;
    if bytes.len() as u64 > limit {
        return Err(BackupError::Validation(format!(
            "Recovery file exceeds the {limit}-byte size limit."
        )));
    }
    Ok(bytes)
}

pub(super) async fn validate_dump(path: &Path) -> Result<(), BackupError> {
    let mut file = tokio::fs::File::open(path).await.map_err(storage)?;
    let mut signature = [0_u8; 5];
    file.read_exact(&mut signature).await.map_err(storage)?;
    if signature != *b"PGDMP" {
        return Err(BackupError::External(
            "pg_dump did not create a PostgreSQL custom-format archive.".into(),
        ));
    }
    Ok(())
}

pub(super) async fn sha256_file(path: &Path) -> Result<String, BackupError> {
    let mut file = tokio::fs::File::open(path).await.map_err(storage)?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut hash = Sha256::new();
    loop {
        let read = file.read(&mut buffer).await.map_err(storage)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    let mut result = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(result)
}

pub(super) async fn write_json(path: &Path, value: &impl Serialize) -> Result<(), BackupError> {
    let bytes = serde_json::to_vec(value).map_err(storage)?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(storage)?;
    file.write_all(&bytes).await.map_err(storage)?;
    file.flush().await.map_err(storage)?;
    drop(file);
    set_private_file(path)
}

pub(super) async fn ensure_private_root(path: &Path) -> Result<(), BackupError> {
    tokio::fs::create_dir_all(path).await.map_err(storage)?;
    let metadata = tokio::fs::symlink_metadata(path).await.map_err(storage)?;
    if metadata.file_type().is_symlink() {
        return Err(BackupError::Validation(
            "Citadel backup staging root cannot be a symbolic link.".into(),
        ));
    }
    set_private_directory(path)
}

pub(super) fn ensure_child(root: &Path, child: &Path) -> Result<(), BackupError> {
    let root = root.canonicalize().map_err(storage)?;
    let parent = child.parent().ok_or_else(|| {
        BackupError::Validation("Citadel recovery staging path is invalid.".into())
    })?;
    let parent = parent.canonicalize().map_err(storage)?;
    if root != parent {
        return Err(BackupError::Validation(
            "Citadel recovery staging path escaped its root.".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn set_private_directory(path: &Path) -> Result<(), BackupError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(storage)
}

#[cfg(not(unix))]
pub(super) fn set_private_directory(_: &Path) -> Result<(), BackupError> {
    Ok(())
}

#[cfg(unix)]
pub(super) fn set_private_file(path: &Path) -> Result<(), BackupError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(storage)
}

#[cfg(not(unix))]
pub(super) fn set_private_file(_: &Path) -> Result<(), BackupError> {
    Ok(())
}

pub(super) fn storage(error: impl std::fmt::Display) -> BackupError {
    BackupError::Storage(error.to_string())
}
