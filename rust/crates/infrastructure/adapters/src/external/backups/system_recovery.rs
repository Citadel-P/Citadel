use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use citadel_backups::{BackupError, CitadelSystemBackupBuilder};
use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest};
use citadel_processes::run;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use url::Url;
use uuid::Uuid;
use zeroize::Zeroizing;

type ProcessEnvironment = Vec<(OsString, OsString)>;
type PgDumpRequest = (Vec<OsString>, ProcessEnvironment);

#[derive(Clone)]
pub struct PgDumpSystemBackupBuilder {
    pool: PgPool,
    database_url: String,
    pg_dump: OsString,
    staging_root: PathBuf,
    maximum_output: usize,
}

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
            "Restore requires the original Secrets__EncryptionKey (32 decoded bytes) before modifying PostgreSQL.".into(),
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

impl PgDumpSystemBackupBuilder {
    #[must_use]
    pub fn new(
        pool: PgPool,
        database_url: String,
        pg_dump: impl Into<OsString>,
        staging_root: PathBuf,
        maximum_output: usize,
    ) -> Self {
        Self {
            pool,
            database_url,
            pg_dump: pg_dump.into(),
            staging_root,
            maximum_output,
        }
    }

    async fn build_bundle(
        &self,
        run_id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<PathBuf, BackupError> {
        ensure_private_root(&self.staging_root).await?;
        cleanup_stale_staging(&self.staging_root).await?;
        let directory = self.staging_root.join(run_id.simple().to_string());
        ensure_child(&self.staging_root, &directory)?;
        if tokio::fs::try_exists(&directory).await.map_err(storage)? {
            tokio::fs::remove_dir_all(&directory)
                .await
                .map_err(storage)?;
        }
        tokio::fs::create_dir(&directory).await.map_err(storage)?;
        set_private_directory(&directory)?;
        let result = self.write_bundle(&directory, cancellation).await;
        if let Err(error) = result {
            let _ = tokio::fs::remove_dir_all(&directory).await;
            return Err(error);
        }
        Ok(directory)
    }

    async fn write_bundle(
        &self,
        directory: &Path,
        cancellation: &CancellationToken,
    ) -> Result<(), BackupError> {
        // Commit a lazily-created identity before pg_dump takes its snapshot.
        // Otherwise a first backup advertises an identity absent from its DB.
        let instance_id = instance_id(&self.pool).await?;
        let database_dir = directory.join("database");
        tokio::fs::create_dir(&database_dir)
            .await
            .map_err(storage)?;
        set_private_directory(&database_dir)?;
        let dump = database_dir.join("citadel.dump");
        let (arguments, environment) = pg_dump_request(&self.database_url, &dump)?;
        let mut request = ProcessRequest::new(self.pg_dump.clone())
            .args(arguments)
            .current_dir(directory)
            .limits(ProcessLimits {
                timeout: Duration::from_secs(14_400),
                maximum_stdout_bytes: self.maximum_output,
                maximum_stderr_bytes: self.maximum_output,
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
                "pg_dump failed: {}",
                bounded_error(&output.stderr)
            )));
        }
        validate_dump(&dump).await?;
        set_private_file(&dump)?;

        let mut checksums = BTreeMap::new();
        checksums.insert(
            "database/citadel.dump".to_owned(),
            sha256_file(&dump).await?,
        );
        let manifest = RecoveryManifest {
            format_version: 1,
            product: "citadel",
            core_version: env!("CARGO_PKG_VERSION"),
            instance_id,
            created_at: chrono::Utc::now(),
            database: DatabaseManifest {
                engine: "PostgreSQL",
                path: "database/citadel.dump",
                sha256: checksums["database/citadel.dump"].clone(),
            },
            required_external_configuration: vec!["Jwt__Key", "Secrets__EncryptionKey"],
        };
        write_json(&directory.join("manifest.json"), &manifest).await?;
        checksums.insert(
            "manifest.json".to_owned(),
            sha256_file(&directory.join("manifest.json")).await?,
        );
        write_json(&directory.join("checksums.json"), &checksums).await?;
        Ok(())
    }
}

