use crate::*;

pub trait BackupPersistence: Send + Sync {
    fn create_repository<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupRepositoryConfiguration,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>>;
    fn list_repositories(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupRepository>, BackupError>>;
    fn get_repository(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRepository, BackupError>>;
    fn update_repository<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>>;
    fn archive_repository(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>>;
    fn record_repository_operation<'a>(
        &'a self,
        id: Uuid,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        succeeded: bool,
        message: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(BackupRepository, BackupRepositoryValidation), BackupError>>;
    fn acquire_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
        operation: &str,
        expires_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>>;
    fn release_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>>;
    fn create_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>>;
    fn list_policies(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>>;
    fn get_policy(&self, id: Uuid) -> BoxFuture<'_, Result<BackupPolicy, BackupError>>;
    fn update_policy<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        row_version: i64,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>>;
    fn platform_summaries<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<runs::read_models::PlatformBackupSummary>, BackupError>>;
    fn rename_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a policies::metadata::RenameBackupPolicyInput,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>>;
    fn update_policy_description<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>>;
    fn archive_policy(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>>;
    fn enqueue_backup(
        &self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
    ) -> BoxFuture<'_, Result<BackupRun, BackupError>>;
    fn enqueue_webhook<'a>(
        &'a self,
        policy_id: Uuid,
        expected_webhook: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRun, BackupError>>;
    fn list_scheduled_policies(&self) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>>;
    fn enqueue_scheduled_backup(
        &self,
        policy_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>>;
    fn claim_backup(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<BackupClaim>, BackupError>>;
    fn prepare_backup_items<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn finish_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        result: &'a BackupExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn list_runs(
        &self,
        actor: ActorId,
        administrator: bool,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRun>, BackupError>>;
    fn get_run(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRun, BackupError>>;
    fn backup_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>>;
    fn cancel_backup(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>>;
    fn enqueue_restore(
        &self,
        request: BackupRestoreRequest,
    ) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>>;
    fn claim_restore(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<RestoreClaim>, BackupError>>;
    fn finish_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        result: &'a RestoreExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>>;
    fn list_restores(
        &self,
        actor: ActorId,
        administrator: bool,
        backup_run_id: Option<Uuid>,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRestoreRun>, BackupError>>;
    fn get_restore(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>>;
    fn restore_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>>;
    fn cancel_restore(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>>;
}
