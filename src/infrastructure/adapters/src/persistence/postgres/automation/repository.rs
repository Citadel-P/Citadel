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

    fn enqueue_with_draft<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        draft: Option<&'a AutomationDraft>,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        self.enqueue_with_draft_impl(actor, id, trigger, args, timeout_seconds, draft)
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
        remote: bool,
    ) -> BoxFuture<'a, Result<AutomationClaimResult, AutomationError>> {
        self.claim_next_impl(stale_before, remote)
    }

    fn cancellation_requested(&self, run_id: Uuid) -> BoxFuture<'_, Result<bool, AutomationError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT cancelrequested FROM actionruns WHERE id=$1")
                .bind(run_id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
        })
    }

    fn defer<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
    ) -> BoxFuture<'a, Result<citadel_automation::AutomationRunStatus, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let cancelled: Option<bool> = sqlx::query_scalar("UPDATE actionruns SET status=CASE WHEN cancelrequested THEN 'Cancelled' ELSE 'Queued' END,finishedat=CASE WHEN cancelrequested THEN CURRENT_TIMESTAMP ELSE NULL END,startedat=NULL,attemptid=NULL,executorid=NULL,nexteligibleat=CURRENT_TIMESTAMP+INTERVAL '15 seconds' WHERE id=$1 AND attemptid=$2 AND status='Running' AND NOT EXISTS(SELECT 1 FROM automationexecutorreservations WHERE runid=$1 AND NOT cleanupconfirmed) RETURNING cancelrequested")
                .bind(claim.run.id).bind(claim.run.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let cancelled = cancelled.ok_or_else(|| {
                AutomationError::Conflict("Automation attempt cannot be deferred.".into())
            })?;
            sqlx::query(
                "DELETE FROM automationexecutorreservations WHERE runid=$1 AND cleanupconfirmed",
            )
            .bind(claim.run.id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            sqlx::query("UPDATE actions SET controlstate=CASE WHEN $3 THEN 'Idle' ELSE 'Queued' END,currentrunid=CASE WHEN $3 THEN NULL ELSE currentrunid END,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2")
                .bind(claim.run.action_id).bind(claim.run.id).bind(cancelled).execute(&mut *tx).await.map_err(storage)?;
            if cancelled {
                let run = get_run_tx(&mut tx, claim.run.id).await?;
                add_run_activity(&mut tx, &run).await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(if cancelled {
                citadel_automation::AutomationRunStatus::Cancelled
            } else {
                citadel_automation::AutomationRunStatus::Queued
            })
        })
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