async fn cleanup_stale_staging(root: &Path) -> Result<(), BackupError> {
    const MAX_ENTRIES: usize = 100;
    const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
    let mut entries = tokio::fs::read_dir(root).await.map_err(storage)?;
    let now = std::time::SystemTime::now();
    let mut removed = 0;
    loop {
        if removed == MAX_ENTRIES {
            break;
        }
        let Some(entry) = entries.next_entry().await.map_err(storage)? else {
            break;
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.len() != 32 || !name.bytes().all(|value| value.is_ascii_hexdigit()) {
            continue;
        }
        let metadata = tokio::fs::symlink_metadata(entry.path())
            .await
            .map_err(storage)?;
        if !metadata.is_dir()
            || metadata
                .modified()
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_none_or(|age| age < MAX_AGE)
        {
            continue;
        }
        let path = entry.path();
        ensure_child(root, &path)?;
        tokio::fs::remove_dir_all(path).await.map_err(storage)?;
        removed += 1;
    }
    Ok(())
}

impl CitadelSystemBackupBuilder for PgDumpSystemBackupBuilder {
    fn build<'a>(
        &'a self,
        run_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PathBuf, BackupError>> {
        Box::pin(self.build_bundle(run_id, cancellation))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryManifest<'a> {
    format_version: u32,
    product: &'a str,
    core_version: &'a str,
    instance_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    database: DatabaseManifest<'a>,
    required_external_configuration: Vec<&'a str>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseManifest<'a> {
    engine: &'a str,
    path: &'a str,
    sha256: String,
}

async fn instance_id(pool: &PgPool) -> Result<Uuid, BackupError> {
    let mut transaction = pool.begin().await.map_err(storage)?;
    let generated = Uuid::now_v7();
    sqlx::query("INSERT INTO citadelinstanceidentity(id,instanceid,createdat) VALUES(1,$1,CURRENT_TIMESTAMP) ON CONFLICT(id) DO NOTHING")
        .bind(generated)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
    let id = sqlx::query_scalar("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage)?;
    transaction.commit().await.map_err(storage)?;
    Ok(id)
}

fn pg_dump_request(database_url: &str, destination: &Path) -> Result<PgDumpRequest, BackupError> {
    let environment = postgres_environment(database_url)?;
    Ok((
        vec![
            "--format=custom".into(),
            "--no-owner".into(),
            "--no-privileges".into(),
            "--file".into(),
            destination.as_os_str().to_owned(),
        ],
        environment,
    ))
}

fn pg_restore_request(database_url: &str, source: &Path) -> Result<PgDumpRequest, BackupError> {
    Ok((
        vec![
            "--format=custom".into(),
            "--clean".into(),
            "--if-exists".into(),
            "--no-owner".into(),
            "--no-privileges".into(),
            "--single-transaction".into(),
            "--exit-on-error".into(),
            "--dbname".into(),
            // pg_restore interprets a --dbname value containing '=' as a libpq
            // connection string. Quote the database as a single conninfo value
            // so unusual names cannot override the host/user from our environment.
            format!(
                "dbname='{}'",
                postgres_database_name(database_url)?
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
            )
            .into(),
            source.as_os_str().to_owned(),
        ],
        postgres_environment(database_url)?,
    ))
}

fn postgres_database_name(database_url: &str) -> Result<String, BackupError> {
    let url = postgres_url(database_url)?;
    decode_postgres_component(url.path().trim_start_matches('/'))
}

fn decode_postgres_component(value: &str) -> Result<String, BackupError> {
    let decoded = urlencoding::decode(value)
        .map_err(|_| BackupError::Validation("PostgreSQL URL contains invalid UTF-8.".into()))?;
    if decoded.contains('\0') {
        return Err(BackupError::Validation(
            "PostgreSQL URL contains a null byte.".into(),
        ));
    }
    Ok(decoded.into_owned())
}

fn postgres_environment(database_url: &str) -> Result<ProcessEnvironment, BackupError> {
    let url = postgres_url(database_url)?;
    let database = decode_postgres_component(url.path().trim_start_matches('/'))?;
    let mut environment = vec![
        ("PGHOST".into(), url.host_str().unwrap_or_default().into()),
        (
            "PGUSER".into(),
            decode_postgres_component(url.username())?.into(),
        ),
        ("PGDATABASE".into(), database.into()),
    ];
    if let Some(port) = url.port() {
        environment.push(("PGPORT".into(), port.to_string().into()));
    }
    if let Some(password) = url.password() {
        environment.push((
            "PGPASSWORD".into(),
            decode_postgres_component(password)?.into(),
        ));
    }
    if let Some((_, ssl_mode)) = url.query_pairs().find(|(key, _)| key == "sslmode") {
        environment.push(("PGSSLMODE".into(), ssl_mode.as_ref().into()));
    }
    Ok(environment)
}

fn postgres_url(database_url: &str) -> Result<Url, BackupError> {
    let url = Url::parse(database_url)
        .map_err(|_| BackupError::Validation("PostgreSQL URL is invalid.".into()))?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err(BackupError::Validation(
            "PostgreSQL URL must use the postgres scheme.".into(),
        ));
    }
    if url.host_str().is_none()
        || url.username().is_empty()
        || url.path().trim_start_matches('/').is_empty()
    {
        return Err(BackupError::Validation(
            "PostgreSQL URL must include host, user, and database.".into(),
        ));
    }
    Ok(url)
}

async fn validate_recovery_bundle(root: &Path) -> Result<PathBuf, BackupError> {
    let metadata = tokio::fs::symlink_metadata(root).await.map_err(storage)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(BackupError::Validation(
            "Recovery bundle must be a real directory, not a symbolic link.".into(),
        ));
    }
    let root = root.canonicalize().map_err(storage)?;
    let manifest_path = checked_bundle_file(&root, "manifest.json").await?;
    let checksums_path = checked_bundle_file(&root, "checksums.json").await?;
    let manifest_bytes = tokio::fs::read(&manifest_path).await.map_err(storage)?;
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
        &tokio::fs::read(&checksums_path).await.map_err(storage)?,
    )
    .map_err(|error| BackupError::Validation(format!("Recovery checksums are invalid: {error}")))?;
    if checksums.len() != 2
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
    Ok(dump)
}

async fn checked_bundle_file(root: &Path, relative: &str) -> Result<PathBuf, BackupError> {
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

async fn validate_dump(path: &Path) -> Result<(), BackupError> {
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

async fn sha256_file(path: &Path) -> Result<String, BackupError> {
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

async fn write_json(path: &Path, value: &impl Serialize) -> Result<(), BackupError> {
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

async fn ensure_private_root(path: &Path) -> Result<(), BackupError> {
    tokio::fs::create_dir_all(path).await.map_err(storage)?;
    let metadata = tokio::fs::symlink_metadata(path).await.map_err(storage)?;
    if metadata.file_type().is_symlink() {
        return Err(BackupError::Validation(
            "Citadel backup staging root cannot be a symbolic link.".into(),
        ));
    }
    set_private_directory(path)
}

fn ensure_child(root: &Path, child: &Path) -> Result<(), BackupError> {
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
fn set_private_directory(path: &Path) -> Result<(), BackupError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(storage)
}

#[cfg(not(unix))]
fn set_private_directory(_: &Path) -> Result<(), BackupError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), BackupError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(storage)
}

#[cfg(not(unix))]
fn set_private_file(_: &Path) -> Result<(), BackupError> {
    Ok(())
}

fn bounded_error(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).chars().take(4096).collect()
}

fn storage(error: impl std::fmt::Display) -> BackupError {
    BackupError::Storage(error.to_string())
}

#[cfg(test)]
mod parity_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_credentials_are_passed_in_environment_not_arguments() {
        let (arguments, environment) = pg_dump_request(
            "postgres://citadel:secret@database:5432/citadel?sslmode=require",
            Path::new("dump"),
        )
        .unwrap();
        assert!(
            !arguments
                .iter()
                .any(|value| value.to_string_lossy().contains("secret"))
        );
        assert!(
            environment
                .iter()
                .any(|(key, value)| { key == "PGPASSWORD" && value == "secret" })
        );
        assert!(
            environment
                .iter()
                .any(|(key, value)| { key == "PGSSLMODE" && value == "require" })
        );
    }

    #[test]
    fn restore_is_transactional_and_credentials_are_not_arguments() {
        let (arguments, environment) = pg_restore_request(
            "postgres://citadel:secret@database:5432/citadel",
            Path::new("citadel.dump"),
        )
        .unwrap();
        assert!(arguments.contains(&"--single-transaction".into()));
        assert!(arguments.contains(&"--exit-on-error".into()));
        assert!(!arguments.iter().any(|value| value == "secret"));
        assert!(
            environment
                .iter()
                .any(|(key, value)| key == "PGPASSWORD" && value == "secret")
        );
    }

    #[tokio::test]
    async fn recovery_bundle_rejects_parent_path() {
        let root = std::env::temp_dir().join(format!("citadel-recovery-{}", Uuid::now_v7()));
        tokio::fs::create_dir(&root).await.unwrap();
        let error = checked_bundle_file(&root, "../citadel.dump")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("unsafe path"));
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn recovery_bundle_detects_database_tampering_before_restore() {
        let root = std::env::temp_dir().join(format!("citadel-recovery-{}", Uuid::now_v7()));
        let database = root.join("database");
        tokio::fs::create_dir_all(&database).await.unwrap();
        let dump = database.join("citadel.dump");
        tokio::fs::write(&dump, b"PGDMPfixture").await.unwrap();
        let dump_hash = sha256_file(&dump).await.unwrap();
        let manifest = RecoveryManifest {
            format_version: 1,
            product: "citadel",
            core_version: "test",
            instance_id: Uuid::now_v7(),
            created_at: chrono::Utc::now(),
            database: DatabaseManifest {
                engine: "PostgreSQL",
                path: "database/citadel.dump",
                sha256: dump_hash.clone(),
            },
            required_external_configuration: vec!["Jwt__Key", "Secrets__EncryptionKey"],
        };
        write_json(&root.join("manifest.json"), &manifest)
            .await
            .unwrap();
        let checksums = BTreeMap::from([
            ("database/citadel.dump".to_owned(), dump_hash),
            (
                "manifest.json".to_owned(),
                sha256_file(&root.join("manifest.json")).await.unwrap(),
            ),
        ]);
        write_json(&root.join("checksums.json"), &checksums)
            .await
            .unwrap();
        assert_eq!(
            validate_recovery_bundle(&root).await.unwrap(),
            dump.canonicalize().unwrap()
        );

        tokio::fs::write(&dump, b"PGDMPtampered").await.unwrap();
        let error = validate_recovery_bundle(&root).await.unwrap_err();
        assert!(error.to_string().contains("does not match"));
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
