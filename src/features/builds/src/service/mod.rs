mod pool_health;
use crate::*;
mod execution;
mod pool_tests;
mod webhooks;

pub struct BuildService {
    preparation: Arc<tokio::sync::Semaphore>,
    cleanup: Option<crate::cleanup::BuildCleanupService>,
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
    pub fn with_preparation_slots(mut self, slots: usize) -> Self {
        self.preparation = Arc::new(tokio::sync::Semaphore::new(slots.clamp(1, 16)));
        self
    }

    pub fn try_preparation(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.preparation.clone().try_acquire_owned().ok()
    }

    pub async fn preparation_available(&self) {
        let _permit = self.preparation.acquire().await;
    }

    pub async fn claim_next(&self) -> Result<Option<BuildClaim>, BuildError> {
        let claim = self.store.claim_next(Utc::now() - self.stale_after).await?;
        if claim.is_some() {
            self.changed();
        }
        Ok(claim)
    }

    pub fn with_cleanup(
        mut self,
        store: Arc<dyn BuildCleanupStore>,
        checker: Arc<dyn BuildCleanupChecker>,
    ) -> Self {
        self.cleanup = Some(crate::cleanup::BuildCleanupService { store, checker });
        self
    }

    pub async fn retained_reservation(
        &self,
        run_id: Uuid,
    ) -> Result<Option<BuildRetainedReservation>, BuildError> {
        match &self.cleanup {
            Some(cleanup) => cleanup.store.retained_reservation(run_id).await,
            None => Ok(None),
        }
    }

    pub async fn resolve_cleanup(
        &self,
        run_id: Uuid,
        attempt_id: Uuid,
        actor: ActorId,
        reason: &str,
        acknowledged: bool,
    ) -> Result<(), BuildError> {
        let cleanup = self
            .cleanup
            .as_ref()
            .ok_or_else(|| BuildError::Conflict("Build recovery is unavailable.".into()))?;
        cleanup
            .resolve(
                run_id,
                attempt_id,
                actor,
                reason,
                acknowledged,
                &self.shutdown,
            )
            .await?;
        self.changed();
        if let Some(notify) = &self.pool_change {
            notify();
        }
        Ok(())
    }

    pub async fn reconcile_cleanup(
        &self,
        after: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<Option<Uuid>, BuildError> {
        let Some(cleanup) = &self.cleanup else {
            return Ok(None);
        };
        let (cursor, changed) = cleanup.process_batch(after, cancellation).await?;
        if changed {
            self.changed();
            if let Some(notify) = &self.pool_change {
                notify();
            }
        }
        Ok(cursor)
    }

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
            preparation: Arc::new(tokio::sync::Semaphore::new(2)),
            cleanup: None,
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
            let pool_id = project
                .build_agent_pool_id
                .ok_or_else(|| BuildError::Validation("A Build Pool is required.".into()))?;
            let pool = self.store.get_pool(pool_id).await?;
            pool_target(&pool.provider_spec)?;
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
