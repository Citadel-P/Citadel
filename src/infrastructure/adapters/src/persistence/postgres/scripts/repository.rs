use super::*;

#[derive(Clone)]
pub struct PostgresScriptRepository {
    pub(super) pool: PgPool,
}

impl PostgresScriptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ScriptRepository for PostgresScriptRepository {
    fn permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, ScriptError>,
    > {
        self.permissions_impl(actor, ids)
    }

    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a ScriptConfiguration,
    ) -> BoxFuture<'a, Result<Script, ScriptError>> {
        self.create_impl(actor, input)
    }

    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<Script>, ScriptError>> {
        self.list_impl(actor, administrator)
    }

    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<Script, ScriptError>> {
        self.get_impl(id)
    }

    fn update<'a>(
        &'a self,
        current: &'a Script,
        input: &'a ScriptConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<Script, ScriptError>> {
        self.update_impl(current, input, actor, metadata_only)
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<Script, ScriptError>> {
        self.rename_impl(id, name, actor)
    }

    fn delete<'a>(&'a self, id: Uuid, actor: ActorId) -> BoxFuture<'a, Result<(), ScriptError>> {
        self.delete_impl(id, actor)
    }

    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        self.enqueue_impl(actor, id, trigger, args, timeout_seconds)
    }

    fn enqueue_with_draft<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        draft: Option<&'a ScriptDraft>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        self.enqueue_with_draft_impl(actor, id, trigger, args, timeout_seconds, draft)
    }

    fn enqueue_webhook<'a>(
        &'a self,
        id: Uuid,
        expected_webhook: &'a WebhookConfig,
        args: &'a serde_json::Value,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        self.enqueue_webhook_impl(id, expected_webhook, args)
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
        remote: bool,
    ) -> BoxFuture<'a, Result<ScriptClaimResult, ScriptError>> {
        self.claim_next_impl(stale_before, remote)
    }

    fn cancellation_requested(&self, run_id: Uuid) -> BoxFuture<'_, Result<bool, ScriptError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT cancelrequested FROM scriptruns WHERE id=$1")
                .bind(run_id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
        })
    }

    fn defer<'a>(
        &'a self,
        claim: &'a ScriptRunClaim,
    ) -> BoxFuture<'a, Result<citadel_scripts::ScriptRunStatus, ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let cancelled: Option<bool> = sqlx::query_scalar("UPDATE scriptruns SET status=CASE WHEN cancelrequested THEN 'Cancelled' ELSE 'Queued' END,finishedat=CASE WHEN cancelrequested THEN CURRENT_TIMESTAMP ELSE NULL END,startedat=NULL,attemptid=NULL,executorid=NULL,nexteligibleat=CURRENT_TIMESTAMP+INTERVAL '15 seconds' WHERE id=$1 AND attemptid=$2 AND status='Running' AND NOT EXISTS(SELECT 1 FROM scriptexecutorreservations WHERE runid=$1 AND NOT cleanupconfirmed) RETURNING cancelrequested")
                .bind(claim.run.id).bind(claim.run.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let cancelled = cancelled.ok_or_else(|| {
                ScriptError::Conflict("Script attempt cannot be deferred.".into())
            })?;
            sqlx::query(
                "DELETE FROM scriptexecutorreservations WHERE runid=$1 AND cleanupconfirmed",
            )
            .bind(claim.run.id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            sqlx::query("UPDATE scripts SET controlstate=CASE WHEN $3 THEN 'Idle' ELSE 'Queued' END,currentrunid=CASE WHEN $3 THEN NULL ELSE currentrunid END,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2")
                .bind(claim.run.script_id).bind(claim.run.id).bind(cancelled).execute(&mut *tx).await.map_err(storage)?;
            if cancelled {
                let run = get_run_tx(&mut tx, claim.run.id).await?;
                add_run_activity(&mut tx, &run).await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(if cancelled {
                citadel_scripts::ScriptRunStatus::Cancelled
            } else {
                citadel_scripts::ScriptRunStatus::Queued
            })
        })
    }

    fn finish<'a>(
        &'a self,
        claim: &'a ScriptRunClaim,
        result: &'a ScriptRunResult,
    ) -> BoxFuture<'a, Result<bool, ScriptError>> {
        self.finish_impl(claim, result)
    }

    fn list_runs<'a>(
        &'a self,
        script_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<ScriptRun>, ScriptError>> {
        self.list_runs_impl(script_id, limit)
    }

    fn get_run<'a>(
        &'a self,
        script_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        self.get_run_impl(script_id, run_id)
    }

    fn list_scheduled<'a>(
        &'a self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Script>, ScriptError>> {
        self.list_scheduled_impl(after, limit)
    }

    fn enqueue_scheduled<'a>(
        &'a self,
        script_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<ScriptRun>, ScriptError>> {
        self.enqueue_scheduled_impl(script_id, scheduled_minute)
    }

    fn cancel<'a>(
        &'a self,
        script_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), ScriptError>> {
        self.cancel_impl(script_id, run_id)
    }
}
