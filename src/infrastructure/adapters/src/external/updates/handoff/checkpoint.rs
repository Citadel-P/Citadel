//! Helper-owned snapshot using the ordinary recovery builder, without Core workers.
use super::super::{
    InstallerComposeInputs, PreparedUpdateCheckpoint, bootstrap::verify_stopped_source,
    docker::LocalDocker, prepare_update_checkpoint, recovery_keys::check_packaged_recovery_keys,
};
use super::{OwnedUpdateHelper, evidence::conflict, runtime::private_home, verification};
use crate::external::backups::system_recovery::{PgDumpSystemBackupBuilder, RecoveryAssets};
use citadel_execution::ProcessRunner;
use citadel_instance::environment;
use citadel_updates::{UpdateError, UpdatePhase};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SourceProcess {
    id: String,
    image: String,
    started_at: String,
    restarts: u64,
}
impl SourceProcess {
    pub(super) fn from_container(core: &Value) -> Result<Self, UpdateError> {
        Ok(Self {
            id: core["Id"].as_str().ok_or_else(conflict)?.into(),
            image: core["Image"].as_str().ok_or_else(conflict)?.into(),
            started_at: core["State"]["StartedAt"]
                .as_str()
                .ok_or_else(conflict)?
                .into(),
            restarts: core["RestartCount"].as_u64().ok_or_else(conflict)?,
        })
    }
}

impl OwnedUpdateHelper {
    /// Call before granting shutdown as well as before the final snapshot. This
    /// verifies this executor can read the recovery assets and reach the registered
    /// PostgreSQL namespace. No normal application services are constructed.
    pub async fn check_checkpoint_access(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<Vec<String>, UpdateError> {
        self.state.verify_lock(&self.owner)?;
        self.intent.verify_saved(&self.state)?;
        let journal = self.state.journal(self.intent.operation.id)?;
        journal.validate_for(&self.intent.operation)?;
        if !matches!(
            journal.operation.phase,
            UpdatePhase::Draining | UpdatePhase::BackingUp
        ) {
            return Err(conflict());
        }
        let source = self.checkpoint_source(process, cancellation).await?;
        let pool = connect(&source.database_url, cancellation).await?;
        let result = async {
            crate::persistence::postgres::updates::check_checkpoint_source(&pool, &journal).await?;
            crate::persistence::postgres::updates::read_update_agent_versions(&pool).await
        }
        .await;
        close(pool).await?;
        result
    }

    /// Await through cancellation. Errors retain maintenance and all evidence;
    /// this is never permission to restart the source or repeat an attempt.
    pub async fn prepare_checkpoint(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<PreparedUpdateCheckpoint, UpdateError> {
        let execution = self.check_grant()?;
        let journal = &execution.journal;
        if journal.operation.phase != UpdatePhase::BackingUp || journal.operation.revision != 2 {
            return Err(conflict());
        }
        self.check_stopped_runtime(process, cancellation).await?;
        let source = self.checkpoint_source(process, cancellation).await?;
        let pool = connect(&source.database_url, cancellation).await?;
        let builder = PgDumpSystemBackupBuilder::new(
            pool.clone(),
            source.database_url,
            "pg_dump",
            self.state.path().into(),
            64 * 1024,
            journal.operation.plan.source.version.clone(),
        )
        .with_recovery_assets(source.assets);
        let result = async {
            crate::persistence::postgres::updates::check_checkpoint_source(&pool, journal).await?;
            if crate::persistence::postgres::updates::read_update_agent_versions(&pool).await?
                != journal.operation.plan.agent_requirements
            {
                return Err(conflict());
            }
            let receipt = prepare_update_checkpoint(
                &self.state,
                &self.owner,
                &journal.operation,
                &builder,
                cancellation,
            )
            .await?;
            self.check_stopped_runtime(process, cancellation).await?;
            Ok(receipt)
        }
        .await;
        drop(builder);
        close(pool).await?;
        result
    }

    pub(super) async fn check_stopped_runtime(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<String, UpdateError> {
        let execution = self.check_grant()?;
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(120),
        };
        let helper =
            verification::acknowledged_helper(&docker, &self.intent, &self.acknowledgement).await?;
        let retained = verify_stopped_source(
            &docker,
            &self.intent.registration,
            &self.acknowledgement.binding.helper_container_id,
        )
        .await?;
        let core =
            verification::container(&docker, &self.intent.registration.core_container_id).await?;
        let stopped = execution.stop.as_ref().ok_or_else(conflict)?;
        if SourceProcess::from_container(&core)? != *stopped
            || core["Image"] != helper.image
            || self.check_grant()? != execution
        {
            return Err(conflict());
        }
        Ok(retained)
    }

    async fn checkpoint_source(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<CheckpointSource, UpdateError> {
        let reg = &self.intent.registration;
        let (checked, config) = InstallerComposeInputs {
            directory: reg.directory.clone(),
            project: reg.project.clone(),
            compose_files: reg.compose_files.clone(),
        }
        .resolve(process, cancellation)
        .await?;
        if checked.configuration_sha256 != reg.configuration_sha256 {
            return Err(conflict());
        }
        let env = &config["services"]["server"]["environment"];
        let external = |name: &str| env[name].as_str().filter(|s| !s.trim().is_empty());
        let root = Path::new("/app/data");
        check_packaged_recovery_keys(
            external(environment::CITADEL_JWT_KEY),
            external(environment::CITADEL_SECRETS_ENCRYPTION_KEY),
        )?;
        // The helper shares only the already-verified PostgreSQL network namespace.
        // Never reuse a host supplied in .env or expose a new database port.
        let mut url = url::Url::parse(if self.intent.operation.plan.database.is_some() {
            "postgres://pg_db:5432/"
        } else {
            "postgres://127.0.0.1:5432/"
        })
        .map_err(|_| conflict())?;
        url.set_username(env[environment::PG_USER].as_str().ok_or_else(conflict)?)
            .map_err(|_| conflict())?;
        url.set_password(Some(
            env[environment::PG_PASSWORD]
                .as_str()
                .ok_or_else(conflict)?,
        ))
        .map_err(|_| conflict())?;
        url.set_path(
            env[environment::PG_DATABASE]
                .as_str()
                .ok_or_else(conflict)?,
        );
        Ok(CheckpointSource {
            database_url: url.to_string(),
            assets: RecoveryAssets {
                core_data_path: root.into(),
                jwt_key_is_external: external(environment::CITADEL_JWT_KEY).is_some(),
                encryption_key_is_external: external(environment::CITADEL_SECRETS_ENCRYPTION_KEY)
                    .is_some(),
            },
        })
    }
}

struct CheckpointSource {
    database_url: String,
    assets: RecoveryAssets,
}

async fn connect(url: &str, cancellation: &CancellationToken) -> Result<sqlx::PgPool, UpdateError> {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(UpdateError::Cancelled),
        result = crate::persistence::postgres::connection::connect_pool(url, 2, Duration::from_secs(5)) => result.map_err(|_| conflict()),
    }
}
async fn close(pool: sqlx::PgPool) -> Result<(), UpdateError> {
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .map_err(|_| conflict())
}
