mod scheduling;

mod direct;

mod execution;

pub(crate) mod source;

use crate::{
    jobs::schedule::cron_is_due,
    runs::{logs::*, progress},
    service::source::automation_source,
};

use std::collections::HashMap;

use std::ffi::OsString;

use std::path::PathBuf;

use std::sync::{Arc, Mutex};

use std::time::Duration;

use chrono::{DateTime, Utc};

use chrono::Timelike;

use citadel_alerts::{AlertEventSink, AlertObservation};

use citadel_primitives::ActorId;

use citadel_execution::{
    OutputLimitPolicy, ProcessError, ProcessLimits, ProcessRequest, ProcessRunner,
};

use citadel_git::repositories::webhooks::RepoWebhookConfig;

use serde_json::Value;

use tokio::sync::{Semaphore, mpsc};

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use crate::*;

pub struct AutomationService {
    process: Arc<dyn ProcessRunner>,
    tasks: Arc<dyn AutomationTaskSpawner>,
    shutdown: CancellationToken,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    store: Arc<dyn AutomationRepository>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    deno_path: OsString,
    work_root: PathBuf,
    cache_directory: Option<PathBuf>,
    allow_net: Option<String>,
    internal_base_url: String,
    endpoint_catalog_json: Arc<str>,
    token_issuer: Arc<dyn AutomationRunTokenIssuer>,
    maximum_log_bytes: usize,
    stale_after: Duration,
    active_runs: Mutex<HashMap<Uuid, CancellationToken>>,
    slots: Arc<Semaphore>,
    options: AutomationOptions,
    entitlements: Option<Arc<dyn AutomationEntitlements>>,
}

impl AutomationService {
    pub fn with_change_notifier(mut self, notifier: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.on_change = Some(notifier);
        self
    }
}

impl AutomationService {
    fn changed(&self) {
        if let Some(notifier) = &self.on_change {
            notifier();
        }
    }
}

impl AutomationService {
    pub fn new(
        process: Arc<dyn ProcessRunner>,
        tasks: Arc<dyn AutomationTaskSpawner>,
        shutdown: CancellationToken,
        store: Arc<dyn AutomationRepository>,
        token_issuer: Arc<dyn AutomationRunTokenIssuer>,
        config: AutomationRuntimeConfig,
    ) -> Self {
        Self {
            process,
            tasks,
            shutdown,
            store,
            on_change: None,
            alerts: None,
            deno_path: config.deno_path,
            work_root: config.work_root,
            cache_directory: None,
            allow_net: None,
            internal_base_url: config.internal_base_url,
            endpoint_catalog_json: Arc::from(config.endpoint_catalog_json),
            token_issuer,
            maximum_log_bytes: config.maximum_log_bytes,
            stale_after: config.stale_after,
            active_runs: Mutex::new(HashMap::new()),
            slots: Arc::new(Semaphore::new(4)),
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
        self.cache_directory = Some(cache_directory);
        self.allow_net = allow_net;
        self
    }
}

impl AutomationService {
    pub fn with_options(mut self, options: AutomationOptions) -> Result<Self, AutomationError> {
        options.validate()?;
        self.slots = Arc::new(Semaphore::new(options.max_parallel_runs));
        self.options = options;
        Ok(self)
    }
}

impl AutomationService {
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
    pub async fn ensure_paid_trigger(&self) -> Result<(), AutomationError> {
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
