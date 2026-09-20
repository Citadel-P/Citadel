#[derive(Clone)]
pub struct PostgresBuildRepository {
    pub(super) pool: PgPool,
}

impl PostgresBuildRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
use super::*;

impl BuildRepository for PostgresBuildRepository {
    fn health_pools(
        &self,
        after: Uuid,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>> {
        self.health_pools_impl(after, limit)
    }

    fn record_pool_health<'a>(
        &'a self,
        pool: &'a BuildAgentPool,
        result: &'a citadel_builds::BuildPoolCheck,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        self.record_pool_health_impl(pool, result)
    }

    fn project_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    > {
        self.project_permissions_impl(actor, ids)
    }

    fn create_pool<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildAgentPoolConfiguration,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.create_pool_impl(actor, input)
    }

    fn list_pools(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>> {
        self.list_pools_impl(actor, administrator)
    }

    fn pool_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    > {
        self.pool_permissions_impl(actor, ids)
    }

    fn get_pool<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.get_pool_impl(id)
    }

    fn claim_pool_test(
        &self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'_, Result<BuildAgentPool, BuildError>> {
        self.claim_pool_test_impl(id, actor)
    }

    fn finish_pool_test<'a>(
        &'a self,
        claim: &'a BuildAgentPool,
        result: &'a citadel_builds::BuildPoolCheck,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.finish_pool_test_impl(claim, result)
    }

    fn update_pool<'a>(
        &'a self,
        current: &'a BuildAgentPool,
        input: &'a BuildAgentPoolConfiguration,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.update_pool_impl(current, input, actor)
    }

    fn archive_pool<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        self.archive_pool_impl(id, actor)
    }

    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildProjectConfiguration,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        self.create_impl(actor, input)
    }

    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildProject>, BuildError>> {
        self.list_impl(actor, administrator)
    }

    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        self.get_impl(id)
    }

    fn update<'a>(
        &'a self,
        current: &'a BuildProject,
        input: &'a BuildProjectConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        self.update_impl(current, input, actor, metadata_only)
    }

    fn archive<'a>(&'a self, id: Uuid, actor: ActorId) -> BoxFuture<'a, Result<(), BuildError>> {
        self.archive_impl(id, actor)
    }

    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.enqueue_impl(actor, id, trigger)
    }

    fn enqueue_webhook<'a>(
        &'a self,
        current: &'a BuildProject,
        branch: &'a str,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.enqueue_webhook_impl(current, branch, commit)
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<BuildClaim>, BuildError>> {
        self.claim_next_impl(stale_before)
    }

    fn finish<'a>(
        &'a self,
        claim: &'a BuildClaim,
        result: &'a BuildExecutionResult,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        self.finish_impl(claim, result)
    }

    fn list_runs<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<BuildRun>, BuildError>> {
        self.list_runs_impl(actor, administrator, project_id, limit)
    }

    fn get_run<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.get_run_impl(id)
    }

    fn logs<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<citadel_builds::BuildLogEntry>, BuildError>> {
        self.logs_impl(id)
    }

    fn cancel_queued<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<bool, BuildError>> {
        self.cancel_queued_impl(id)
    }

    fn append_log<'a>(
        &'a self,
        run_id: Uuid,
        log: &'a citadel_builds::BuildLog,
    ) -> BoxFuture<'a, Result<citadel_builds::BuildLogEntry, BuildError>> {
        self.append_log_impl(run_id, log)
    }
}
