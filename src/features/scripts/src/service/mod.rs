mod scheduling;

mod direct;

mod execution;

pub(crate) mod source;

use crate::runs::progress;
use crate::service::source::script_source;
use citadel_primitives::schedule::schedule_is_due;

use std::collections::HashMap;

use std::path::PathBuf;

use std::sync::{Arc, Mutex};

use std::time::Duration;

use chrono::{DateTime, Utc};

use chrono::Timelike;

use citadel_alerts::{AlertEventSink, AlertObservation};

use citadel_primitives::ActorId;

use citadel_execution::ProcessRunner;

use citadel_primitives::WebhookConfig;

use serde_json::Value;

use tokio::sync::{Semaphore, mpsc};

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use crate::*;

pub struct ScriptService {
    sandbox: crate::sandbox::ScriptSandbox,
    tasks: Arc<dyn ScriptTaskSpawner>,
    shutdown: CancellationToken,
    on_change: Option<Arc<dyn Fn(Option<Uuid>) + Send + Sync>>,
    store: Arc<dyn ScriptRepository>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    internal_base_url: String,
    endpoint_catalog_json: Arc<str>,
    token_issuer: Arc<dyn ScriptRunTokenIssuer>,
    maximum_log_bytes: usize,
    stale_after: Duration,
    active_runs: Mutex<HashMap<Uuid, CancellationToken>>,
    slots: Arc<Semaphore>,
    remote_slots: Arc<Semaphore>,
    remote: Option<Arc<dyn RemoteScriptExecutor>>,
    remote_base_url: String,
    progress: tokio::sync::broadcast::Sender<ScriptProgress>,
    options: ScriptOptions,
    entitlements: Option<Arc<dyn ScriptEntitlements>>,
}

impl ScriptService {
    pub fn with_change_notifier(
        mut self,
        notifier: Arc<dyn Fn(Option<Uuid>) + Send + Sync>,
    ) -> Self {
        self.on_change = Some(notifier);
        self
    }
}

impl ScriptService {
    fn changed(&self, script_id: Option<Uuid>) {
        if let Some(notifier) = &self.on_change {
            notifier(script_id);
        }
    }
}

impl ScriptService {
    pub fn new(
        process: Arc<dyn ProcessRunner>,
        workspace: Arc<dyn crate::workspace::ScriptWorkspacePort>,
        tasks: Arc<dyn ScriptTaskSpawner>,
        shutdown: CancellationToken,
        store: Arc<dyn ScriptRepository>,
        token_issuer: Arc<dyn ScriptRunTokenIssuer>,
        config: ScriptRuntimeConfig,
    ) -> Self {
        Self {
            sandbox: crate::sandbox::ScriptSandbox {
                process,
                workspace,
                deno_path: config.deno_path,
                work_root: config.work_root,
                cache_directory: None,
                allow_net: None,
            },
            tasks,
            shutdown,
            store,
            on_change: None,
            alerts: None,
            internal_base_url: config.internal_base_url,
            endpoint_catalog_json: Arc::from(config.endpoint_catalog_json),
            token_issuer,
            maximum_log_bytes: config.maximum_log_bytes,
            stale_after: config.stale_after,
            active_runs: Mutex::new(HashMap::new()),
            slots: Arc::new(Semaphore::new(4)),
            remote_slots: Arc::new(Semaphore::new(16)),
            remote: None,
            remote_base_url: String::new(),
            progress: tokio::sync::broadcast::channel(128).0,
            options: ScriptOptions::default(),
            entitlements: None,
        }
    }
}

impl ScriptService {
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }
}

impl ScriptService {
    pub fn with_sandbox(mut self, cache_directory: PathBuf, allow_net: Option<String>) -> Self {
        self.sandbox.cache_directory = Some(cache_directory);
        self.sandbox.allow_net = allow_net;
        self
    }
}

impl ScriptService {
    pub fn with_options(mut self, options: ScriptOptions) -> Result<Self, ScriptError> {
        options.validate()?;
        self.slots = Arc::new(Semaphore::new(options.max_parallel_runs));
        self.remote_slots = Arc::new(Semaphore::new(options.max_remote_runs));
        self.options = options;
        Ok(self)
    }
}

impl ScriptService {
    pub fn with_remote(mut self, remote: Arc<dyn RemoteScriptExecutor>, base_url: String) -> Self {
        self.remote = Some(remote);
        self.remote_base_url = base_url;
        self
    }
    pub async fn reconcile_remote(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), ScriptError> {
        if let Some(remote) = &self.remote
            && remote.reconcile(cancellation).await?
        {
            self.changed(None);
        }
        Ok(())
    }
    pub async fn resolve_cleanup(
        &self,
        script_id: Uuid,
        run_id: Uuid,
        attempt_id: Uuid,
        actor: ActorId,
        reason: &str,
        acknowledged: bool,
    ) -> Result<(), ScriptError> {
        if !acknowledged || reason.trim().is_empty() || reason.chars().count() > 1000 {
            return Err(ScriptError::Validation(
                "Confirm that the old script has stopped and provide a reason (1–1000 characters)."
                    .into(),
            ));
        }
        let run = self.store.get_run(script_id, run_id).await?;
        if !run.cleanup_pending
            || run.attempt_id != Some(attempt_id)
            || matches!(
                run.status,
                ScriptRunStatus::Queued | ScriptRunStatus::Running
            )
        {
            return Err(ScriptError::Conflict(
                "This attempt is not awaiting cleanup. Refresh the run.".into(),
            ));
        }
        let remote = self
            .remote
            .as_ref()
            .ok_or_else(|| ScriptError::Conflict("Remote execution is unavailable.".into()))?;
        remote
            .resolve(&run, actor, reason.trim(), &self.shutdown)
            .await?;
        self.changed(Some(script_id));
        Ok(())
    }
    pub fn options(&self) -> ScriptOptions {
        self.options
    }
}

impl ScriptService {
    pub fn validate_input(
        &self,
        input: &mut ScriptConfiguration,
        actor: ActorId,
    ) -> Result<(), ScriptError> {
        let timeout = input
            .timeout_seconds
            .unwrap_or(self.options.default_timeout_seconds);
        self.options.validate_timeout(timeout)?;
        input.timeout_seconds = Some(timeout);
        input.validate(actor)
    }
}

impl ScriptService {
    pub fn with_entitlements(mut self, entitlements: Arc<dyn ScriptEntitlements>) -> Self {
        self.entitlements = Some(entitlements);
        self
    }
}

impl ScriptService {
    pub async fn ensure_automated_operations(&self) -> Result<(), ScriptError> {
        if let Some(entitlements) = &self.entitlements
            && entitlements.automated_operations().await?
        {
            return Ok(());
        }
        Err(ScriptError::LicenseRequired)
    }
}

impl ScriptService {
    pub fn store(&self) -> &Arc<dyn ScriptRepository> {
        &self.store
    }
}
