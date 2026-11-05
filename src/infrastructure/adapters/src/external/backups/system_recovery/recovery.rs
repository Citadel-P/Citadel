use super::builder::RECOVERY_FILES;
use super::files::*;
use super::postgres::*;
use citadel_backups::BackupError;
use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest};
use citadel_processes::run;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;
#[derive(Clone)]
pub struct CitadelSystemRestoreOptions {
    pub bundle: PathBuf,
    pub database_url: String,
    pub pg_restore: OsString,
    pub maximum_output: usize,
    // Recovery material is supplied out of band, never included in logs or the
    // pg_restore environment. Presence/shape is checked before target writes.
    pub secret_encryption_key: Option<Zeroizing<Vec<u8>>>,
}

pub async fn restore_citadel_system(
    options: &CitadelSystemRestoreOptions,
    cancellation: &CancellationToken,
) -> Result<(), BackupError> {
    if options
        .secret_encryption_key
        .as_ref()
        .is_none_or(|key| key.len() != 32)
    {
        return Err(BackupError::Validation(
            "Restore requires the original CITADEL_SECRETS_ENCRYPTION_KEY (32 decoded bytes) before modifying PostgreSQL.".into(),
        ));
    }
    let dump = validate_recovery_bundle(&options.bundle).await?;
    let (arguments, environment) = pg_restore_request(&options.database_url, &dump)?;
    let mut request = ProcessRequest::new(options.pg_restore.clone())
        .args(arguments)
        .limits(ProcessLimits {
            timeout: Duration::from_secs(14_400),
            maximum_stdout_bytes: options.maximum_output,
            maximum_stderr_bytes: options.maximum_output,
            output_limit_policy: OutputLimitPolicy::Truncate,
        });
    for (key, value) in environment {
        request = request.env(key, value);
    }
    let output = run(request, cancellation)
        .await
        .map_err(|error| BackupError::External(error.to_string()))?;
    if !output.succeeded() {
        return Err(BackupError::External(format!(
            "pg_restore failed: {}",
            bounded_error(&output.stderr)
        )));
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RecoveryManifest<'a> {
    pub(super) format_version: u32,
    pub(super) product: &'a str,
    pub(super) core_version: &'a str,
    pub(super) instance_id: Uuid,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
    pub(super) database: DatabaseManifest<'a>,
    pub(super) required_external_configuration: Vec<&'a str>,
    #[serde(default)]
    pub(super) recovery_files: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DatabaseManifest<'a> {
    pub(super) engine: &'a str,
    pub(super) path: &'a str,
    pub(super) sha256: String,
}

pub(crate) struct CheckedRecoveryBundle {
    pub dump: PathBuf,
    pub instance_id: Uuid,
    pub core_version: String,
    pub required_external_configuration: Vec<String>,
    pub files: Vec<String>,
    pub checksums_sha256: String,
}

pub(crate) async fn validate_recovery_bundle(root: &Path) -> Result<PathBuf, BackupError> {
    inspect_recovery_bundle(root)
        .await
        .map(|bundle| bundle.dump)
}

pub(crate) async fn inspect_recovery_bundle(
    root: &Path,
) -> Result<CheckedRecoveryBundle, BackupError> {
    let metadata = tokio::fs::symlink_metadata(root).await.map_err(storage)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(BackupError::Validation(
            "Recovery bundle must be a real directory, not a symbolic link.".into(),
        ));
    }
    let root = root.canonicalize().map_err(storage)?;
    let manifest_path = checked_bundle_file(&root, "manifest.json").await?;
    let checksums_path = checked_bundle_file(&root, "checksums.json").await?;
    let manifest_bytes = read_recovery_file(&manifest_path, MAX_RECOVERY_METADATA_BYTES).await?;
    let manifest: RecoveryManifest<'_> =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            BackupError::Validation(format!("Recovery manifest is invalid: {error}"))
        })?;
    if manifest.format_version != 1 || manifest.product != "citadel" {
        return Err(BackupError::Validation(
            "Recovery bundle format or product is unsupported.".into(),
        ));
    }
    if manifest.database.engine != "PostgreSQL" {
        return Err(BackupError::Validation(
            "Recovery bundle database engine is unsupported.".into(),
        ));
    }
    let checksums: BTreeMap<String, String> = serde_json::from_slice(
        &read_recovery_file(&checksums_path, MAX_RECOVERY_METADATA_BYTES).await?,
    )
    .map_err(|error| BackupError::Validation(format!("Recovery checksums are invalid: {error}")))?;
    if manifest
        .recovery_files
        .iter()
        .any(|name| !RECOVERY_FILES.contains(&name.as_str()))
        || manifest
            .recovery_files
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != manifest.recovery_files.len()
        || checksums.len() != 2 + manifest.recovery_files.len()
        || manifest
            .recovery_files
            .iter()
            .any(|name| !checksums.contains_key(name))
        || !checksums.contains_key("manifest.json")
        || !checksums.contains_key(manifest.database.path)
    {
        return Err(BackupError::Validation(
            "Recovery checksum inventory is incomplete or contains unexpected files.".into(),
        ));
    }
    for (relative, expected) in &checksums {
        if expected.len() != 64 || !expected.bytes().all(|value| value.is_ascii_hexdigit()) {
            return Err(BackupError::Validation(format!(
                "Recovery checksum for '{relative}' is invalid."
            )));
        }
        let path = checked_bundle_file(&root, relative).await?;
        let actual = sha256_file(&path).await?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(BackupError::Validation(format!(
                "Recovery checksum for '{relative}' does not match."
            )));
        }
    }
    if !manifest
        .database
        .sha256
        .eq_ignore_ascii_case(&checksums[manifest.database.path])
    {
        return Err(BackupError::Validation(
            "Recovery manifest database checksum does not match its inventory.".into(),
        ));
    }
    let dump = checked_bundle_file(&root, manifest.database.path).await?;
    validate_dump(&dump).await?;
    Ok(CheckedRecoveryBundle {
        dump,
        instance_id: manifest.instance_id,
        core_version: manifest.core_version.into(),
        required_external_configuration: manifest
            .required_external_configuration
            .iter()
            .map(|s| (*s).into())
            .collect(),
        files: checksums
            .into_keys()
            .chain(["checksums.json".into()])
            .collect(),
        checksums_sha256: sha256_file(&checksums_path).await?,
    })
}
