#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use chrono::{Datelike, Timelike};
use chrono_tz::Tz;
use citadel_alerts::{AlertEventSink, AlertObservation};
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
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(AutomationError::Validation(
                "Automation Action name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 600)
        {
            return Err(AutomationError::Validation(
                "Automation Action description cannot exceed 600 characters.".to_owned(),
            ));
        }
        if self.code.trim().is_empty() || self.code.len() > 256 * 1024 {
            return Err(AutomationError::Validation(
                "Automation code must contain between 1 byte and 256 KiB.".to_owned(),
            ));
        }
        let args = self.default_args_json.as_deref().unwrap_or("{}");
        if args.len() > 64 * 1024 {
            return Err(AutomationError::Validation(
                "Automation Action default arguments cannot exceed 64 KiB.".to_owned(),
            ));
        }
        if !serde_json::from_str::<Value>(args).is_ok_and(|value| value.is_object()) {
            return Err(AutomationError::Validation(
                "Default arguments must be a JSON object.".to_owned(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(60);
        if !(1..=86_400).contains(&timeout) {
            return Err(AutomationError::Validation(
                "Automation timeout must be between 1 and 86400 seconds.".to_owned(),
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

impl AutomationActionView {
    pub fn snapshot(&self) -> citadel_domain::AutomationActionActivitySnapshot {
        let mut webhook = self.webhook.clone();
        if let Some(Value::Object(fields)) = &mut webhook {
            for (key, value) in fields {
                if key.eq_ignore_ascii_case("secret") && !value.is_null() {
                    *value = Value::String("********".into());
                }
            }
        }
        citadel_domain::AutomationActionActivitySnapshot {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            code: self.code.clone(),
            default_args_json: self.default_args_json.clone(),
            enabled: self.enabled,
            schedule_enabled: self.schedule_enabled,
            schedule_cron: self.schedule_cron.clone(),
            schedule_time_zone: self.schedule_time_zone.clone(),
            webhook,
            timeout_seconds: self.timeout_seconds,
            alert_on_failure: self.alert_on_failure,
            run_as_actor_id: self.run_as_actor_id,
        }
    }
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
    pub args_json: String,
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

pub trait AutomationRunTokenIssuer: Send + Sync {
    fn issue<'a>(
        &'a self,
        run_as_actor_id: ActorId,
        run_id: Uuid,
        lifetime: Duration,
    ) -> BoxFuture<'a, Result<String, AutomationError>>;
}

pub trait AutomationStore: Send + Sync {
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AutomationActionInput,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>>;
    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AutomationActionView>, AutomationError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>>;
    fn update<'a>(
        &'a self,
        current: &'a AutomationActionView,
        input: &'a AutomationActionInput,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>>;
    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>>;
    fn delete<'a>(&'a self, id: Uuid, actor: ActorId)
    -> BoxFuture<'a, Result<(), AutomationError>>;
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<AutomationRunView, AutomationError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunClaim>, AutomationError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        result: &'a AutomationRunResult,
    ) -> BoxFuture<'a, Result<bool, AutomationError>>;
    fn list_runs<'a>(
        &'a self,
        action_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationRunView>, AutomationError>>;
    fn get_run<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<AutomationRunView, AutomationError>>;
    fn list_scheduled<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<AutomationActionView>, AutomationError>>;
    fn enqueue_scheduled<'a>(
        &'a self,
        action_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunView>, AutomationError>>;
    fn cancel<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AutomationError>>;
}

pub struct AutomationService {
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    store: Arc<dyn AutomationStore>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    deno_path: OsString,
    work_root: PathBuf,
    internal_base_url: String,
    endpoint_catalog_json: Arc<str>,
    token_issuer: Arc<dyn AutomationRunTokenIssuer>,
    maximum_log_bytes: usize,
    stale_after: Duration,
    active_runs: Mutex<HashMap<Uuid, CancellationToken>>,
}

pub struct AutomationRuntimeConfig {
    pub deno_path: OsString,
    pub work_root: PathBuf,
    pub internal_base_url: String,
    pub endpoint_catalog_json: String,
    pub maximum_log_bytes: usize,
    pub stale_after: Duration,
}

impl AutomationService {
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
        store: Arc<dyn AutomationStore>,
        token_issuer: Arc<dyn AutomationRunTokenIssuer>,
        config: AutomationRuntimeConfig,
    ) -> Self {
        Self {
            store,
            on_change: None,
            alerts: None,
            deno_path: config.deno_path,
            work_root: config.work_root,
            internal_base_url: config.internal_base_url,
            endpoint_catalog_json: Arc::from(config.endpoint_catalog_json),
            token_issuer,
            maximum_log_bytes: config.maximum_log_bytes,
            stale_after: config.stale_after,
            active_runs: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
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
        self.changed();
        let run_cancellation = cancellation.child_token();
        self.active_runs
            .lock()
            .map_err(|_| {
                AutomationError::Storage("Automation cancellation state is poisoned.".to_owned())
            })?
            .insert(claim.run.id, run_cancellation.clone());
        let result = self.execute(&claim, &run_cancellation).await;
        if let Ok(mut active) = self.active_runs.lock() {
            active.remove(&claim.run.id);
        }
        if self.store.finish(&claim, &result).await? {
            self.changed();
            self.raise_failure_alert(&claim, &result).await;
        }
        Ok(true)
    }

    async fn raise_failure_alert(&self, claim: &AutomationRunClaim, result: &AutomationRunResult) {
        if !matches!(result.status, "Failed" | "TimedOut") {
            return;
        }
        let Some(alerts) = &self.alerts else {
            return;
        };
        let action = match self.store.get(claim.run.action_id).await {
            Ok(action) if action.alert_on_failure => action,
            Ok(_) => return,
            Err(error) => {
                tracing::error!(%error, run_id=%claim.run.id, "Automation alert policy lookup failed");
                return;
            }
        };
        let message = result.error.as_deref().unwrap_or("Automation run failed.");
        let observation = AlertObservation {
            alert_type: "AutomationActionRunFailed".to_owned(),
            info: serde_json::json!({
                "RunId": claim.run.id,
                "Trigger": claim.run.trigger,
                "Status": result.status,
                "ExitCode": result.exit_code,
                "ErrorMessage": message,
                "HumanMessage": message,
            }),
            resource_id: action.id,
            resource_name: action.name,
            resource_type: "AutomationAction".to_owned(),
            deduplication_component: claim.run.id.to_string(),
            observed_at: Utc::now(),
            value: None,
            matched: true,
        };
        if let Err(error) = alerts.observe(&observation).await {
            tracing::error!(%error, run_id=%claim.run.id, "Automation failure alert evaluation failed");
        }
    }

    pub async fn cancel(&self, action_id: Uuid, run_id: Uuid) -> Result<(), AutomationError> {
        let token = self
            .active_runs
            .lock()
            .map_err(|_| {
                AutomationError::Storage("Automation cancellation state is poisoned.".to_owned())
            })?
            .get(&run_id)
            .cloned();
        if let Some(token) = token {
            let run = self.store.get_run(action_id, run_id).await?;
            if run.status != "Running" {
                return Err(AutomationError::Conflict(
                    "Automation run is not active.".to_owned(),
                ));
            }
            token.cancel();
            return Ok(());
        }
        self.store.cancel(action_id, run_id).await
    }

    pub async fn queue_due_scheduled(&self, now: DateTime<Utc>) -> Result<usize, AutomationError> {
        let minute = now
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .ok_or_else(|| {
                AutomationError::Storage("Could not normalize scheduler time.".to_owned())
            })?;
        let mut queued = 0;
        for action in self.store.list_scheduled().await? {
            if cron_is_due(
                action.schedule_cron.as_deref(),
                &action.schedule_time_zone,
                minute,
            ) && self
                .store
                .enqueue_scheduled(action.id, minute)
                .await?
                .is_some()
            {
                queued += 1;
                self.changed();
            }
        }
        Ok(queued)
    }

    async fn execute(
        &self,
        claim: &AutomationRunClaim,
        cancellation: &CancellationToken,
    ) -> AutomationRunResult {
        let token = match self
            .token_issuer
            .issue(
                ActorId::new(claim.run.run_as_actor_id),
                claim.run.id,
                Duration::from_secs(claim.run.timeout_seconds as u64 + 60),
            )
            .await
        {
            Ok(token) => token,
            Err(error) => {
                return AutomationRunResult::failed(
                    None,
                    format!("Run-as authorization failed: {error}"),
                );
            }
        };
        let directory = self.work_root.join(claim.run.id.to_string());
        if let Err(error) = tokio::fs::create_dir_all(&directory).await {
            return AutomationRunResult::failed(None, format!("Could not prepare run: {error}"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // The script contains a short-lived credential. Restrict the run
            // directory before writing it, regardless of the process umask.
            if let Err(error) =
                tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).await
            {
                let _ = tokio::fs::remove_dir(&directory).await;
                return AutomationRunResult::failed(
                    None,
                    format!("Could not secure run directory: {error}"),
                );
            }
        }
        let script = directory.join("action.ts");
        let source = automation_source(
            &self.internal_base_url,
            &token,
            &claim.run,
            &self.endpoint_catalog_json,
        );
        if let Err(error) = tokio::fs::write(&script, source.as_bytes()).await {
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
            .env("CITADEL_ACTION_ARGS", &claim.run.args_json)
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
                let logs = redact_run_logs(
                    &joined_logs(
                        &output.stdout,
                        &output.stderr,
                        output.stdout_truncated || output.stderr_truncated,
                    ),
                    &token,
                );
                if output.succeeded() {
                    AutomationRunResult::success(output.exit_code, logs)
                } else {
                    let mut result = AutomationRunResult::failed(output.exit_code, logs);
                    result.error = Some(format!(
                        "Deno exited with code {}.",
                        output
                            .exit_code
                            .map_or_else(|| "unknown".into(), |code| code.to_string())
                    ));
                    result
                }
            }
            Err(ProcessError::Cancelled) => AutomationRunResult::cancelled(),
            Err(ProcessError::Timeout(_)) => AutomationRunResult::timed_out(),
            Err(error) => AutomationRunResult::failed(None, error.to_string()),
        }
    }
}

fn automation_source(
    base_url: &str,
    token: &str,
    run: &AutomationRunView,
    endpoint_catalog_json: &str,
) -> String {
    let base_url =
        serde_json::to_string(base_url.trim_end_matches('/')).expect("string serializes");
    let token = serde_json::to_string(token).expect("string serializes");
    let run_json = serde_json::json!({
        "id": run.id,
        "actionId": run.action_id,
        "actionName": run.action_name,
        "trigger": run.trigger,
        "queuedAt": run.queued_at,
    });
    format!(
        r#"const __citadelBaseUrl = {base_url};
const __citadelToken = {token};
const __citadelEndpointCatalog = {endpoint_catalog_json};
const args = {args};
const run = Object.freeze({run_json});

async function __citadelRequest(method, path, body) {{
  const normalizedPath = String(path || "");
  if (!normalizedPath.startsWith("/")) throw new Error("Citadel API path must start with '/'.");
  const response = await fetch(`${{__citadelBaseUrl}}${{normalizedPath}}`, {{
    method,
    headers: {{ authorization: `Bearer ${{__citadelToken}}`, "content-type": "application/json" }},
    body: body === undefined ? undefined : JSON.stringify(body)
  }});
  const text = await response.text();
  if (!response.ok) throw new Error(`Citadel API ${{method}} ${{normalizedPath}} failed: ${{response.status}} ${{text}}`);
  return text ? JSON.parse(text) : null;
}}

function __citadelAppendQuery(path, query) {{
  if (!query) return path;
  const search = new URLSearchParams();
  for (const [name, value] of Object.entries(query)) {{
    if (value === undefined || value === null) continue;
    if (Array.isArray(value)) {{
      for (const item of value) if (item !== undefined && item !== null) search.append(name, String(item));
    }} else search.append(name, String(value));
  }}
  const value = search.toString();
  return value ? `${{path}}?${{value}}` : path;
}}

function __citadelBuildOperation(endpoint) {{
  return (...operationArgs) => {{
    let index = 0;
    let path = endpoint.path.replace(/\{{([^}}:]+)(?::[^}}]+)?\}}/g, (_, name) => {{
      const value = operationArgs[index++];
      if (value === undefined || value === null || value === "") throw new Error(`Citadel API ${{endpoint.key}} requires path parameter '${{name}}'.`);
      return encodeURIComponent(String(value));
    }});
    const query = endpoint.method === "GET" ? operationArgs[index++] : undefined;
    const body = endpoint.method === "GET" ? undefined : operationArgs[index++];
    return __citadelRequest(endpoint.method, __citadelAppendQuery(path, query), body);
  }};
}}

const __citadelApi = {{}};
const __citadelGroups = {{}};
for (const endpoint of __citadelEndpointCatalog) {{
  const operation = __citadelBuildOperation(endpoint);
  __citadelApi[endpoint.key] = operation;
  (__citadelGroups[endpoint.group] ??= {{}})[endpoint.key] = operation;
}}
const citadel = Object.freeze({{
  ...__citadelGroups,
  api: Object.freeze(__citadelApi),
  request: __citadelRequest,
  get: (path) => __citadelRequest("GET", path),
  post: (path, body) => __citadelRequest("POST", path, body),
  patch: (path, body) => __citadelRequest("PATCH", path, body),
  put: (path, body) => __citadelRequest("PUT", path, body),
  delete: (path, body) => __citadelRequest("DELETE", path, body)
}});

{code}
"#,
        args = run.args_json,
        code = run.code_snapshot,
    )
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

fn redact_run_logs(value: &str, token: &str) -> String {
    if token.is_empty() {
        return redact_logs(value);
    }
    redact_logs(&value.replace(token, "[redacted]"))
}

pub fn redact_logs(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut redact_next = false;
    for segment in value.split_inclusive(char::is_whitespace) {
        let token_end = segment.find(char::is_whitespace).unwrap_or(segment.len());
        let (part, whitespace) = segment.split_at(token_end);
        let lower = part.to_ascii_lowercase();
        if redact_next {
            output.push_str("[redacted]");
            redact_next = false;
        } else if lower == "bearer" {
            output.push_str(part);
            redact_next = true;
        } else if lower.starts_with("token=")
            || lower.starts_with("api_key=")
            || lower.starts_with("apikey=")
            || lower.starts_with("password=")
            || lower.starts_with("secret=")
        {
            output.push_str(part.split_once('=').map_or(part, |(key, _)| key));
            output.push_str("=[redacted]");
        } else {
            output.push_str(part);
        }
        output.push_str(whitespace);
    }
    output
}

fn cron_is_due(expression: Option<&str>, time_zone: &str, now: DateTime<Utc>) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    let fields = expression.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return false;
    }
    let Ok(zone) = time_zone.parse::<Tz>() else {
        return false;
    };
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
    #[error("external operation failed: {0}")]
    External(String),
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

    #[test]
    fn cron_supports_steps_ranges_lists_and_time_zones() {
        let now = DateTime::parse_from_rfc3339("2026-07-14T08:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(cron_is_due(Some("30 10 * * 2"), "Europe/Paris", now));
        assert!(cron_is_due(Some("*/15 8-10 * * 1,2"), "UTC", now));
        assert!(!cron_is_due(Some("31 10 * * *"), "Europe/Paris", now));
        assert!(!cron_is_due(Some("* * *"), "UTC", now));
    }

    #[test]
    fn log_redaction_masks_common_credentials() {
        let output =
            redact_logs("Bearer abc.def token=secret api_key=value password=hunter2 secret=hidden");
        assert_eq!(
            output,
            "Bearer [redacted] token=[redacted] api_key=[redacted] password=[redacted] secret=[redacted]"
        );
        assert_eq!(
            redact_logs("first\nBearer secret\npassword=value\nlast"),
            "first\nBearer [redacted]\npassword=[redacted]\nlast"
        );
    }

    #[test]
    fn generated_script_exposes_typed_clients_without_persisting_the_token() {
        let now = Utc::now();
        let run = AutomationRunView {
            id: Uuid::now_v7(),
            action_id: Uuid::now_v7(),
            action_name: "test".to_owned(),
            trigger: "Manual".to_owned(),
            status: "Running".to_owned(),
            run_as_actor_id: Uuid::now_v7(),
            triggered_by_actor_id: None,
            args_json: r#"{"value":1}"#.to_owned(),
            code_snapshot: "console.log(args.value);".to_owned(),
            code_hash: "hash".to_owned(),
            timeout_seconds: 30,
            queued_at: now,
            started_at: Some(now),
            finished_at: None,
            duration_ms: None,
            exit_code: None,
            logs: None,
            error_message: None,
        };
        let source = automation_source(
            "http://127.0.0.1:8000/",
            "temporary-token",
            &run,
            r#"[{"key":"listPlatforms","group":"platforms","method":"GET","path":"/api/v1/platforms"}]"#,
        );
        assert!(source.contains("__citadelGroups[endpoint.group]"));
        assert!(source.contains("listPlatforms"));
        assert!(source.contains("temporary-token"));
        assert!(source.ends_with("console.log(args.value);\n"));
    }
}
