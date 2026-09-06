#![forbid(unsafe_code)]

mod logs;
pub use logs::{BuildLogEntry, BuildLogNotifier, BuildLogSink, NoopBuildLogSink};

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildArgSpec {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildSecretSpec {
    pub id: String,
    pub secret_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: Option<String>,
    pub context_path: Option<String>,
    pub dockerfile_path: Option<String>,
    pub target: Option<String>,
    pub build_args: Option<Vec<BuildArgSpec>>,
    pub build_secrets: Option<Vec<BuildSecretSpec>>,
    pub platform_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Option<Vec<String>>,
    pub webhook: Option<Value>,
    pub timeout_seconds: Option<i32>,
    pub retention_run_count: Option<i32>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default = "platform_builder")]
    pub builder_kind: String,
    pub build_agent_pool_id: Option<Uuid>,
}

fn platform_builder() -> String {
    "Platform".to_owned()
}

impl BuildProjectInput {
    pub fn validate(&mut self) -> Result<(), BuildError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(BuildError::Validation(
                "Build Project name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        self.description = self
            .description
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 600)
        {
            return Err(BuildError::Validation(
                "Build Project description cannot exceed 600 characters.".to_owned(),
            ));
        }
        if self.git_repository_id.is_nil() || self.registry_id.is_nil() {
            return Err(BuildError::Validation(
                "Git Repository and Registry are required.".to_owned(),
            ));
        }
        match self.builder_kind.as_str() {
            "Platform"
                if self.platform_id.is_some_and(|id| !id.is_nil())
                    && self.build_agent_pool_id.is_none() => {}
            "BuildAgentPool"
                if self.platform_id.is_none()
                    && self.build_agent_pool_id.is_some_and(|id| !id.is_nil()) => {}
            _ => {
                return Err(BuildError::Validation(
                    "Select exactly one valid Build execution target.".to_owned(),
                ));
            }
        }
        let branch = normalize_required(self.branch.take(), "main")?;
        if !valid_git_branch(&branch) {
            return Err(BuildError::Validation(
                "Git branch name is invalid.".to_owned(),
            ));
        }
        self.branch = Some(branch);
        self.context_path = Some(normalize_path(self.context_path.take(), ".")?);
        self.dockerfile_path = Some(normalize_path(self.dockerfile_path.take(), "Dockerfile")?);
        self.image_repository = self
            .image_repository
            .trim()
            .trim_start_matches('/')
            .to_owned();
        if self.image_repository.is_empty() || self.image_repository.len() > 512 {
            return Err(BuildError::Validation(
                "Image repository is required and cannot exceed 512 bytes.".to_owned(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(1_800);
        if !(60..=86_400).contains(&timeout) {
            return Err(BuildError::Validation(
                "Build timeout must be between 60 and 86400 seconds.".to_owned(),
            ));
        }
        let retention = self.retention_run_count.unwrap_or(20);
        if !(1..=1_000).contains(&retention) {
            return Err(BuildError::Validation(
                "Build retention must keep between 1 and 1000 runs.".to_owned(),
            ));
        }
        self.timeout_seconds = Some(timeout);
        self.retention_run_count = Some(retention);
        let mut tags = self
            .tag_templates
            .take()
            .unwrap_or_else(|| vec!["{branch}-{shortSha}".to_owned()]);
        tags = tags
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect();
        tags.sort();
        tags.dedup();
        if tags.is_empty() {
            return Err(BuildError::Validation(
                "At least one image tag template is required.".to_owned(),
            ));
        }
        self.tag_templates = Some(tags);
        let mut secret_ids = HashSet::new();
        let build_secrets = self.build_secrets.get_or_insert_with(Vec::new);
        if build_secrets.len() > 64 {
            return Err(BuildError::Validation(
                "A Build cannot reference more than 64 Secrets.".to_owned(),
            ));
        }
        for secret in build_secrets {
            secret.id = secret.id.trim().to_owned();
            if secret.id.is_empty()
                || !secret
                    .id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
            {
                return Err(BuildError::Validation(format!(
                    "BuildKit secret id '{}' is invalid.",
                    secret.id
                )));
            }
            if secret.secret_id.is_nil() {
                return Err(BuildError::Validation(format!(
                    "BuildKit secret '{}' must reference a Citadel Secret.",
                    secret.id
                )));
            }
            if !secret_ids.insert(secret.id.to_ascii_lowercase()) {
                return Err(BuildError::Validation(format!(
                    "BuildKit secret id '{}' is mapped more than once.",
                    secret.id
                )));
            }
        }
        let build_args = self.build_args.get_or_insert_with(Vec::new);
        if build_args.len() > 256 {
            return Err(BuildError::Validation(
                "A Build cannot define more than 256 build arguments.".to_owned(),
            ));
        }
        let mut argument_names = HashSet::new();
        for argument in build_args {
            argument.name = argument.name.trim().to_owned();
            if argument.name.is_empty()
                || argument.name.len() > 256
                || !argument_names.insert(argument.name.to_ascii_lowercase())
            {
                return Err(BuildError::Validation(
                    "Build argument names must be non-empty and unique.".to_owned(),
                ));
            }
            if argument
                .value
                .as_ref()
                .is_some_and(|value| value.len() > 16 * 1024)
            {
                return Err(BuildError::Validation(
                    "A Build argument value cannot exceed 16 KiB.".to_owned(),
                ));
            }
        }
        if self.tag_ids.len() > 100 {
            return Err(BuildError::Validation(
                "A Build cannot have more than 100 Tags.".to_owned(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        Ok(())
    }
}

fn normalize_required(value: Option<String>, fallback: &str) -> Result<String, BuildError> {
    let value = value
        .unwrap_or_else(|| fallback.to_owned())
        .trim()
        .to_owned();
    if value.is_empty() || value.len() > 512 {
        Err(BuildError::Validation(
            "Build path or branch is invalid.".to_owned(),
        ))
    } else {
        Ok(value)
    }
}

fn valid_git_branch(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 255
        && !branch.starts_with('-')
        && !branch.starts_with('.')
        && !branch.ends_with('.')
        && !branch.ends_with('/')
        && !branch.contains("..")
        && !branch.contains("@{")
        && !branch.contains([' ', '~', '^', ':', '?', '*', '[', '\\', '\r', '\n'])
}

fn normalize_path(value: Option<String>, fallback: &str) -> Result<String, BuildError> {
    let value = normalize_required(value, fallback)?.replace('\\', "/");
    let value = value.trim_start_matches('/').to_owned();
    if value.split('/').any(|part| matches!(part, ".." | ".git")) {
        return Err(BuildError::Validation(
            "Build paths cannot traverse or access Git metadata.".to_owned(),
        ));
    }
    Ok(value)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProjectView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub build_args: Vec<BuildArgSpec>,
    pub build_secrets: Vec<BuildSecretSpec>,
    pub builder_kind: String,
    pub platform_id: Option<Uuid>,
    pub build_agent_pool_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Vec<String>,
    pub webhook: Option<Value>,
    pub timeout_seconds: i32,
    pub retention_run_count: i32,
    pub current_run_id: Option<Uuid>,
    pub control_state: String,
    pub control_started_at: Option<i64>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRunView {
    pub id: Uuid,
    pub build_project_id: Uuid,
    pub project_name_snapshot: String,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: Option<String>,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub registry_id: Uuid,
    pub registry_host: String,
    pub image_repository: String,
    pub image_references: Vec<String>,
    pub trigger: String,
    pub status: String,
    pub image_digest: Option<String>,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildAgentPoolInput {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub provider_spec: Value,
    pub max_active_builders: Option<i32>,
    pub queue_timeout_seconds: Option<i32>,
    pub provisioning_timeout_seconds: Option<i32>,
    pub registration_timeout_seconds: Option<i32>,
    pub heartbeat_timeout_seconds: Option<i32>,
    pub cleanup_timeout_seconds: Option<i32>,
    pub maximum_instance_lifetime_seconds: Option<i32>,
    pub failure_retention_minutes: Option<i32>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl BuildAgentPoolInput {
    pub fn validate(&mut self) -> Result<(), BuildError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(BuildError::Validation(
                "Build Agent Pool name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        let provider = self
            .provider_spec
            .get("$type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(provider, "HetznerCloud" | "GenericEdge") {
            return Err(BuildError::Validation(
                "Build Agent Pool provider is unsupported.".to_owned(),
            ));
        }
        validate_range(
            self.max_active_builders.unwrap_or(1),
            1,
            100,
            "maximum active builders",
        )?;
        if self.tag_ids.len() > 100 {
            return Err(BuildError::Validation(
                "A Build Agent Pool cannot have more than 100 Tags.".to_owned(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        validate_range(
            self.queue_timeout_seconds.unwrap_or(3600),
            60,
            86_400,
            "queue timeout",
        )?;
        validate_range(
            self.provisioning_timeout_seconds.unwrap_or(600),
            30,
            3600,
            "provisioning timeout",
        )?;
        validate_range(
            self.registration_timeout_seconds.unwrap_or(300),
            30,
            3600,
            "registration timeout",
        )?;
        validate_range(
            self.heartbeat_timeout_seconds.unwrap_or(90),
            10,
            3600,
            "heartbeat timeout",
        )?;
        validate_range(
            self.cleanup_timeout_seconds.unwrap_or(600),
            30,
            3600,
            "cleanup timeout",
        )?;
        validate_range(
            self.maximum_instance_lifetime_seconds.unwrap_or(7200),
            300,
            86_400,
            "maximum instance lifetime",
        )?;
        validate_range(
            self.failure_retention_minutes.unwrap_or(0),
            0,
            10_080,
            "failure retention",
        )?;
        Ok(())
    }
}
fn validate_range(value: i32, min: i32, max: i32, label: &str) -> Result<(), BuildError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(BuildError::Validation(format!(
            "Build Agent Pool {label} is out of range."
        )))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildAgentPoolView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub provider: String,
    pub provider_spec: Value,
    pub max_active_builders: i32,
    pub queue_timeout_seconds: i32,
    pub provisioning_timeout_seconds: i32,
    pub registration_timeout_seconds: i32,
    pub heartbeat_timeout_seconds: i32,
    pub cleanup_timeout_seconds: i32,
    pub maximum_instance_lifetime_seconds: i32,
    pub failure_retention_minutes: i32,
    pub last_validation_status: String,
    pub last_validation_message: Option<String>,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub control_state: String,
    pub control_triggered_by: Option<Uuid>,
    pub control_started_at: Option<i64>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

#[derive(Debug, Clone)]
pub struct BuildClaim {
    pub project: BuildProjectView,
    pub run: BuildRunView,
}

#[derive(Debug, Clone)]
pub struct BuildExecutionResult {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub image_digest: Option<String>,
    pub resolved_commit_sha: Option<String>,
    pub image_references: Vec<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub logs: Vec<BuildLog>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildLog {
    pub stream: String,
    pub message: String,
}

pub trait BuildStore: Send + Sync {
    fn create_pool<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildAgentPoolInput,
    ) -> BoxFuture<'a, Result<BuildAgentPoolView, BuildError>>;
    fn list_pools(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPoolView>, BuildError>>;
    fn get_pool<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildAgentPoolView, BuildError>>;
    fn archive_pool<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BuildError>>;
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildProjectInput,
    ) -> BoxFuture<'a, Result<BuildProjectView, BuildError>>;
    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildProjectView>, BuildError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildProjectView, BuildError>>;
    fn archive<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BuildError>>;
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<BuildRunView, BuildError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<BuildClaim>, BuildError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a BuildClaim,
        result: &'a BuildExecutionResult,
    ) -> BoxFuture<'a, Result<(), BuildError>>;
    fn list_runs<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<BuildRunView>, BuildError>>;
    fn get_run<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildRunView, BuildError>>;
    fn logs<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<Vec<BuildLogEntry>, BuildError>>;
    fn append_log<'a>(
        &'a self,
        run_id: Uuid,
        log: &'a BuildLog,
    ) -> BoxFuture<'a, Result<BuildLogEntry, BuildError>>;
    fn cancel_queued<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<bool, BuildError>>;
}

pub trait BuildExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        claim: &'a BuildClaim,
        logs: &'a dyn BuildLogSink,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult>;
}

pub trait BuildSecretResolver: Send + Sync {
    fn resolve(
        &self,
        secret_id: Uuid,
    ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BuildError>>;
}

pub struct BuildRegistryCredentials {
    pub username: String,
    pub password: zeroize::Zeroizing<String>,
}

pub trait BuildRegistryCredentialResolver: Send + Sync {
    fn resolve(
        &self,
        registry_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRegistryCredentials>, BuildError>>;
}

pub struct BuildService {
    on_log: Option<BuildLogNotifier>,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    store: Arc<dyn BuildStore>,
    executor: Arc<dyn BuildExecutor>,
    active: Mutex<std::collections::HashMap<Uuid, CancellationToken>>,
    stale_after: chrono::Duration,
    alerts: Option<Arc<dyn AlertEventSink>>,
}

impl BuildService {
    pub fn with_log_notifier(mut self, notifier: BuildLogNotifier) -> Self {
        self.on_log = Some(notifier);
        self
    }
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
        store: Arc<dyn BuildStore>,
        executor: Arc<dyn BuildExecutor>,
        stale_after: chrono::Duration,
    ) -> Self {
        Self {
            on_log: None,
            store,
            on_change: None,
            executor,
            active: Mutex::new(std::collections::HashMap::new()),
            stale_after,
            alerts: None,
        }
    }
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }
    pub fn store(&self) -> &Arc<dyn BuildStore> {
        &self.store
    }
    pub async fn process_one(&self, shutdown: &CancellationToken) -> Result<bool, BuildError> {
        let Some(claim) = self.store.claim_next(Utc::now() - self.stale_after).await? else {
            return Ok(false);
        };
        self.changed();
        let cancellation = shutdown.child_token();
        self.active
            .lock()
            .map_err(|_| BuildError::Storage("Build cancellation state is poisoned.".to_owned()))?
            .insert(claim.run.id, cancellation.clone());
        let logs =
            logs::PersistedBuildLogs::new(self.store.clone(), claim.run.id, self.on_log.clone());
        let execution = self.executor.execute(&claim, &logs, &cancellation);
        tokio::pin!(execution);
        let result = tokio::select! {
            result = &mut execution => result,
            () = tokio::time::sleep(std::time::Duration::from_secs(claim.run.timeout_seconds as u64)) => {
                cancellation.cancel();
                // Executors must observe cancellation and finish child cleanup before the
                // claim is persisted as terminal. This prevents detached Docker/Git work.
                let _ = execution.await;
                BuildExecutionResult {
                    status: "TimedOut",
                    exit_code: None,
                    image_digest: None,
                    resolved_commit_sha: None,
                    image_references: vec![],
                    error_code: Some("build.timeout".to_owned()),
                    error_message: Some("Build Run exceeded its configured timeout.".to_owned()),
                    logs: vec![BuildLog {
                        stream: "stderr".to_owned(),
                        message: "Build Run exceeded its configured timeout.".to_owned(),
                    }],
                }
            }
        };
        if let Ok(mut active) = self.active.lock() {
            active.remove(&claim.run.id);
        }
        self.store.finish(&claim, &result).await?;
        self.changed();
        if matches!(result.status, "Failed" | "TimedOut")
            && let Some(alerts) = &self.alerts
        {
            let message = result
                .error_message
                .as_deref()
                .unwrap_or("Build Run failed.");
            let observation = AlertObservation {
                alert_type: "BuildRunFailed".to_owned(),
                info: serde_json::json!({
                    "RunId": claim.run.id,
                    "Trigger": claim.run.trigger,
                    "Status": result.status,
                    "ExitCode": result.exit_code,
                    "ErrorMessage": message,
                    "HumanMessage": message,
                }),
                resource_id: claim.project.id,
                resource_name: claim.project.name.clone(),
                resource_type: "Build".to_owned(),
                deduplication_component: claim.run.id.to_string(),
                observed_at: Utc::now(),
                value: None,
                matched: true,
            };
            if let Err(error) = alerts.observe(&observation).await {
                tracing::error!(%error, run_id=%claim.run.id, "Build failure alert evaluation failed");
            }
        }
        Ok(true)
    }
    pub async fn cancel(&self, id: Uuid) -> Result<(), BuildError> {
        if let Some(token) = self
            .active
            .lock()
            .map_err(|_| BuildError::Storage("Build cancellation state is poisoned.".to_owned()))?
            .get(&id)
            .cloned()
        {
            token.cancel();
            return Ok(());
        }
        if self.store.cancel_queued(id).await? {
            Ok(())
        } else {
            Err(BuildError::Conflict("Build Run is not active.".to_owned()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("{0}")]
    Validation(String),
    #[error("Build resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> BuildProjectInput {
        BuildProjectInput {
            name: " demo ".into(),
            description: None,
            enabled: true,
            git_repository_id: Uuid::now_v7(),
            branch: None,
            context_path: None,
            dockerfile_path: None,
            target: None,
            build_args: None,
            build_secrets: None,
            platform_id: Some(Uuid::now_v7()),
            registry_id: Uuid::now_v7(),
            image_repository: " team/demo ".into(),
            tag_templates: None,
            webhook: None,
            timeout_seconds: None,
            retention_run_count: None,
            tag_ids: vec![],
            builder_kind: "Platform".into(),
            build_agent_pool_id: None,
        }
    }

    #[test]
    fn validates_and_normalizes_build_project() {
        let mut value = input();
        value.validate().unwrap();
        assert_eq!(value.name, "demo");
        assert_eq!(value.context_path.as_deref(), Some("."));
        assert_eq!(value.timeout_seconds, Some(1800));
    }

    #[test]
    fn rejects_invalid_duplicate_or_unbound_build_secrets() {
        let mut invalid = input();
        invalid.build_secrets = Some(vec![BuildSecretSpec {
            id: "npm token".into(),
            secret_id: Uuid::now_v7(),
        }]);
        assert!(invalid.validate().is_err());
        let mut duplicate = input();
        duplicate.build_secrets = Some(vec![
            BuildSecretSpec {
                id: "npmrc".into(),
                secret_id: Uuid::now_v7(),
            },
            BuildSecretSpec {
                id: "NPMRC".into(),
                secret_id: Uuid::now_v7(),
            },
        ]);
        assert!(duplicate.validate().is_err());
        let mut unbound = input();
        unbound.build_secrets = Some(vec![BuildSecretSpec {
            id: "npmrc".into(),
            secret_id: Uuid::nil(),
        }]);
        assert!(unbound.validate().is_err());
    }
}
