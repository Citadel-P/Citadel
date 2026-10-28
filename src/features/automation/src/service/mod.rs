mod scheduling;

mod direct;

mod execution;

pub(crate) mod source;

use crate::runs::progress;
use crate::service::source::automation_source;
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

pub struct AutomationService {
    sandbox: crate::sandbox::AutomationSandbox,
    tasks: Arc<dyn AutomationTaskSpawner>,
    shutdown: CancellationToken,
    on_change: Option<Arc<dyn Fn(Option<Uuid>) + Send + Sync>>,
    store: Arc<dyn AutomationRepository>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    internal_base_url: String,
    endpoint_catalog_json: Arc<str>,
    token_issuer: Arc<dyn AutomationRunTokenIssuer>,
    maximum_log_bytes: usize,
    stale_after: Duration,
    active_runs: Mutex<HashMap<Uuid, CancellationToken>>,
    slots: Arc<Semaphore>,
    remote_slots: Arc<Semaphore>,
    remote: Option<Arc<dyn RemoteAutomationExecutor>>,
    remote_base_url: String,
    progress: tokio::sync::broadcast::Sender<AutomationProgress>,
    options: AutomationOptions,
    entitlements: Option<Arc<dyn AutomationEntitlements>>,
}

impl AutomationService {
    pub fn with_change_notifier(
        mut self,
        notifier: Arc<dyn Fn(Option<Uuid>) + Send + Sync>,
    ) -> Self {
        self.on_change = Some(notifier);
        self
    }
}

impl AutomationService {
    fn changed(&self, action_id: Option<Uuid>) {
        if let Some(notifier) = &self.on_change {
            notifier(action_id);
        }
    }
}

impl AutomationService {
    pub fn new(
        process: Arc<dyn ProcessRunner>,
        workspace: Arc<dyn crate::workspace::AutomationWorkspacePort>,
        tasks: Arc<dyn AutomationTaskSpawner>,
        shutdown: CancellationToken,
        store: Arc<dyn AutomationRepository>,
        token_issuer: Arc<dyn AutomationRunTokenIssuer>,
        config: AutomationRuntimeConfig,
    ) -> Self {
        Self {
            sandbox: crate::sandbox::AutomationSandbox {
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
            options: AutomationOptions::default(),
            entitlements: None,
        }
    }
}

impl AutomationService {
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }
}

impl AutomationService {
    pub fn with_sandbox(mut self, cache_directory: PathBuf, allow_net: Option<String>) -> Self {
        self.sandbox.cache_directory = Some(cache_directory);
        self.sandbox.allow_net = allow_net;
        self
    }
}

impl AutomationService {
    pub fn with_options(mut self, options: AutomationOptions) -> Result<Self, AutomationError> {
        options.validate()?;
        self.slots = Arc::new(Semaphore::new(options.max_parallel_runs));
        self.remote_slots = Arc::new(Semaphore::new(options.max_remote_runs));
        self.options = options;
        Ok(self)
    }
}

impl AutomationService {
    pub fn with_remote(
        mut self,
        remote: Arc<dyn RemoteAutomationExecutor>,
        base_url: String,
    ) -> Self {
        self.remote = Some(remote);
        self.remote_base_url = base_url;
        self
    }
    pub async fn reconcile_remote(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), AutomationError> {
        if let Some(remote) = &self.remote
            && remote.reconcile(cancellation).await?
        {
            self.changed(None);
        }
        Ok(())
    }
    pub async fn resolve_cleanup(
        &self,
        action_id: Uuid,
        run_id: Uuid,
        attempt_id: Uuid,
        actor: ActorId,
        reason: &str,
        acknowledged: bool,
    ) -> Result<(), AutomationError> {
        if !acknowledged || reason.trim().is_empty() || reason.chars().count() > 1000 {
            return Err(AutomationError::Validation(
                "Confirm that the old script has stopped and provide a reason (1–1000 characters)."
                    .into(),
            ));
        }
        let run = self.store.get_run(action_id, run_id).await?;
        if !run.cleanup_pending
            || run.attempt_id != Some(attempt_id)
            || matches!(
                run.status,
                AutomationRunStatus::Queued | AutomationRunStatus::Running
            )
        {
            return Err(AutomationError::Conflict(
                "This attempt is not awaiting cleanup. Refresh the run.".into(),
            ));
        }
        let remote = self
            .remote
            .as_ref()
            .ok_or_else(|| AutomationError::Conflict("Remote execution is unavailable.".into()))?;
        remote
            .resolve(&run, actor, reason.trim(), &self.shutdown)
            .await?;
        self.changed(Some(action_id));
        Ok(())
    }
    pub fn options(&self) -> AutomationOptions {
        self.options
    }
}

impl AutomationService {
    pub fn validate_input(
        &self,
        input: &mut AutomationActionConfiguration,
        actor: ActorId,
    ) -> Result<(), AutomationError> {
        let timeout = input
            .timeout_seconds
            .unwrap_or(self.options.default_timeout_seconds);
        self.options.validate_timeout(timeout)?;
        input.timeout_seconds = Some(timeout);
        input.validate(actor)
    }
}

impl AutomationService {
    pub fn with_entitlements(mut self, entitlements: Arc<dyn AutomationEntitlements>) -> Self {
        self.entitlements = Some(entitlements);
        self
    }
}

impl AutomationService {
    pub async fn ensure_automated_operations(&self) -> Result<(), AutomationError> {
        if let Some(entitlements) = &self.entitlements
            && entitlements.automated_operations().await?
        {
            return Ok(());
        }
        Err(AutomationError::LicenseRequired)
    }
}

impl AutomationService {
    pub fn store(&self) -> &Arc<dyn AutomationRepository> {
        &self.store
    }
}
