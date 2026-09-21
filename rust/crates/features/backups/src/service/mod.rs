use crate::runs::progress;
use crate::*;
mod backups;
mod cancellation;
mod repositories;
mod restores;
mod scheduling;

pub struct BackupService {
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

fn poison<T>(_: std::sync::PoisonError<T>) -> BackupError {
    BackupError::Storage("Backup cancellation state is poisoned.".into())
}

impl BackupService {
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
        }
    }

    pub fn with_repository_operation_lease(mut self, lease: chrono::Duration) -> Self {
        self.repository_operation_lease = lease;
        self
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

    pub async fn ensure_automated_operations(&self) -> Result<(), BackupError> {
        if let Some(entitlements) = &self.entitlements
            && entitlements.automated_operations().await?
        {
            return Ok(());
        }
        Err(BackupError::LicenseRequired)
    }
}
