#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use citadel_domain::ActorId;
use citadel_execution::{OutputLimitPolicy, ProcessError, ProcessLimits, ProcessRequest, run};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationActionInput {
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: Option<String>,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: Option<String>,
    pub webhook: Option<Value>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl AutomationActionInput {
    pub fn validate(&mut self, default_actor: ActorId) -> Result<(), AutomationError> {
        self.name = self.name.trim().to_owned();
        if !(3..=64).contains(&self.name.chars().count()) {
            return Err(AutomationError::Validation(
                "Automation Action name must contain between 3 and 64 characters.".to_owned(),
            ));
        }
        if self.code.trim().is_empty() || self.code.len() > 1024 * 1024 {
            return Err(AutomationError::Validation(
                "Automation code must contain between 1 byte and 1 MiB.".to_owned(),
            ));
        }
        let args = self.default_args_json.as_deref().unwrap_or("{}");
        if !serde_json::from_str::<Value>(args).is_ok_and(|value| value.is_object()) {
            return Err(AutomationError::Validation(
                "Default arguments must be a JSON object.".to_owned(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(60);
        if !(1..=3600).contains(&timeout) {
            return Err(AutomationError::Validation(
                "Automation timeout must be between 1 and 3600 seconds.".to_owned(),
            ));
        }
        if self.schedule_enabled
            && (self.schedule_cron.as_deref().is_none_or(str::is_empty)
                || self.schedule_time_zone.as_deref().is_none_or(str::is_empty))
        {
            return Err(AutomationError::Validation(
                "Enabled schedules require both a cron expression and a time zone.".to_owned(),
            ));
        }
        self.default_args_json = Some(args.to_owned());
        self.timeout_seconds = Some(timeout);
        self.schedule_time_zone = Some(
            self.schedule_time_zone
                .take()
                .unwrap_or_else(|| "UTC".to_owned()),
        );
        self.run_as_actor_id = Some(self.run_as_actor_id.unwrap_or(default_actor.value()));
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationActionView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub webhook: Option<Value>,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub row_version: i64,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunView {
    pub id: Uuid,
    pub action_id: Uuid,
    pub action_name: String,
    pub trigger: String,
    pub status: String,
    pub run_as_actor_id: Uuid,
    pub triggered_by_actor_id: Option<Uuid>,
    pub args_json: Value,
    pub code_snapshot: String,
    pub code_hash: String,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub logs: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AutomationRunClaim {
    pub run: AutomationRunView,
}

pub trait AutomationStore: Send + Sync {
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AutomationActionInput,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>>;
    fn list<'a>(&'a self) -> BoxFuture<'a, Result<Vec<AutomationActionView>, AutomationError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>>;
    fn delete<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), AutomationError>>;
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a Value,
    ) -> BoxFuture<'a, Result<AutomationRunView, AutomationError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunClaim>, AutomationError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        result: &'a AutomationRunResult,
    ) -> BoxFuture<'a, Result<(), AutomationError>>;
    fn list_runs<'a>(
        &'a self,
        action_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationRunView>, AutomationError>>;
    fn cancel<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AutomationError>>;
}

pub struct AutomationService {
    store: Arc<dyn AutomationStore>,
    deno_path: OsString,
    work_root: PathBuf,
    internal_base_url: String,
    maximum_log_bytes: usize,
    stale_after: Duration,
}

impl AutomationService {
    pub fn new(
        store: Arc<dyn AutomationStore>,
        deno_path: OsString,
        work_root: PathBuf,
        internal_base_url: String,
        maximum_log_bytes: usize,
        stale_after: Duration,
    ) -> Self {
        Self {
            store,
            deno_path,
            work_root,
            internal_base_url,
            maximum_log_bytes,
            stale_after,
        }
    }

    pub fn store(&self) -> &Arc<dyn AutomationStore> {
        &self.store
    }

    pub async fn process_one(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<bool, AutomationError> {
        let stale_before = Utc::now()
            - chrono::Duration::from_std(self.stale_after)
                .map_err(|error| AutomationError::Storage(error.to_string()))?;
        let Some(claim) = self.store.claim_next(stale_before).await? else {
            return Ok(false);
        };
        let result = self.execute(&claim, cancellation).await;
        self.store.finish(&claim, &result).await?;
        Ok(true)
    }

    async fn execute(
        &self,
        claim: &AutomationRunClaim,
        cancellation: &CancellationToken,
    ) -> AutomationRunResult {
        let directory = self.work_root.join(claim.run.id.to_string());
        if let Err(error) = tokio::fs::create_dir_all(&directory).await {
            return AutomationRunResult::failed(None, format!("Could not prepare run: {error}"));
        }
        let script = directory.join("action.ts");
        if let Err(error) = tokio::fs::write(&script, claim.run.code_snapshot.as_bytes()).await {
            let _ = tokio::fs::remove_dir_all(&directory).await;
            return AutomationRunResult::failed(None, format!("Could not write action: {error}"));
        }
        let request = ProcessRequest::new(self.deno_path.clone())
            .args([
                OsString::from("run"),
                OsString::from("--no-prompt"),
                OsString::from(format!(
                    "--allow-net={}",
                    allow_net_authority(&self.internal_base_url)
                )),
                script.as_os_str().to_owned(),
            ])
            .current_dir(&directory)
            .env("CITADEL_ACTION_ARGS", claim.run.args_json.to_string())
            .limits(ProcessLimits {
                timeout: Duration::from_secs(claim.run.timeout_seconds as u64),
                maximum_stdout_bytes: self.maximum_log_bytes,
                maximum_stderr_bytes: self.maximum_log_bytes,
                output_limit_policy: OutputLimitPolicy::Truncate,
            });
        let output = run(request, cancellation).await;
        let _ = tokio::fs::remove_dir_all(&directory).await;
        match output {
            Ok(output) => {
                let logs = joined_logs(
                    &output.stdout,
                    &output.stderr,
                    output.stdout_truncated || output.stderr_truncated,
                );
                if output.succeeded() {
                    AutomationRunResult::success(output.exit_code, logs)
                } else {
                    AutomationRunResult::failed(output.exit_code, logs)
                }
            }
            Err(ProcessError::Cancelled) => AutomationRunResult::cancelled(),
            Err(ProcessError::Timeout(_)) => AutomationRunResult::timed_out(),
            Err(error) => AutomationRunResult::failed(None, error.to_string()),
        }
    }
}

pub struct AutomationRunResult {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub logs: String,
    pub error: Option<String>,
}

impl AutomationRunResult {
    fn success(exit_code: Option<i32>, logs: String) -> Self {
        Self {
            status: "Succeeded",
            exit_code,
            logs,
            error: None,
        }
    }
    fn failed(exit_code: Option<i32>, message: String) -> Self {
        Self {
            status: "Failed",
            exit_code,
            logs: message.clone(),
            error: Some(message),
        }
    }
    fn cancelled() -> Self {
        Self {
            status: "Cancelled",
            exit_code: None,
            logs: String::new(),
            error: Some("Automation run was cancelled.".to_owned()),
        }
    }
    fn timed_out() -> Self {
        Self {
            status: "TimedOut",
            exit_code: None,
            logs: String::new(),
            error: Some("Automation run timed out.".to_owned()),
        }
    }
}

pub fn code_hash(code: &str) -> String {
    let bytes = Sha256::digest(code.as_bytes());
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn joined_logs(stdout: &[u8], stderr: &[u8], truncated: bool) -> String {
    let mut output = String::from_utf8_lossy(stdout).into_owned();
    if !stderr.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&String::from_utf8_lossy(stderr));
    }
    if truncated {
        output.push_str("\n[output truncated]");
    }
    output
}

fn allow_net_authority(base_url: &str) -> &str {
    base_url
        .split_once("://")
        .map_or(base_url, |(_, authority)| authority)
        .trim_end_matches('/')
}

#[derive(Debug, thiserror::Error)]
pub enum AutomationError {
    #[error("{0}")]
    Validation(String),
    #[error("Automation Action was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> AutomationActionInput {
        AutomationActionInput {
            name: "prune-images".to_owned(),
            description: None,
            code: "console.log('ok')".to_owned(),
            default_args_json: Some("{}".to_owned()),
            enabled: true,
            schedule_enabled: false,
            schedule_cron: None,
            schedule_time_zone: None,
            webhook: None,
            timeout_seconds: Some(30),
            alert_on_failure: true,
            run_as_actor_id: None,
            tag_ids: vec![],
        }
    }

    #[test]
    fn validates_json_timeout_schedule_and_default_actor() {
        let actor = ActorId::new(Uuid::now_v7());
        let mut valid = input();
        valid.validate(actor).unwrap();
        assert_eq!(valid.run_as_actor_id, Some(actor.value()));
        let mut invalid = input();
        invalid.default_args_json = Some("[]".to_owned());
        assert!(invalid.validate(actor).is_err());
        assert_eq!(
            allow_net_authority("http://127.0.0.1:8000"),
            "127.0.0.1:8000"
        );
        let mut invalid = input();
        invalid.schedule_enabled = true;
        assert!(invalid.validate(actor).is_err());
    }
}
