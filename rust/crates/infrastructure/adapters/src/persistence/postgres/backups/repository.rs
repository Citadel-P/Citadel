#[derive(Clone)]
pub struct PostgresBackupPersistence {
    pub(super) pool: PgPool,
    pub(super) repository_lease_seconds: i32,
    pub(super) source_lease_seconds: i32,
    pub(super) restore_timeout_seconds: i32,
}

impl PostgresBackupPersistence {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            repository_lease_seconds: 300,
            source_lease_seconds: 300,
            restore_timeout_seconds: 14_400,
        }
    }
    pub fn with_lease_options(
        mut self,
        repository_seconds: i32,
        source_seconds: i32,
        restore_seconds: i32,
    ) -> Self {
        self.repository_lease_seconds = repository_seconds.max(30);
        self.source_lease_seconds = source_seconds.max(30);
        self.restore_timeout_seconds = restore_seconds.max(5);
        self
    }
}
use super::*;

impl BackupPersistence for PostgresBackupPersistence {
    fn create_repository<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupRepositoryConfiguration,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        self.create_repository_impl(actor, input)
    }

    fn list_repositories(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupRepository>, BackupError>> {
        self.list_repositories_impl(actor, administrator)
    }

    fn get_repository(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRepository, BackupError>> {
        self.get_repository_impl(id)
    }

    fn update_repository<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        self.update_repository_impl(id, patch)
    }

    fn archive_repository(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        self.archive_repository_impl(id)
    }

    fn record_repository_operation<'a>(
        &'a self,
        id: Uuid,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        succeeded: bool,
        message: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(BackupRepository, BackupRepositoryValidation), BackupError>> {
        self.record_repository_operation_impl(
            id,
            operation,
            location,
            platform_id,
            succeeded,
            message,
        )
    }

    fn acquire_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
        operation: &str,
        expires_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.acquire_repository_operation_impl(repository_id, operation_id, operation, expires_at)
    }

    fn release_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>> {
        self.release_repository_operation_impl(repository_id, operation_id)
    }

    fn create_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.create_policy_impl(actor, input)
    }

    fn list_policies(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>> {
        self.list_policies_impl(actor, administrator)
    }

    fn get_policy(&self, id: Uuid) -> BoxFuture<'_, Result<BackupPolicy, BackupError>> {
        self.get_policy_impl(id)
    }

    fn platform_summaries<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<summaries::PlatformBackupSummary>, BackupError>> {
        self.platform_summaries_impl(actor, administrator, platform_ids)
    }

    fn rename_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a policy_metadata::RenameBackupPolicyInput,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.rename_policy_impl(actor, input)
    }

    fn update_policy<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        row_version: i64,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.update_policy_impl(actor, id, row_version, input)
    }

    fn update_policy_description<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        self.update_policy_description_impl(actor, id, description)
    }

    fn archive_policy(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        self.archive_policy_impl(id)
    }

    fn enqueue_backup(
        &self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
    ) -> BoxFuture<'_, Result<BackupRun, BackupError>> {
        self.enqueue_backup_impl(actor, policy_id, trigger)
    }

    fn enqueue_webhook<'a>(
        &'a self,
        policy_id: Uuid,
        expected_webhook: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRun, BackupError>> {
        self.enqueue_webhook_impl(policy_id, expected_webhook)
    }

    fn list_scheduled_policies(&self) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>> {
        self.list_scheduled_policies_impl()
    }

    fn enqueue_scheduled_backup(
        &self,
        policy_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.enqueue_scheduled_backup_impl(policy_id, scheduled_minute)
    }

    fn claim_backup(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<BackupClaim>, BackupError>> {
        self.claim_backup_impl(stale_before)
    }

    fn prepare_backup_items<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        self.prepare_backup_items_impl(claim, plan)
    }

    fn finish_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        result: &'a BackupExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        self.finish_backup_impl(claim, result)
    }

    fn list_runs(
        &self,
        actor: ActorId,
        administrator: bool,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRun>, BackupError>> {
        self.list_runs_impl(actor, administrator, policy_id, limit)
    }

    fn get_run(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRun, BackupError>> {
        self.get_run_impl(id)
    }

    fn backup_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        self.backup_logs_impl(id)
    }

    fn cancel_backup(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.cancel_backup_impl(id)
    }

    fn enqueue_restore(
        &self,
        request: BackupRestoreRequest,
    ) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>> {
        self.enqueue_restore_impl(request)
    }

    fn claim_restore(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<RestoreClaim>, BackupError>> {
        self.claim_restore_impl(stale_before)
    }

    fn finish_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        result: &'a RestoreExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        self.finish_restore_impl(claim, result)
    }

    fn list_restores(
        &self,
        actor: ActorId,
        administrator: bool,
        backup_run_id: Option<Uuid>,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRestoreRun>, BackupError>> {
        self.list_restores_impl(actor, administrator, backup_run_id, policy_id, limit)
    }

    fn get_restore(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>> {
        self.get_restore_impl(id)
    }

    fn restore_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        self.restore_logs_impl(id)
    }

    fn cancel_restore(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        self.cancel_restore_impl(id)
    }
}
