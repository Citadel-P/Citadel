use crate::*;

pub trait BuildRepository: Send + Sync {
    fn health_pools(
        &self,
        after: Uuid,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>>;
    fn record_pool_health<'a>(
        &'a self,
        pool: &'a BuildAgentPool,
        result: &'a BuildPoolCheck,
    ) -> BoxFuture<'a, Result<bool, BuildError>>;
    fn project_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    >;
    fn create_pool<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildAgentPoolConfiguration,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>>;
    fn list_pools(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>>;
    fn get_pool<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>>;
    fn pool_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    >;
    fn archive_pool<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), BuildError>>;
    fn update_pool<'a>(
        &'a self,
        current: &'a BuildAgentPool,
        input: &'a BuildAgentPoolConfiguration,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>>;
    fn claim_pool_test(
        &self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'_, Result<BuildAgentPool, BuildError>>;
    fn finish_pool_test<'a>(
        &'a self,
        claim: &'a BuildAgentPool,
        result: &'a BuildPoolCheck,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>>;
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildProjectConfiguration,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>>;
    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildProject>, BuildError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildProject, BuildError>>;
    fn update<'a>(
        &'a self,
        current: &'a BuildProject,
        input: &'a BuildProjectConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>>;
    fn archive<'a>(&'a self, id: Uuid, actor: ActorId) -> BoxFuture<'a, Result<(), BuildError>>;
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>>;
    fn enqueue_webhook<'a>(
        &'a self,
        current: &'a BuildProject,
        branch: &'a str,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<BuildClaim>, BuildError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a BuildClaim,
        result: &'a BuildExecutionResult,
    ) -> BoxFuture<'a, Result<bool, BuildError>>;
    fn list_runs<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<BuildRun>, BuildError>>;
    fn get_run<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildRun, BuildError>>;
    fn logs<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<Vec<BuildLogEntry>, BuildError>>;
    fn append_log<'a>(
        &'a self,
        run_id: Uuid,
        log: &'a BuildLog,
    ) -> BoxFuture<'a, Result<BuildLogEntry, BuildError>>;
    fn cancel_queued<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<bool, BuildError>>;
}
