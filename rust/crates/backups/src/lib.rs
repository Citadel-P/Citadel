#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryInput {
    pub name: String,
    pub description: Option<String>,
    pub spec: Value,
    pub password_secret_id: Uuid,
}
impl BackupRepositoryInput {
    pub fn validate(&mut self) -> Result<(), BackupError> {
        validate_name(&mut self.name, "Backup Repository")?;
        if self.password_secret_id.is_nil() {
            return Err(BackupError::Validation(
                "Backup Repository password Secret is required.".into(),
            ));
        }
        match discriminator(&self.spec)? {
            "FileSystem" => {
                let location = required_string(&self.spec, "location")?;
                let path = required_string(&self.spec, "path")?;
                let platform = self
                    .spec
                    .get("platformId")
                    .and_then(Value::as_str)
                    .and_then(|v| Uuid::parse_str(v).ok());
                if path.trim().is_empty()
                    || !matches!(location, "Core" | "Platform")
                    || (location == "Core" && platform.is_some())
                    || (location == "Platform" && platform.is_none())
                {
                    return Err(BackupError::Validation(
                        "Filesystem Backup Repository location is invalid.".into(),
                    ));
                }
            }
            "S3Compatible" => {
                let endpoint = required_string(&self.spec, "endpoint")?;
                let url = reqwest_url(endpoint)?;
                let insecure = self
                    .spec
                    .get("allowInsecureHttp")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if url.scheme() == "http" && !insecure {
                    return Err(BackupError::Validation(
                        "S3 endpoint must use HTTPS unless insecure HTTP is explicitly allowed."
                            .into(),
                    ));
                }
                if required_string(&self.spec, "bucket")?.trim().is_empty() {
                    return Err(BackupError::Validation("S3 bucket is required.".into()));
                }
                for key in ["accessKeySecretId", "secretKeySecretId"] {
                    required_uuid(&self.spec, key)?;
                }
            }
            _ => {
                return Err(BackupError::Validation(
                    "Backup Repository type is unsupported.".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub repository_type: String,
    pub spec: Value,
    pub password_secret_id: Uuid,
    pub status: String,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub control_started_at: Option<i64>,
    pub last_pruned_at: Option<DateTime<Utc>>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicyInput {
    pub name: String,
    pub description: Option<String>,
    pub source: Value,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook: Option<Value>,
    pub keep_last_successful: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}
impl BackupPolicyInput {
    pub fn validate(&mut self, actor: ActorId) -> Result<(), BackupError> {
        validate_name(&mut self.name, "Backup Policy")?;
        if self.backup_repository_id.is_nil() {
            return Err(BackupError::Validation(
                "Backup Repository is required.".into(),
            ));
        }
        match discriminator(&self.source)? {
            "DockerVolume" => {
                required_uuid(&self.source, "platformId")?;
                required_string(&self.source, "volumeName")?;
            }
            "CitadelSystem" => {}
            "Stack" => {
                required_uuid(&self.source, "stackId")?;
            }
            "Deployment" => {
                required_uuid(&self.source, "deploymentId")?;
            }
            "SwarmService" => {
                required_uuid(&self.source, "swarmServiceId")?;
            }
            _ => {
                return Err(BackupError::Validation(
                    "Backup source type is unsupported.".into(),
                ));
            }
        };
        let keep = self.keep_last_successful.unwrap_or(14);
        if !(1..=1000).contains(&keep) {
            return Err(BackupError::Validation(
                "Backup retention must be between 1 and 1000 snapshots.".into(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(14400);
        if !(60..=86400).contains(&timeout) {
            return Err(BackupError::Validation(
                "Backup timeout must be between 60 and 86400 seconds.".into(),
            ));
        }
        self.keep_last_successful = Some(keep);
        self.timeout_seconds = Some(timeout);
        if self
            .cron
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            let zone = self.time_zone.as_deref().unwrap_or("UTC");
            if !valid_cron(self.cron.as_deref().unwrap_or_default()) || zone.parse::<Tz>().is_err()
            {
                return Err(BackupError::Validation(
                    "Backup schedule requires a valid five-field cron expression and time zone."
                        .into(),
                ));
            }
            self.time_zone = Some(zone.to_owned());
        } else {
            self.cron = None;
            self.time_zone = Some(self.time_zone.take().unwrap_or_else(|| "UTC".to_owned()));
        }
        self.run_as_actor_id = Some(self.run_as_actor_id.unwrap_or(actor.value()));
        if self.tag_ids.len() > 100 {
            return Err(BackupError::Validation(
                "A Backup Policy cannot have more than 100 Tags.".to_owned(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicyView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub source: Value,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook: Option<Value>,
    pub keep_last_successful: i32,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub first_successful_run_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunView {
    pub id: Uuid,
    pub backup_policy_id: Uuid,
    pub policy_name_snapshot: String,
    pub backup_repository_id: Uuid,
    pub repository_type_snapshot: String,
    pub source_snapshot: Value,
    pub trigger: String,
    pub status: String,
    pub snapshot_availability: String,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub warnings: Vec<String>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
    pub items: Vec<BackupRunItemView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunItemView {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub platform_id: Uuid,
    pub volume_name: String,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub status: String,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreRunView {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub backup_repository_id: Uuid,
    pub source_backup_run_item_id: Option<Uuid>,
    pub target_platform_id: Uuid,
    pub target_docker_node_id: Option<String>,
    pub target_volume_name: String,
    pub overwrite_existing: bool,
    pub status: String,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct BackupRestoreRequest {
    pub actor: ActorId,
    pub backup_run_id: Uuid,
    pub target_platform_id: Uuid,
    pub target_volume_name: String,
    pub overwrite_existing: bool,
    pub target_docker_node_id: Option<String>,
    pub source_backup_run_item_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupLog {
    pub stream: String,
    pub message: String,
}
#[derive(Debug, Clone)]
pub struct BackupClaim {
    pub policy: BackupPolicyView,
    pub repository: BackupRepositoryView,
    pub run: BackupRunView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSourceItem {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub volume_name: String,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
}

impl BackupSourceItem {
    #[must_use]
    pub fn new(
        platform_id: Uuid,
        volume_name: String,
        docker_node_id: Option<String>,
        node_hostname: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            platform_id,
            volume_name,
            docker_node_id,
            node_hostname,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSourcePlan {
    pub display_name: String,
    pub items: Vec<BackupSourceItem>,
    pub warnings: Vec<String>,
    pub local_directory: Option<PathBuf>,
}

pub trait CitadelSystemBackupBuilder: Send + Sync {
    fn build<'a>(
        &'a self,
        run_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PathBuf, BackupError>>;
}

#[derive(Debug, Clone)]
pub struct BackupRunItemResult {
    pub id: Uuid,
    pub status: &'static str,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}
#[derive(Debug, Clone)]
pub struct RestoreClaim {
    pub repository: BackupRepositoryView,
    pub run: BackupRestoreRunView,
    pub source: BackupRunView,
    pub source_item: Option<BackupRunItemView>,
}
#[derive(Debug, Clone)]
pub struct BackupExecutionResult {
    pub status: &'static str,
    pub snapshot_availability: &'static str,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
    pub logs: Vec<BackupLog>,
    pub items: Vec<BackupRunItemResult>,
}
#[derive(Debug, Clone)]
pub struct RestoreExecutionResult {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub logs: Vec<BackupLog>,
}

#[derive(Debug, Clone)]
pub struct BackupRepositoryOperationResult {
    pub repository: BackupRepositoryView,
    pub validation: BackupRepositoryValidationView,
    pub succeeded: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryValidationView {
    pub id: Uuid,
    pub backup_repository_id: Uuid,
    pub location: String,
    pub platform_id: Option<Uuid>,
    pub status: String,
    pub last_validated_at: DateTime<Utc>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

pub trait BackupStore: Send + Sync {
    fn create_repository<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupRepositoryInput,
    ) -> BoxFuture<'a, Result<BackupRepositoryView, BackupError>>;
    fn list_repositories(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupRepositoryView>, BackupError>>;
    fn get_repository(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRepositoryView, BackupError>>;
    fn archive_repository(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>>;
    fn record_repository_operation<'a>(
        &'a self,
        id: Uuid,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        succeeded: bool,
        message: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(BackupRepositoryView, BackupRepositoryValidationView), BackupError>>;
    fn acquire_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
        operation: &str,
        expires_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>>;
    fn release_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>>;
    fn create_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupPolicyInput,
    ) -> BoxFuture<'a, Result<BackupPolicyView, BackupError>>;
    fn list_policies(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicyView>, BackupError>>;
    fn get_policy(&self, id: Uuid) -> BoxFuture<'_, Result<BackupPolicyView, BackupError>>;
    fn archive_policy(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>>;
    fn enqueue_backup(
        &self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
    ) -> BoxFuture<'_, Result<BackupRunView, BackupError>>;
    fn list_scheduled_policies(&self) -> BoxFuture<'_, Result<Vec<BackupPolicyView>, BackupError>>;
    fn enqueue_scheduled_backup(
        &self,
        policy_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>>;
    fn claim_backup(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<BackupClaim>, BackupError>>;
    fn prepare_backup_items<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn finish_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        result: &'a BackupExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn list_runs(
        &self,
        actor: ActorId,
        administrator: bool,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRunView>, BackupError>>;
    fn get_run(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRunView, BackupError>>;
    fn backup_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>>;
    fn cancel_backup(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>>;
    fn enqueue_restore(
        &self,
        request: BackupRestoreRequest,
    ) -> BoxFuture<'_, Result<BackupRestoreRunView, BackupError>>;
    fn claim_restore(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<RestoreClaim>, BackupError>>;
    fn finish_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        result: &'a RestoreExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn list_restores(
        &self,
        actor: ActorId,
        administrator: bool,
        backup_run_id: Option<Uuid>,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRestoreRunView>, BackupError>>;
    fn get_restore(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRestoreRunView, BackupError>>;
    fn restore_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>>;
    fn cancel_restore(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>>;
}
pub trait BackupExecutor: Send + Sync {
    fn repository<'a>(
        &'a self,
        repository: &'a BackupRepositoryView,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>>;
    fn backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult>;
    fn restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, RestoreExecutionResult>;
}

pub trait BackupSourcePlanner: Send + Sync {
    fn plan<'a>(
        &'a self,
        claim: &'a BackupClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BackupSourcePlan, BackupError>>;
}
pub trait BackupSecretResolver: Send + Sync {
    fn resolve(&self, id: Uuid) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BackupError>>;
}

pub trait BackupRunAuthorizer: Send + Sync {
    fn authorize_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn authorize_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
}

pub struct BackupService {
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    store: Arc<dyn BackupStore>,
    executor: Arc<dyn BackupExecutor>,
    planner: Arc<dyn BackupSourcePlanner>,
    active_backups: Mutex<HashMap<Uuid, CancellationToken>>,
    active_restores: Mutex<HashMap<Uuid, CancellationToken>>,
    stale_after: chrono::Duration,
    authorizer: Arc<dyn BackupRunAuthorizer>,
}
impl BackupService {
    pub fn with_change_notifier(mut self, notifier: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.on_change = Some(notifier);
        self
    }

    fn changed(&self) {
        if let Some(notifier) = &self.on_change {
            notifier();
        }
    }

    pub fn new(
        store: Arc<dyn BackupStore>,
        executor: Arc<dyn BackupExecutor>,
        planner: Arc<dyn BackupSourcePlanner>,
        stale_after: chrono::Duration,
        authorizer: Arc<dyn BackupRunAuthorizer>,
    ) -> Self {
        Self {
            store,
            on_change: None,
            executor,
            planner,
            active_backups: Mutex::new(HashMap::new()),
            active_restores: Mutex::new(HashMap::new()),
            stale_after,
            authorizer,
        }
    }
    pub fn store(&self) -> &Arc<dyn BackupStore> {
        &self.store
    }
    pub async fn repository_operation(
        &self,
        id: Uuid,
        operation: &str,
        location: &str,
        platform_id: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<BackupRepositoryOperationResult, BackupError> {
        if !matches!(operation, "Validate" | "Initialize" | "Check" | "Prune") {
            return Err(BackupError::Validation(
                "Backup Repository operation is invalid.".into(),
            ));
        }
        let repository = self.store.get_repository(id).await?;
        let operation_id = Uuid::now_v7();
        if !self
            .store
            .acquire_repository_operation(
                id,
                operation_id,
                operation,
                Utc::now() + chrono::Duration::minutes(16),
            )
            .await?
        {
            return Err(BackupError::Conflict(
                "Backup Repository is already in use.".into(),
            ));
        }
        let result = self
            .executor
            .repository(&repository, operation, location, platform_id, cancellation)
            .await;
        let error_message = result.as_ref().err().map(ToString::to_string);
        let recorded = self
            .store
            .record_repository_operation(
                id,
                operation,
                location,
                platform_id,
                result.is_ok(),
                error_message.as_deref(),
            )
            .await;
        let released = self
            .store
            .release_repository_operation(id, operation_id)
            .await;
        let (repository, validation) = recorded?;
        released?;
        self.changed();
        Ok(BackupRepositoryOperationResult {
            repository,
            validation,
            succeeded: result.is_ok(),
            error_message,
        })
    }
    pub async fn process_backup(&self, shutdown: &CancellationToken) -> Result<bool, BackupError> {
        let Some(claim) = self
            .store
            .claim_backup(Utc::now() - self.stale_after)
            .await?
        else {
            return Ok(false);
        };
        self.changed();
        let token = shutdown.child_token();
        self.active_backups
            .lock()
            .map_err(poison)?
            .insert(claim.run.id, token.clone());
        let result = match self.authorizer.authorize_backup(&claim).await {
            Ok(()) => match self.planner.plan(&claim, &token).await {
                Ok(plan) => match self.store.prepare_backup_items(&claim, &plan).await {
                    Ok(()) => self.executor.backup(&claim, &plan, &token).await,
                    Err(error) => {
                        let mut message = error.to_string();
                        if let Some(directory) = plan.local_directory.as_deref()
                            && let Err(cleanup) = tokio::fs::remove_dir_all(directory).await
                            && cleanup.kind() != std::io::ErrorKind::NotFound
                        {
                            message.push_str(&format!(
                                "; recovery staging cleanup also failed: {cleanup}"
                            ));
                        }
                        failed_before_execution("SourceBusy", message)
                    }
                },
                Err(error) => failed_before_execution("SourceUnavailable", error.to_string()),
            },
            Err(error) => rejected_backup(error.to_string()),
        };
        if let Ok(mut active) = self.active_backups.lock() {
            active.remove(&claim.run.id);
        }
        self.store.finish_backup(&claim, &result).await?;
        self.changed();
        Ok(true)
    }
    pub async fn queue_due_scheduled(&self, now: DateTime<Utc>) -> Result<usize, BackupError> {
        let minute = now
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .ok_or_else(|| BackupError::Storage("Could not normalize scheduler time.".into()))?;
        let mut queued = 0;
        for policy in self.store.list_scheduled_policies().await? {
            if schedule_is_due(
                policy.cron.as_deref(),
                policy.time_zone.as_deref().unwrap_or("UTC"),
                minute,
            ) && self
                .store
                .enqueue_scheduled_backup(policy.id, minute)
                .await?
            {
                queued += 1;
                self.changed();
            }
        }
        Ok(queued)
    }
    pub async fn process_restore(&self, shutdown: &CancellationToken) -> Result<bool, BackupError> {
        let Some(claim) = self
            .store
            .claim_restore(Utc::now() - self.stale_after)
            .await?
        else {
            return Ok(false);
        };
        self.changed();
        let token = shutdown.child_token();
        self.active_restores
            .lock()
            .map_err(poison)?
            .insert(claim.run.id, token.clone());
        let result = match self.authorizer.authorize_restore(&claim).await {
            Ok(()) => self.executor.restore(&claim, &token).await,
            Err(error) => rejected_restore(error.to_string()),
        };
        if let Ok(mut active) = self.active_restores.lock() {
            active.remove(&claim.run.id);
        }
        self.store.finish_restore(&claim, &result).await?;
        self.changed();
        Ok(true)
    }
    pub async fn cancel_backup(&self, id: Uuid) -> Result<(), BackupError> {
        if let Some(t) = self
            .active_backups
            .lock()
            .map_err(poison)?
            .get(&id)
            .cloned()
        {
            t.cancel();
            return Ok(());
        }
        if self.store.cancel_backup(id).await? {
            Ok(())
        } else {
            Err(BackupError::Conflict("Backup Run is not active.".into()))
        }
    }
    pub async fn cancel_restore(&self, id: Uuid) -> Result<(), BackupError> {
        if let Some(t) = self
            .active_restores
            .lock()
            .map_err(poison)?
            .get(&id)
            .cloned()
        {
            t.cancel();
            return Ok(());
        }
        if self.store.cancel_restore(id).await? {
            Ok(())
        } else {
            Err(BackupError::Conflict("Restore Run is not active.".into()))
        }
    }
}

fn rejected_backup(message: String) -> BackupExecutionResult {
    BackupExecutionResult {
        status: "Failed",
        snapshot_availability: "NotCreated",
        restic_snapshot_id: None,
        parent_snapshot_id: None,
        files_processed: None,
        bytes_processed: None,
        bytes_added: None,
        exit_code: None,
        error_code: Some("AuthorizationRevoked".to_owned()),
        error_message: Some(message),
        warnings: vec![],
        logs: vec![],
        items: vec![],
    }
}

fn failed_before_execution(code: &str, message: String) -> BackupExecutionResult {
    BackupExecutionResult {
        status: "Failed",
        snapshot_availability: "NotCreated",
        restic_snapshot_id: None,
        parent_snapshot_id: None,
        files_processed: None,
        bytes_processed: None,
        bytes_added: None,
        exit_code: None,
        error_code: Some(code.to_owned()),
        error_message: Some(message),
        warnings: vec![],
        logs: vec![],
        items: vec![],
    }
}

fn rejected_restore(message: String) -> RestoreExecutionResult {
    RestoreExecutionResult {
        status: "Failed",
        exit_code: None,
        error_code: Some("AuthorizationRevoked".to_owned()),
        error_message: Some(message),
        logs: vec![],
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("{0}")]
    Validation(String),
    #[error("Backup resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("backup storage failed: {0}")]
    Storage(String),
    #[error("external Backup operation failed: {0}")]
    External(String),
}
fn poison<T>(_: std::sync::PoisonError<T>) -> BackupError {
    BackupError::Storage("Backup cancellation state is poisoned.".into())
}
fn validate_name(value: &mut String, label: &str) -> Result<(), BackupError> {
    *value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 128 {
        Err(BackupError::Validation(format!(
            "{label} name must contain between 1 and 128 characters."
        )))
    } else {
        Ok(())
    }
}
fn discriminator(value: &Value) -> Result<&str, BackupError> {
    value.get("$type").and_then(Value::as_str).ok_or_else(|| {
        BackupError::Validation("Polymorphic Backup input requires a $type discriminator.".into())
    })
}
fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, BackupError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| BackupError::Validation(format!("Backup field '{key}' is required.")))
}
fn required_uuid(value: &Value, key: &str) -> Result<Uuid, BackupError> {
    required_string(value, key).and_then(|v| {
        Uuid::parse_str(v)
            .map_err(|_| BackupError::Validation(format!("Backup field '{key}' must be a UUID.")))
    })
}
fn reqwest_url(value: &str) -> Result<url::Url, BackupError> {
    let url = url::Url::parse(value)
        .map_err(|_| BackupError::Validation("S3 endpoint is invalid.".into()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        Err(BackupError::Validation("S3 endpoint is invalid.".into()))
    } else {
        Ok(url)
    }
}

fn valid_cron(expression: &str) -> bool {
    let fields = expression.split_whitespace().collect::<Vec<_>>();
    fields.len() == 5
        && cron_field_valid(fields[0], 0, 59)
        && cron_field_valid(fields[1], 0, 23)
        && cron_field_valid(fields[2], 1, 31)
        && cron_field_valid(fields[3], 1, 12)
        && cron_field_valid(fields[4], 0, 7)
}

fn schedule_is_due(expression: Option<&str>, time_zone: &str, now: DateTime<Utc>) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    let fields = expression.split_whitespace().collect::<Vec<_>>();
    let Ok(zone) = time_zone.parse::<Tz>() else {
        return false;
    };
    if fields.len() != 5 {
        return false;
    }
    let local = now.with_timezone(&zone);
    let day_of_month = cron_matches(fields[2], local.day(), 1, 31);
    let week_day = local.weekday().num_days_from_sunday();
    let day_of_week = cron_matches(fields[4], week_day, 0, 7)
        || (week_day == 0 && cron_matches(fields[4], 7, 0, 7));
    let day = if fields[2] != "*" && fields[4] != "*" {
        day_of_month || day_of_week
    } else {
        day_of_month && day_of_week
    };
    cron_matches(fields[0], local.minute(), 0, 59)
        && cron_matches(fields[1], local.hour(), 0, 23)
        && cron_matches(fields[3], local.month(), 1, 12)
        && day
}

fn cron_field_valid(field: &str, minimum: u32, maximum: u32) -> bool {
    (minimum..=maximum).any(|value| cron_matches(field, value, minimum, maximum))
}

fn cron_matches(field: &str, value: u32, minimum: u32, maximum: u32) -> bool {
    field.split(',').any(|part| {
        let (range, step) = part.split_once('/').map_or((part, 1), |(range, step)| {
            (range, step.parse().unwrap_or(0))
        });
        if step == 0 {
            return false;
        }
        let bounds = if range == "*" {
            Some((minimum, maximum))
        } else if let Some((start, end)) = range.split_once('-') {
            start.parse().ok().zip(end.parse().ok())
        } else {
            range.parse().ok().map(|single| (single, single))
        };
        bounds.is_some_and(|(start, end)| {
            start >= minimum
                && end <= maximum
                && start <= value
                && value <= end
                && (value - start).is_multiple_of(step)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn repository_requires_polymorphic_type() {
        let mut input = BackupRepositoryInput {
            name: "repo".into(),
            description: None,
            spec: json!({}),
            password_secret_id: Uuid::now_v7(),
        };
        assert!(matches!(input.validate(), Err(BackupError::Validation(_))))
    }
    #[test]
    fn policy_normalizes_defaults() {
        let actor = ActorId::new(Uuid::now_v7());
        let mut input = BackupPolicyInput {
            name: " policy ".into(),
            description: None,
            source: json!({"$type":"CitadelSystem"}),
            backup_repository_id: Uuid::now_v7(),
            enabled: true,
            cron: None,
            time_zone: None,
            webhook: None,
            keep_last_successful: None,
            timeout_seconds: None,
            alert_on_failure: true,
            run_as_actor_id: None,
            tag_ids: vec![],
        };
        input.validate(actor).unwrap();
        assert_eq!(input.name, "policy");
        assert_eq!(input.keep_last_successful, Some(14));
    }

    #[test]
    fn schedule_validation_and_time_zone_matching_follow_five_field_cron() {
        let now = DateTime::parse_from_rfc3339("2026-07-14T08:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(schedule_is_due(Some("30 10 * * 2"), "Europe/Paris", now));
        assert!(!schedule_is_due(Some("31 10 * * 2"), "Europe/Paris", now));
        assert!(!valid_cron("* * *"));

        let actor = ActorId::new(Uuid::now_v7());
        let mut input = BackupPolicyInput {
            name: "invalid schedule".into(),
            description: None,
            source: json!({"$type":"CitadelSystem"}),
            backup_repository_id: Uuid::now_v7(),
            enabled: true,
            cron: Some("not cron".into()),
            time_zone: Some("Nowhere/Invalid".into()),
            webhook: None,
            keep_last_successful: None,
            timeout_seconds: None,
            alert_on_failure: false,
            run_as_actor_id: None,
            tag_ids: vec![],
        };
        assert!(matches!(
            input.validate(actor),
            Err(BackupError::Validation(_))
        ));
    }
}
