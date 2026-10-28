use super::*;

impl PostgresScriptRepository {
    pub(super) fn enqueue_run<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        mode: EnqueueMode<'a>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        Box::pin(async move {
            if !args.is_object() || args.to_string().len() > 64 * 1024 {
                return Err(ScriptError::Validation(
                    "Script arguments must be a JSON object of at most 64 KiB.".to_owned(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let script = sqlx::query("SELECT * FROM scripts WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(ScriptError::NotFound)?;
            let enabled: bool = script.try_get("enabled").map_err(storage)?;
            if let EnqueueMode::Draft(Some(draft)) = mode {
                if trigger != "Test" {
                    return Err(ScriptError::Validation(
                        "Draft code is only accepted for tests.".into(),
                    ));
                }
                if script.try_get::<i64, _>("rowversion").map_err(storage)?
                    != draft.expected_row_version
                {
                    return Err(ScriptError::Conflict(
                        "Script changed after draft authorization. Reload and retry.".into(),
                    ));
                }
            }
            if let EnqueueMode::Webhook(expected) = mode {
                let current = script
                    .try_get::<Option<sqlx::types::Json<Option<WebhookConfig>>>, _>("webhook")
                    .map_err(storage)?
                    .and_then(|sqlx::types::Json(value)| value);
                if current.as_ref() != Some(expected) {
                    return Err(ScriptError::Conflict(
                        "Webhook configuration changed. Retry with the current configuration."
                            .into(),
                    ));
                }
            }
            if !enabled && trigger != "Test" {
                return Err(ScriptError::Conflict("Script is disabled.".to_owned()));
            }
            let state: String = script.try_get("controlstate").map_err(storage)?;
            let run_id = Uuid::now_v7();
            let code: String = match mode {
                EnqueueMode::Draft(Some(draft)) if trigger == "Test" => draft.code.clone(),
                _ => script.try_get("code").map_err(storage)?,
            };
            let name: String = script.try_get("name").map_err(storage)?;
            let run_as: Uuid = script.try_get("runasactorid").map_err(storage)?;
            let triggered_by = (trigger != "Webhook").then_some(actor.value());
            let configured_timeout: i32 = script.try_get("timeoutseconds").map_err(storage)?;
            let timeout = timeout_seconds.unwrap_or(configured_timeout);
            if !(1..=86_400).contains(&timeout) {
                return Err(ScriptError::Validation(
                    "Script timeout must be between 1 and 86400 seconds.".to_owned(),
                ));
            }
            if state != "Idle" {
                let reason = "Another run for this Script is already queued or running.";
                let row = sqlx::query("INSERT INTO scriptruns(id,scriptid,scriptname,argsjson,codehash,codesnapshot,queuedat,finishedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid,errormessage) VALUES($1,$2,$3,$4,$5,$6,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,$7,'Rejected',$8,$9,$10,$11) RETURNING *, false AS cleanuppending")
                    .bind(run_id).bind(id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                    .bind(run_as).bind(timeout).bind(trigger).bind(triggered_by).bind(reason)
                    .fetch_one(&mut *tx).await.map_err(database)?;
                add_run_activity(&mut tx, &map_run(row)?).await?;
                tx.commit().await.map_err(storage)?;
                return Err(ScriptError::Conflict(reason.into()));
            }
            sqlx::query("INSERT INTO scriptruns(id,scriptid,scriptname,argsjson,codehash,codesnapshot,queuedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'Queued',$9,$10,$11)")
                .bind(run_id).bind(id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                .bind(Utc::now()).bind(run_as).bind(timeout).bind(trigger).bind(triggered_by)
                .execute(&mut *tx).await.map_err(database)?;
            sqlx::query("UPDATE scriptruns SET executor=$2 WHERE id=$1")
                .bind(run_id)
                .bind(
                    script
                        .try_get::<serde_json::Value, _>("executor")
                        .map_err(storage)?,
                )
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("SELECT pg_notify('citadel_script_work','')")
                .execute(&mut *tx)
                .await
                .map_err(database)?;
            sqlx::query("UPDATE scripts SET controlstate='Queued',currentrunid=$2,rowversion=rowversion+1 WHERE id=$1").bind(id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            let run = get_run_tx(&mut tx, run_id).await?;
            add_run_activity(&mut tx, &run).await?;
            tx.commit().await.map_err(storage)?;
            Ok(run)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn enqueue_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
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

impl PostgresScriptRepository {
    pub(super) fn enqueue_with_draft_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
        draft: Option<&'a ScriptDraft>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        Box::pin(async move {
            self.enqueue_run(
                actor,
                id,
                trigger,
                args,
                timeout_seconds,
                EnqueueMode::Draft(draft),
            )
            .await
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn enqueue_webhook_impl<'a>(
        &'a self,
        id: Uuid,
        expected_webhook: &'a WebhookConfig,
        args: &'a serde_json::Value,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
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

impl PostgresScriptRepository {
    pub(super) fn claim_next_impl<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
        remote: bool,
    ) -> BoxFuture<'a, Result<ScriptClaimResult, ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let recovered_scripts = recover_interrupted(&mut tx, stale_before).await?;
            let row = sqlx::query("SELECT run.id,run.scriptid FROM scriptruns run JOIN scripts script ON script.id=run.scriptid WHERE run.status='Queued' AND script.currentrunid=run.id AND (run.executor->>'kind'<>'Core')=$1 AND (run.nexteligibleat IS NULL OR run.nexteligibleat<=CURRENT_TIMESTAMP) ORDER BY run.queuedat,run.id FOR UPDATE OF run,script SKIP LOCKED LIMIT 1").bind(remote).fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                tx.commit().await.map_err(storage)?;
                return Ok(ScriptClaimResult {
                    claim: None,
                    recovered_scripts,
                });
            };
            let run_id: Uuid = row.try_get("id").map_err(storage)?;
            let script_id: Uuid = row.try_get("scriptid").map_err(storage)?;
            let run = start_run(&mut tx, script_id, run_id).await?;
            tx.commit().await.map_err(storage)?;
            Ok(ScriptClaimResult {
                claim: Some(ScriptRunClaim { run }),
                recovered_scripts,
            })
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn finish_impl<'a>(
        &'a self,
        claim: &'a ScriptRunClaim,
        result: &'a ScriptRunResult,
    ) -> BoxFuture<'a, Result<bool, ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let finished = Utc::now();
            let affected = sqlx::query("UPDATE scriptruns SET status=$2,finishedat=$3,durationms=GREATEST(0,(EXTRACT(EPOCH FROM ($3-startedat))*1000)::bigint),exitcode=$4,logs=$5,errormessage=$6 WHERE id=$1 AND status='Running' AND attemptid=$7")
                .bind(claim.run.id).bind(result.status.as_str()).bind(finished).bind(result.exit_code).bind(&result.logs).bind(result.error.as_deref()).bind(claim.run.attempt_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                sqlx::query("DELETE FROM scriptexecutorreservations WHERE runid=$1 AND attemptid=$2 AND cleanupconfirmed")
                    .bind(claim.run.id).bind(claim.run.attempt_id).execute(&mut *tx).await.map_err(storage)?;
                sqlx::query("UPDATE scripts SET controlstate='Idle',controlstartedat=NULL,currentrunid=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2 AND NOT EXISTS(SELECT 1 FROM scriptexecutorreservations WHERE runid=$2)").bind(claim.run.script_id).bind(claim.run.id).execute(&mut *tx).await.map_err(storage)?;
                let run = get_run_tx(&mut tx, claim.run.id).await?;
                add_run_activity(&mut tx, &run).await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(affected == 1)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn list_runs_impl<'a>(
        &'a self,
        script_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<ScriptRun>, ScriptError>> {
        Box::pin(async move {
            let query = format!(
                "SELECT {RUN_SUMMARY_COLUMNS} FROM scriptruns WHERE scriptid=$1 ORDER BY queuedat DESC,id DESC LIMIT $2"
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(script_id)
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

impl PostgresScriptRepository {
    pub(super) fn get_run_impl<'a>(
        &'a self,
        script_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>> {
        Box::pin(async move { get_run(&self.pool, script_id, run_id).await })
    }
}

impl PostgresScriptRepository {
    pub(super) fn cancel_impl<'a>(
        &'a self,
        script_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE scriptruns SET status='Cancelled',finishedat=CURRENT_TIMESTAMP,errormessage='Script run was cancelled.' WHERE id=$1 AND scriptid=$2 AND status='Queued'").bind(run_id).bind(script_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                sqlx::query("UPDATE scripts SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(script_id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
                let run = get_run_tx(&mut tx, run_id).await?;
                add_run_activity(&mut tx, &run).await?;
                tx.commit().await.map_err(storage)?;
                Ok(())
            } else {
                let affected = sqlx::query("UPDATE scriptruns SET cancelrequested=true WHERE id=$1 AND scriptid=$2 AND status='Running'")
                    .bind(run_id).bind(script_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
                tx.commit().await.map_err(storage)?;
                if affected == 1 {
                    Ok(())
                } else {
                    Err(ScriptError::Conflict(
                        "Script run is not active.".to_owned(),
                    ))
                }
            }
        })
    }
}
