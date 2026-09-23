mod pool_health;
use crate::*;
mod execution;
mod pool_tests;
mod webhooks;

pub struct BuildService {
    tasks: Arc<dyn BuildTaskSpawner>,
    shutdown: CancellationToken,
    entitlements: Option<Arc<dyn BuildEntitlements>>,
    pool_change: Option<Arc<dyn Fn() + Send + Sync>>,
    pool_checker: Option<Arc<dyn BuildPoolChecker>>,
    on_log: Option<BuildLogNotifier>,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    store: Arc<dyn BuildRepository>,
    executor: Arc<dyn BuildExecutor>,
    active: Mutex<std::collections::HashMap<Uuid, CancellationToken>>,
    stale_after: chrono::Duration,
    alerts: Option<Arc<dyn AlertEventSink>>,
}

impl BuildEntitlements for BuildService {
    fn enabled(
        &self,
        capability: citadel_licensing::LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, BuildError>> {
        Box::pin(async move {
            match &self.entitlements {
                Some(entitlements) => entitlements.enabled(capability).await,
                None => Ok(false),
            }
        })
    }
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

    pub(super) fn changed(&self) {
        if let Some(notifier) = &self.on_change {
            notifier();
        }
    }

    pub fn new(
        tasks: Arc<dyn BuildTaskSpawner>,
        shutdown: CancellationToken,
        store: Arc<dyn BuildRepository>,
        executor: Arc<dyn BuildExecutor>,
        stale_after: chrono::Duration,
    ) -> Self {
        Self {
            tasks,
            shutdown,
            entitlements: None,
            pool_change: None,
            pool_checker: None,
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

    pub fn with_entitlements(mut self, entitlements: Arc<dyn BuildEntitlements>) -> Self {
        self.entitlements = Some(entitlements);
        self
    }

    pub async fn ensure_entitled(
        &self,
        capability: citadel_licensing::LicenseCapability,
    ) -> Result<(), BuildError> {
        if let Some(entitlements) = &self.entitlements
            && entitlements.enabled(capability).await?
        {
            return Ok(());
        }
        Err(BuildError::LicenseRequired(capability))
    }

    pub async fn ensure_execution_entitlements(
        &self,
        project: &BuildProject,
        trigger: &str,
    ) -> Result<(), BuildError> {
        if trigger != "Manual" {
            self.ensure_entitled(citadel_licensing::LicenseCapability::AutomatedOperations)
                .await?;
        }
        if project.builder_kind == "BuildAgentPool" {
            self.ensure_entitled(citadel_licensing::LicenseCapability::ElasticBuildExecution)
                .await?;
        }
        Ok(())
    }

    pub fn store(&self) -> &Arc<dyn BuildRepository> {
        &self.store
    }

    pub fn with_pool_checker(mut self, checker: Arc<dyn BuildPoolChecker>) -> Self {
        self.pool_checker = Some(checker);
        self
    }

    pub fn with_pool_change_notifier(mut self, notifier: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.pool_change = Some(notifier);
        self
    }
}
