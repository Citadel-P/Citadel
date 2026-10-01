use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSourceItem {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub volume_name: String,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
}

impl BackupSourceItem {
    #[must_use]
    pub fn new(
        platform_id: Uuid,
        volume_name: String,
        docker_node_id: Option<String>,
        node_hostname: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            platform_id,
            volume_name,
            docker_node_id,
            node_hostname,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSourcePlan {
    pub display_name: String,
    pub items: Vec<BackupSourceItem>,
    pub warnings: Vec<String>,
    pub local_directory: Option<PathBuf>,
}

pub trait CitadelSystemBackupBuilder: Send + Sync {
    fn build<'a>(
        &'a self,
        run_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PathBuf, BackupError>>;
}

pub trait BackupExecutor: Send + Sync {
    fn backup_with_progress<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        cancellation: &'a CancellationToken,
        _progress: Arc<dyn Fn(BackupLog) + Send + Sync>,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        self.backup(claim, plan, cancellation)
    }
    fn restore_with_progress<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        cancellation: &'a CancellationToken,
        _progress: Arc<dyn Fn(BackupLog) + Send + Sync>,
    ) -> BoxFuture<'a, RestoreExecutionResult> {
        self.restore(claim, cancellation)
    }
    fn repository<'a>(
        &'a self,
        repository: &'a BackupRepository,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>>;
    fn backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult>;
    fn restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, RestoreExecutionResult>;
}

pub trait BackupSourcePlanner: Send + Sync {
    fn cleanup_staging<'a>(
        &'a self,
        plan: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn preview<'a>(
        &'a self,
        kind: policies::read_models::BackupPreviewKind,
        id: Uuid,
        actor: ActorId,
        administrator: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<policies::read_models::BackupSourcePreview, BackupError>>;
    fn validate_source<'a>(
        &'a self,
        source: &'a crate::spec::BackupSourceSpec,
        repository: &'a BackupRepository,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn plan<'a>(
        &'a self,
        claim: &'a BackupClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BackupSourcePlan, BackupError>>;
}

pub trait BackupSecretResolver: Send + Sync {
    fn resolve(&self, id: Uuid) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BackupError>>;
}

pub trait BackupRunAuthorizer: Send + Sync {
    fn authorize_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn authorize_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
}

pub trait BackupEntitlements: Send + Sync {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, BackupError>>;
}
