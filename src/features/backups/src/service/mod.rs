use crate::runs::progress;
use crate::*;
mod backups;
mod cancellation;
mod completion;
mod leases;
mod repositories;
mod restores;
mod scheduling;
#[cfg(test)]
mod tests;

pub struct BackupService {
    tracking: Arc<tokio::sync::Semaphore>,
    progress: tokio::sync::broadcast::Sender<Arc<progress::BackupProgressItem>>,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    store: Arc<dyn BackupPersistence>,
    executor: Arc<dyn BackupExecutor>,
    planner: Arc<dyn BackupSourcePlanner>,
    active_backups: Mutex<HashMap<Uuid, CancellationToken>>,
    active_restores: Mutex<HashMap<Uuid, CancellationToken>>,
    stale_after: chrono::Duration,
    repository_operation_lease: chrono::Duration,
    authorizer: Arc<dyn BackupRunAuthorizer>,
    entitlements: Option<Arc<dyn BackupEntitlements>>,
    scheduling_enabled: bool,
}

/// A durable claim owns its tracking permit until execution and completion finish.
/// Only the service can construct it; dispatchers transfer it into an owned task.
pub struct ClaimedBackupWork {
    claim: BackupWorkClaim,
    _tracking: tokio::sync::OwnedSemaphorePermit,
}

enum BackupWorkClaim {
    Backup(Box<BackupClaim>),
    Restore(Box<RestoreClaim>),
}

fn rejected_backup(message: String) -> BackupExecutionResult {
    failed_before_execution("AuthorizationRevoked", message)
}

fn failed_before_execution(code: &str, message: String) -> BackupExecutionResult {
    BackupExecutionResult {
        status: crate::BackupRunStatus::Failed,
        snapshot_availability: crate::BackupSnapshotAvailability::NotCreated,
        restic_snapshot_id: None,
        parent_snapshot_id: None,
        files_processed: None,
        bytes_processed: None,
        bytes_added: None,
        exit_code: None,
        error_code: Some(code.to_owned()),
        error_message: Some(message.clone()),
        warnings: vec![],
        logs: vec![BackupLog {
            stream: "stderr".into(),
            message,
        }],
        items: vec![],
    }
}

fn rejected_restore(message: String) -> RestoreExecutionResult {
    RestoreExecutionResult {
        status: crate::BackupRestoreStatus::Failed,
        exit_code: None,
        error_code: Some("AuthorizationRevoked".to_owned()),
        error_message: Some(message.clone()),
        logs: vec![BackupLog {
            stream: "stderr".into(),
            message,
        }],
    }
}

fn poison<T>(_: std::sync::PoisonError<T>) -> BackupError {
    BackupError::Storage("Backup cancellation state is poisoned.".into())
}

impl BackupService {
    pub async fn execute_claim(
        &self,
        work: ClaimedBackupWork,
        shutdown: &CancellationToken,
    ) -> Result<bool, BackupError> {
        let ClaimedBackupWork { claim, _tracking } = work;
        match claim {
            BackupWorkClaim::Backup(claim) => self.execute_backup(*claim, shutdown).await,
            BackupWorkClaim::Restore(claim) => self.execute_restore(*claim, shutdown).await,
        }
    }

    pub fn with_tracking_slots(mut self, slots: usize) -> Self {
        self.tracking = Arc::new(tokio::sync::Semaphore::new(slots.clamp(1, 128)));
        self
    }
    pub(super) async fn tracking_slot(
        &self,
        shutdown: &CancellationToken,
    ) -> Option<tokio::sync::OwnedSemaphorePermit> {
        tokio::select! { biased; () = shutdown.cancelled() => None, permit = self.tracking.clone().acquire_owned() => permit.ok() }
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
        store: Arc<dyn BackupPersistence>,
        executor: Arc<dyn BackupExecutor>,
        planner: Arc<dyn BackupSourcePlanner>,
        stale_after: chrono::Duration,
        authorizer: Arc<dyn BackupRunAuthorizer>,
    ) -> Self {
        Self {
            tracking: Arc::new(tokio::sync::Semaphore::new(16)),
            progress: tokio::sync::broadcast::channel(256).0,
            store,
            on_change: None,
            executor,
            planner,
            active_backups: Mutex::new(HashMap::new()),
            active_restores: Mutex::new(HashMap::new()),
            stale_after,
            repository_operation_lease: chrono::Duration::minutes(16),
            authorizer,
            entitlements: None,
            scheduling_enabled: true,
        }
    }

    pub fn with_repository_operation_lease(mut self, lease: chrono::Duration) -> Self {
        self.repository_operation_lease = lease;
        self
    }

    pub async fn reconcile_cleanup(
        &self,
        after: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<Option<Uuid>, BackupError> {
        let next = self.executor.reconcile_cleanup(after, cancellation).await?;
        self.changed();
        Ok(next)
    }

    pub fn store(&self) -> &Arc<dyn BackupPersistence> {
        &self.store
    }

    pub fn planner(&self) -> &Arc<dyn BackupSourcePlanner> {
        &self.planner
    }

    pub fn subscribe_progress(
        &self,
    ) -> tokio::sync::broadcast::Receiver<Arc<progress::BackupProgressItem>> {
        self.progress.subscribe()
    }

    pub(super) fn progress(&self, id: Uuid, status: &str, message: &str) {
        let _ = self
            .progress
            .send(Arc::new(progress::BackupProgressItem::message(
                id, status, message,
            )));
    }

    pub(super) fn log_progress(&self, id: Uuid) -> Arc<dyn Fn(BackupLog) + Send + Sync> {
        let sender = self.progress.clone();
        Arc::new(move |log| {
            let _ = sender.send(Arc::new(progress::BackupProgressItem {
                run_id: id,
                status: None,
                message: Some(log.message),
                stream: Some(log.stream),
                exit_code: None,
            }));
        })
    }

    pub fn with_entitlements(mut self, entitlements: Arc<dyn BackupEntitlements>) -> Self {
        self.entitlements = Some(entitlements);
        self
    }

    pub fn with_scheduling_enabled(mut self, enabled: bool) -> Self {
        self.scheduling_enabled = enabled;
        self
    }

    pub async fn scheduling_available(&self) -> Result<bool, BackupError> {
        if !self.scheduling_enabled {
            return Ok(false);
        }
        match self.ensure_automated_operations().await {
            Ok(()) => Ok(true),
            Err(BackupError::LicenseRequired) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub async fn ensure_automated_operations(&self) -> Result<(), BackupError> {
        if let Some(entitlements) = &self.entitlements
            && entitlements.automated_operations().await?
        {
            return Ok(());
        }
        Err(BackupError::LicenseRequired)
    }
}
