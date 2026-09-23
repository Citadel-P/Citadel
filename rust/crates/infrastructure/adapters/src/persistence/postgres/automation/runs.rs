use super::*;

impl PostgresAutomationRepository {
    pub(super) fn enqueue_run<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        mode: EnqueueMode<'a>,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        Box::pin(async move {
            if !args.is_object() || args.to_string().len() > 64 * 1024 {
                return Err(AutomationError::Validation(
                    "Action arguments must be a JSON object of at most 64 KiB.".to_owned(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let action = sqlx::query("SELECT * FROM actions WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(AutomationError::NotFound)?;
            let enabled: bool = action.try_get("enabled").map_err(storage)?;
            if let EnqueueMode::Webhook(expected) = mode {
                let current = action
                    .try_get::<Option<sqlx::types::Json<Option<RepoWebhookConfig>>>, _>("webhook")
                    .map_err(storage)?
                    .and_then(|sqlx::types::Json(value)| value);
                if current.as_ref() != Some(expected) {
                    return Err(AutomationError::Conflict(
                        "Webhook configuration changed. Retry with the current configuration."
                            .into(),
                    ));
                }
            }
            if !enabled && trigger != "Test" {
                return Err(AutomationError::Conflict(
                    "Automation Action is disabled.".to_owned(),
                ));
            }
            let state: String = action.try_get("controlstate").map_err(storage)?;
            let run_id = Uuid::now_v7();
            let code: String = match mode {
                EnqueueMode::Execute(Some(code)) if trigger == "Test" => code.to_owned(),
                _ => action.try_get("code").map_err(storage)?,
            };
            let name: String = action.try_get("name").map_err(storage)?;
            let run_as: Uuid = action.try_get("runasactorid").map_err(storage)?;
            let triggered_by = (trigger != "Webhook").then_some(actor.value());
            let configured_timeout: i32 = action.try_get("timeoutseconds").map_err(storage)?;
            let timeout = timeout_seconds.unwrap_or(configured_timeout);
            if !(1..=86_400).contains(&timeout) {
                return Err(AutomationError::Validation(
                    "Automation timeout must be between 1 and 86400 seconds.".to_owned(),
                ));
            }
            if state != "Idle" {
                let reason = "Another run for this Action is already queued or running.";
                let row = sqlx::query("INSERT INTO actionruns(id,actionid,actionname,argsjson,codehash,codesnapshot,queuedat,finishedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid,errormessage) VALUES($1,$2,$3,$4,$5,$6,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,$7,'Rejected',$8,$9,$10,$11) RETURNING *")
                    .bind(run_id).bind(id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                    .bind(run_as).bind(timeout).bind(trigger).bind(triggered_by).bind(reason)
                    .fetch_one(&mut *tx).await.map_err(database)?;
                add_run_activity(&mut tx, &map_run(row)?).await?;
                tx.commit().await.map_err(storage)?;
                return Err(AutomationError::Conflict(reason.into()));
            }
            sqlx::query("INSERT INTO actionruns(id,actionid,actionname,argsjson,codehash,codesnapshot,queuedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'Queued',$9,$10,$11)")
                .bind(run_id).bind(id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                .bind(Utc::now()).bind(run_as).bind(timeout).bind(trigger).bind(triggered_by)
                .execute(&mut *tx).await.map_err(database)?;
            sqlx::query("SELECT pg_notify('citadel_automation_work','')")
                .execute(&mut *tx)
                .await
                .map_err(database)?;
            sqlx::query("UPDATE actions SET controlstate='Queued',currentrunid=$2,rowversion=rowversion+1 WHERE id=$1").bind(id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            let run = get_run_tx(&mut tx, run_id).await?;
            add_run_activity(&mut tx, &run).await?;
            let run = if matches!(mode, EnqueueMode::Execute(_)) {
                start_run(&mut tx, id, run_id).await?
            } else {
                run
            };
            tx.commit().await.map_err(storage)?;
            Ok(run)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn enqueue_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        self.enqueue_run(
            actor,
            id,
            trigger,
            args,
            timeout_seconds,
            EnqueueMode::Queue,
        )
    }
}

impl PostgresAutomationRepository {
    pub(super) fn enqueue_for_execution_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        code: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AutomationRunClaim, AutomationError>> {
        Box::pin(async move {
            self.enqueue_run(
                actor,
                id,
                trigger,
                args,
                timeout_seconds,
                EnqueueMode::Execute(code),
            )
            .await
            .map(|run| AutomationRunClaim { run })
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn enqueue_webhook_impl<'a>(
        &'a self,
        id: Uuid,
        expected_webhook: &'a RepoWebhookConfig,
        args: &'a serde_json::Value,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        self.enqueue_run(
            ActorId::new(Uuid::from_u128(1)),
            id,
            "Webhook",
            args,
            None,
            EnqueueMode::Webhook(expected_webhook),
        )
    }
}

impl PostgresAutomationRepository {
    pub(super) fn claim_next_impl<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunClaim>, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            recover_interrupted(&mut tx, stale_before).await?;
            let row = sqlx::query("SELECT run.id,run.actionid FROM actionruns run JOIN actions action ON action.id=run.actionid WHERE run.status='Queued' AND action.currentrunid=run.id ORDER BY run.queuedat,run.id FOR UPDATE OF run,action SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                tx.commit().await.map_err(storage)?;
                return Ok(None);
            };
            let run_id: Uuid = row.try_get("id").map_err(storage)?;
            let action_id: Uuid = row.try_get("actionid").map_err(storage)?;
            let run = start_run(&mut tx, action_id, run_id).await?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(AutomationRunClaim { run }))
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn finish_impl<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        result: &'a AutomationRunResult,
    ) -> BoxFuture<'a, Result<bool, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let finished = Utc::now();
            let affected = sqlx::query("UPDATE actionruns SET status=$2,finishedat=$3,durationms=GREATEST(0,(EXTRACT(EPOCH FROM ($3-startedat))*1000)::bigint),exitcode=$4,logs=$5,errormessage=$6 WHERE id=$1 AND status='Running'")
                .bind(claim.run.id).bind(result.status).bind(finished).bind(result.exit_code).bind(&result.logs).bind(result.error.as_deref()).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                sqlx::query("UPDATE actions SET controlstate='Idle',controlstartedat=NULL,currentrunid=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(claim.run.action_id).bind(claim.run.id).execute(&mut *tx).await.map_err(storage)?;
                let run = get_run_tx(&mut tx, claim.run.id).await?;
                add_run_activity(&mut tx, &run).await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(affected == 1)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn list_runs_impl<'a>(
        &'a self,
        action_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationRun>, AutomationError>> {
        Box::pin(async move {
            let query = format!(
                "SELECT {RUN_SUMMARY_COLUMNS} FROM actionruns WHERE actionid=$1 ORDER BY queuedat DESC,id DESC LIMIT $2"
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(action_id)
                .bind(i64::try_from(limit.clamp(1, 100)).unwrap_or(100))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_run)
                .collect()
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn get_run_impl<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>> {
        Box::pin(async move { get_run(&self.pool, action_id, run_id).await })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn cancel_impl<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE actionruns SET status='Cancelled',finishedat=CURRENT_TIMESTAMP,errormessage='Automation run was cancelled.' WHERE id=$1 AND actionid=$2 AND status='Queued'").bind(run_id).bind(action_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                sqlx::query("UPDATE actions SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(action_id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
                let run = get_run_tx(&mut tx, run_id).await?;
                add_run_activity(&mut tx, &run).await?;
                tx.commit().await.map_err(storage)?;
                Ok(())
            } else {
                tx.rollback().await.map_err(storage)?;
                Err(AutomationError::Conflict(
                    "Only a queued Automation run can be cancelled by this endpoint.".to_owned(),
                ))
            }
        })
    }
}
