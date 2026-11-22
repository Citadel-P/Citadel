use super::*;

impl PostgresScriptRepository {
    pub(super) fn enqueue_scheduled_impl<'a>(
        &'a self,
        script_id: Uuid,
        scheduled_minute: DateTime<Utc>,
        max_timeout_seconds: i32,
    ) -> BoxFuture<'a, Result<Option<ScriptRun>, ScriptError>> {
        Box::pin(async move {
            if scheduled_minute.timestamp().rem_euclid(60) != 0
                || scheduled_minute.timestamp_subsec_nanos() != 0
            {
                return Ok(None);
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let Some(script) = sqlx::query("SELECT * FROM scripts WHERE id=$1 AND enabled=true AND scheduleenabled=true FOR UPDATE")
                .bind(script_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)? else {
                    tx.rollback().await.map_err(storage)?;
                    return Ok(None);
                };
            let last: Option<DateTime<Utc>> =
                script.try_get("lastscheduledrunat").map_err(storage)?;
            let state: String = script.try_get("controlstate").map_err(storage)?;
            let cron: Option<String> = script.try_get("schedulecron").map_err(storage)?;
            let zone: String = script.try_get("scheduletimezone").map_err(storage)?;
            let timeout: i32 = script.try_get("timeoutseconds").map_err(storage)?;
            if !citadel_primitives::schedule::schedule_is_due(
                cron.as_deref(),
                &zone,
                scheduled_minute,
            ) || !(1..=max_timeout_seconds).contains(&timeout)
                || last.is_some_and(|value| value >= scheduled_minute)
                || state != "Idle"
            {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let run_id = Uuid::now_v7();
            let code: String = script.try_get("code").map_err(storage)?;
            let name: String = script.try_get("name").map_err(storage)?;
            let run_as: Uuid = script.try_get("runasactorid").map_err(storage)?;
            let args: serde_json::Value = script.try_get("defaultargsjson").map_err(storage)?;
            sqlx::query("INSERT INTO scriptruns(id,scriptid,scriptname,argsjson,codehash,codesnapshot,queuedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'Queued',$9,'Schedule',NULL)")
                .bind(run_id).bind(script_id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                .bind(Utc::now()).bind(run_as).bind(timeout)
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
            let affected = sqlx::query("UPDATE scripts SET controlstate='Queued',currentrunid=$2,lastscheduledrunat=$3,rowversion=rowversion+1 WHERE id=$1 AND controlstate='Idle'")
                .bind(script_id).bind(run_id).bind(scheduled_minute)
                .execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected != 1 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let run = get_run_tx(&mut tx, run_id).await?;
            add_run_activity(&mut tx, &run).await?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(run))
        })
    }
}
