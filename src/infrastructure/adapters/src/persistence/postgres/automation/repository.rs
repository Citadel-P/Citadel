use super::*;

#[derive(Clone)]
pub struct PostgresAutomationRepository {
    pub(super) pool: PgPool,
}

impl PostgresAutomationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AutomationRepository for PostgresAutomationRepository {
    fn permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<
            std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>,
            AutomationError,
        >,
    > {
        self.permissions_impl(actor, ids)
    }

    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AutomationActionConfiguration,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        self.create_impl(actor, input)
    }

    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AutomationAction>, AutomationError>> {
        self.list_impl(actor, administrator)
    }

    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        self.get_impl(id)
    }

    fn update<'a>(
        &'a self,
        current: &'a AutomationAction,
        input: &'a AutomationActionConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        self.update_impl(current, input, actor, metadata_only)
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        self.rename_impl(id, name, actor)
    }

    fn delete<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        self.delete_impl(id, actor)
    }

    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        self.enqueue_impl(actor, id, trigger, args, timeout_seconds)
    }

    fn enqueue_for_execution<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        code: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AutomationRunClaim, AutomationError>> {
        self.enqueue_for_execution_impl(actor, id, trigger, args, timeout_seconds, code)
    }

    fn enqueue_webhook<'a>(
        &'a self,
        id: Uuid,
        expected_webhook: &'a WebhookConfig,
        args: &'a serde_json::Value,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        self.enqueue_webhook_impl(id, expected_webhook, args)
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunClaim>, AutomationError>> {
        self.claim_next_impl(stale_before)
    }

    fn finish<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        result: &'a AutomationRunResult,
    ) -> BoxFuture<'a, Result<bool, AutomationError>> {
        self.finish_impl(claim, result)
    }

    fn list_runs<'a>(
        &'a self,
        action_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationRun>, AutomationError>> {
        self.list_runs_impl(action_id, limit)
    }

    fn get_run<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        self.get_run_impl(action_id, run_id)
    }

    fn list_scheduled<'a>(
        &'a self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationAction>, AutomationError>> {
        self.list_scheduled_impl(after, limit)
    }

    fn enqueue_scheduled<'a>(
        &'a self,
        action_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRun>, AutomationError>> {
        self.enqueue_scheduled_impl(action_id, scheduled_minute)
    }

    fn cancel<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        self.cancel_impl(action_id, run_id)
    }
}
