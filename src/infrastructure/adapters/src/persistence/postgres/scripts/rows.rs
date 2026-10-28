use super::*;

pub(super) async fn start_run(
    tx: &mut Transaction<'_, Postgres>,
    script_id: Uuid,
    run_id: Uuid,
) -> Result<ScriptRun, ScriptError> {
    let started = Utc::now();
    sqlx::query(
        "UPDATE scriptruns SET status='Running',startedat=$2,attemptid=gen_random_uuid() WHERE id=$1 AND status='Queued'",
    )
    .bind(run_id)
    .bind(started)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    sqlx::query("UPDATE scripts SET controlstate='Processing',controlstartedat=$2,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$3").bind(script_id).bind(started.timestamp()).bind(run_id).execute(&mut **tx).await.map_err(storage)?;
    let run = get_run_tx(tx, run_id).await?;
    if matches!(run.executor, citadel_scripts::ScriptExecutor::Core) {
        add_run_activity(tx, &run).await?;
    }
    Ok(run)
}

pub(super) async fn lock_action(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Script, ScriptError> {
    sqlx::query("SELECT * FROM scripts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(ScriptError::NotFound)
        .and_then(map_action)
}

pub(super) async fn get_action_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Script, ScriptError> {
    let mut script = sqlx::query("SELECT * FROM scripts WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(ScriptError::NotFound)
        .and_then(map_action)?;
    enrich_actions(&mut *tx, std::slice::from_mut(&mut script)).await?;
    Ok(script)
}

pub(super) async fn enrich_actions(
    connection: &mut sqlx::PgConnection,
    scripts: &mut [Script],
) -> Result<(), ScriptError> {
    if scripts.is_empty() {
        return Ok(());
    }
    let ids: Vec<_> = scripts.iter().map(|script| script.id).collect();
    let mut positions: std::collections::HashMap<_, _> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let tags = sqlx::query("SELECT link.resourceid,tag.id,tag.name,tag.color FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype='Script' AND link.resourceid=ANY($1) ORDER BY tag.name,tag.id")
        .bind(&ids).fetch_all(&mut *connection).await.map_err(storage)?;
    for row in tags {
        let id: Uuid = row.try_get("resourceid").map_err(storage)?;
        if let Some(index) = positions.get(&id) {
            scripts[*index].tags.push(citadel_tags::TagSummary {
                id: row.try_get("id").map_err(storage)?,
                name: row.try_get("name").map_err(storage)?,
                color: row.try_get("color").map_err(storage)?,
            });
        }
    }
    let query = format!(
        "SELECT summary.* FROM unnest($1::uuid[]) ids(id) CROSS JOIN LATERAL (SELECT {RUN_SUMMARY_COLUMNS} FROM scriptruns WHERE scriptid=ids.id ORDER BY queuedat DESC,id DESC LIMIT 1) summary"
    );
    let runs = sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(&ids)
        .fetch_all(&mut *connection)
        .await
        .map_err(storage)?;
    for row in runs {
        let run = map_run(row)?;
        if let Some(index) = positions.remove(&run.script_id) {
            scripts[index].latest_run = Some(run);
        }
    }
    Ok(())
}

pub(super) async fn get_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<ScriptRun, ScriptError> {
    sqlx::query("SELECT *, EXISTS(SELECT 1 FROM scriptexecutorreservations WHERE runid=scriptruns.id) AS cleanuppending FROM scriptruns WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(ScriptError::NotFound)
        .and_then(map_run)
}

pub(super) async fn get_run(
    pool: &PgPool,
    script_id: Uuid,
    run_id: Uuid,
) -> Result<ScriptRun, ScriptError> {
    sqlx::query("SELECT *, EXISTS(SELECT 1 FROM scriptexecutorreservations WHERE runid=scriptruns.id) AS cleanuppending FROM scriptruns WHERE id=$1 AND scriptid=$2")
        .bind(run_id)
        .bind(script_id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(ScriptError::NotFound)
        .and_then(map_run)
}

pub(super) fn parse_args(value: &str) -> Result<serde_json::Value, ScriptError> {
    serde_json::from_str(value)
        .map_err(|_| ScriptError::Validation("Default arguments must be valid JSON.".to_owned()))
}

pub(super) fn map_action(row: sqlx::postgres::PgRow) -> Result<Script, ScriptError> {
    Ok(Script {
        executor: row
            .try_get::<sqlx::types::Json<citadel_scripts::ScriptExecutor>, _>("executor")
            .map_err(storage)?
            .0,
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        code: row.try_get("code").map_err(storage)?,
        default_args_json: row
            .try_get::<serde_json::Value, _>("defaultargsjson")
            .map_err(storage)?
            .to_string(),
        enabled: row.try_get("enabled").map_err(storage)?,
        schedule_enabled: row.try_get("scheduleenabled").map_err(storage)?,
        schedule_cron: row.try_get("schedulecron").map_err(storage)?,
        schedule_time_zone: row.try_get("scheduletimezone").map_err(storage)?,
        webhook: row
            .try_get::<Option<sqlx::types::Json<Option<WebhookConfig>>>, _>("webhook")
            .map_err(storage)?
            .and_then(|sqlx::types::Json(value)| value),
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        alert_on_failure: row.try_get("alertonfailure").map_err(storage)?,
        run_as_actor_id: row.try_get("runasactorid").map_err(storage)?,
        control_state: row
            .try_get::<String, _>("controlstate")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        current_run_id: row.try_get("currentrunid").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,

        updated_at: row.try_get("updatedat").map_err(storage)?,
        last_scheduled_run_at: row.try_get("lastscheduledrunat").map_err(storage)?,
        tags: Vec::new(),
        latest_run: None,

        audit: citadel_primitives::AuditMetadata {
            created_at: row.try_get("createdat").map_err(storage)?,
            created_by_actor_id: citadel_primitives::ActorId::new(
                row.try_get("createdbyactorid").map_err(storage)?,
            ),
        },
    })
}

pub(super) fn map_run(row: sqlx::postgres::PgRow) -> Result<ScriptRun, ScriptError> {
    Ok(ScriptRun {
        attempt_id: row.try_get("attemptid").map_err(storage)?,
        executor_id: row.try_get("executorid").map_err(storage)?,
        cleanup_pending: row.try_get("cleanuppending").map_err(storage)?,
        executor: row
            .try_get::<sqlx::types::Json<citadel_scripts::ScriptExecutor>, _>("executor")
            .map_err(storage)?
            .0,
        id: row.try_get("id").map_err(storage)?,
        script_id: row.try_get("scriptid").map_err(storage)?,
        script_name: row.try_get("scriptname").map_err(storage)?,
        trigger: row.try_get("trigger").map_err(storage)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        run_as_actor_id: row.try_get("runasactorid").map_err(storage)?,
        triggered_by_actor_id: row.try_get("triggeredbyactorid").map_err(storage)?,
        args_json: row
            .try_get::<serde_json::Value, _>("argsjson")
            .map_err(storage)?
            .to_string(),
        code_snapshot: row.try_get("codesnapshot").map_err(storage)?,
        code_hash: row.try_get("codehash").map_err(storage)?,
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        queued_at: row.try_get("queuedat").map_err(storage)?,
        started_at: row.try_get("startedat").map_err(storage)?,
        finished_at: row.try_get("finishedat").map_err(storage)?,
        duration_ms: row.try_get("durationms").map_err(storage)?,
        exit_code: row.try_get("exitcode").map_err(storage)?,
        logs: row.try_get("logs").map_err(storage)?,
        error_message: row.try_get("errormessage").map_err(storage)?,
    })
}

pub(super) fn storage(error: impl std::fmt::Display) -> ScriptError {
    ScriptError::Storage(error.to_string())
}

pub(super) fn database(error: sqlx::Error) -> ScriptError {
    if error
        .as_database_error()
        .is_some_and(|value| value.is_unique_violation())
    {
        ScriptError::Conflict("Script already exists or has an active run.".to_owned())
    } else {
        storage(error)
    }
}
