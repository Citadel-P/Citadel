use super::files::*;
use super::postgres::*;
use super::recovery::{DatabaseManifest, RecoveryManifest};
use crate::persistence::postgres::backups::instance_identity::{existing_instance_id, instance_id};
use citadel_backups::{BackupError, CitadelSystemBackupBuilder};
use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest};
use citadel_instance::environment;
use citadel_processes::run;
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
#[derive(Clone)]
pub struct PgDumpSystemBackupBuilder {
    local_budget: citadel_runtime::IoBudget,
    pool: PgPool,
    database_url: String,
    pg_dump: OsString,
    staging_root: PathBuf,
    maximum_output: usize,
    recovery_assets: Option<RecoveryAssets>,
    source_core_version: String,
    expected_instance: Option<Uuid>,
}

/// File-backed keys are archived; explicitly configured keys stay external.
#[derive(Clone)]
pub struct RecoveryAssets {
    pub core_data_path: PathBuf,
    pub jwt_key_is_external: bool,
    pub encryption_key_is_external: bool,
}

impl PgDumpSystemBackupBuilder {
    #[must_use]
    pub fn new(
        pool: PgPool,
        database_url: String,
        pg_dump: impl Into<OsString>,
        staging_root: PathBuf,
        maximum_output: usize,
        source_core_version: String,
    ) -> Self {
        Self {
            local_budget: citadel_runtime::IoBudget::new(
                2.try_into().expect("positive capacity"),
                citadel_runtime::runtime_metrics::RuntimeWork::LocalHeavyWork,
            ),
            pool,
            database_url,
            pg_dump: pg_dump.into(),
            staging_root,
            maximum_output,
            recovery_assets: None,
            source_core_version,
            expected_instance: None,
        }
    }

    /// Checkpoint creation must never initialize or repair the source identity.
    pub(crate) fn for_checkpoint(
        &self,
        staging_root: PathBuf,
        instance: Uuid,
        version: &str,
    ) -> Result<Self, BackupError> {
        if instance.is_nil()
            || self.source_core_version != version
            || self.recovery_assets.is_none()
        {
            return Err(BackupError::Validation(
                "Checkpoint source identity or recovery assets are missing.".into(),
            ));
        }
        let mut builder = self.clone();
        builder.staging_root = staging_root;
        builder.expected_instance = Some(instance);
        Ok(builder)
    }

    pub fn with_local_budget(mut self, budget: citadel_runtime::IoBudget) -> Self {
        self.local_budget = budget;
        self
    }

    pub fn with_recovery_assets(mut self, assets: RecoveryAssets) -> Self {
        self.recovery_assets = Some(assets);
        self
    }

    async fn build_bundle(
        &self,
        run_id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<PathBuf, BackupError> {
        let _local = self.local_budget.enter(cancellation).await.ok_or_else(|| {
            BackupError::External("System backup cancelled while waiting for Core capacity.".into())
        })?;
        ensure_private_root(&self.staging_root)
            .await
            .map_err(|_| BackupError::CleanupPending)?;
        // Other owners may still be writing a bundle. Age is not cleanup proof.
        let directory = self.staging_root.join(run_id.simple().to_string());
        ensure_child(&self.staging_root, &directory)?;
        // Do not replay into an existing attempt's staging directory.
        tokio::fs::create_dir(&directory)
            .await
            .map_err(|_| BackupError::CleanupPending)?;
        let result = async {
            set_private_directory(&directory)?;
            self.write_bundle(&directory, cancellation).await
        }
        .await;
        if let Err(error) = result {
            match tokio::fs::remove_dir_all(&directory).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(BackupError::CleanupPending),
            }
            return Err(error);
        }
        Ok(directory)
    }

    async fn write_bundle(
        &self,
        directory: &Path,
        cancellation: &CancellationToken,
    ) -> Result<(), BackupError> {
        // Ordinary first backups initialize identity before taking the snapshot.
        // Update checkpoints must only observe the already approved identity.
        let instance_id = match self.expected_instance {
            Some(expected) => {
                let read = existing_instance_id(&self.pool);
                let actual = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err(BackupError::External("Checkpoint cancelled.".into())),
                    result = tokio::time::timeout(Duration::from_secs(5), read) => result
                        .map_err(|_| BackupError::External("Checkpoint identity read timed out.".into()))??,
                };
                if actual != Some(expected) {
                    return Err(BackupError::Validation(
                        "Checkpoint database identity does not match the approved instance.".into(),
                    ));
                }
                expected
            }
            None => instance_id(&self.pool).await?,
        };
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
        let (recovery_files, required_external_configuration) = match &self.recovery_assets {
            Some(assets) => copy_recovery_assets(assets, directory, &mut checksums).await?,
            None => (
                vec![],
                vec![
                    environment::CITADEL_JWT_KEY,
                    environment::CITADEL_SECRETS_ENCRYPTION_KEY,
                ],
            ),
        };
        let manifest = RecoveryManifest {
            format_version: 1,
            product: "citadel",
            core_version: &self.source_core_version,
            instance_id,
            created_at: chrono::Utc::now(),
            database: DatabaseManifest {
                engine: "PostgreSQL",
                path: "database/citadel.dump",
                sha256: checksums["database/citadel.dump"].clone(),
            },
            required_external_configuration,
            recovery_files,
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

pub(super) const RECOVERY_FILES: &[&str] = &[
    "recovery/jwtsecret",
    "recovery/secret-encryption-key",
    "recovery/agent/signing-key",
];

pub(super) async fn copy_recovery_assets(
    assets: &RecoveryAssets,
    directory: &Path,
    checksums: &mut BTreeMap<String, String>,
) -> Result<(Vec<String>, Vec<&'static str>), BackupError> {
    let mut files = Vec::new();
    let mut external = Vec::new();
    for (name, setting, configured) in [
        (
            "jwtsecret",
            environment::CITADEL_JWT_KEY,
            assets.jwt_key_is_external,
        ),
        (
            "secret-encryption-key",
            environment::CITADEL_SECRETS_ENCRYPTION_KEY,
            assets.encryption_key_is_external,
        ),
        ("agent/signing-key", "", false),
    ] {
        if configured {
            external.push(setting);
            continue;
        }
        let root = assets.core_data_path.canonicalize().map_err(storage)?;
        let source = checked_bundle_file(&root, name).await?;
        let relative = format!("recovery/{name}");
        let target = directory.join(&relative);
        ensure_private_root(target.parent().expect("recovery subdirectory")).await?;
        let bytes = read_recovery_file(&source, MAX_RECOVERY_KEY_BYTES).await?;
        tokio::fs::write(&target, bytes.as_slice())
            .await
            .map_err(storage)?;
        set_private_file(&target)?;
        checksums.insert(relative.clone(), sha256_file(&target).await?);
        files.push(relative);
    }
    Ok((files, external))
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
