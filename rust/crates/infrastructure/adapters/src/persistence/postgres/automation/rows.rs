use super::*;

pub(super) async fn start_run(
    tx: &mut Transaction<'_, Postgres>,
    action_id: Uuid,
    run_id: Uuid,
) -> Result<AutomationRun, AutomationError> {
    let started = Utc::now();
    sqlx::query(
        "UPDATE actionruns SET status='Running',startedat=$2 WHERE id=$1 AND status='Queued'",
    )
    .bind(run_id)
    .bind(started)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    sqlx::query("UPDATE actions SET controlstate='Processing',controlstartedat=$2,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$3").bind(action_id).bind(started.timestamp()).bind(run_id).execute(&mut **tx).await.map_err(storage)?;
    let run = get_run_tx(tx, run_id).await?;
    add_run_activity(tx, &run).await?;
    Ok(run)
}

pub(super) async fn lock_action(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AutomationAction, AutomationError> {
    sqlx::query("SELECT * FROM actions WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_action)
}

pub(super) async fn get_action_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AutomationAction, AutomationError> {
    let mut action = sqlx::query("SELECT * FROM actions WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_action)?;
    enrich_actions(&mut *tx, std::slice::from_mut(&mut action)).await?;
    Ok(action)
}

pub(super) async fn enrich_actions(
    connection: &mut sqlx::PgConnection,
    actions: &mut [AutomationAction],
) -> Result<(), AutomationError> {
    if actions.is_empty() {
        return Ok(());
    }
    let ids: Vec<_> = actions.iter().map(|action| action.id).collect();
    let mut positions: std::collections::HashMap<_, _> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let tags = sqlx::query("SELECT link.resourceid,tag.id,tag.name,tag.color FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype='AutomationAction' AND link.resourceid=ANY($1) ORDER BY tag.name,tag.id")
        .bind(&ids).fetch_all(&mut *connection).await.map_err(storage)?;
    for row in tags {
        let id: Uuid = row.try_get("resourceid").map_err(storage)?;
        if let Some(index) = positions.get(&id) {
            actions[*index].tags.push(citadel_tags::TagSummary {
                id: row.try_get("id").map_err(storage)?,
                name: row.try_get("name").map_err(storage)?,
                color: row.try_get("color").map_err(storage)?,
            });
        }
    }
    let query = format!(
        "SELECT summary.* FROM unnest($1::uuid[]) ids(id) CROSS JOIN LATERAL (SELECT {RUN_SUMMARY_COLUMNS} FROM actionruns WHERE actionid=ids.id ORDER BY queuedat DESC,id DESC LIMIT 1) summary"
    );
    let runs = sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(&ids)
        .fetch_all(&mut *connection)
        .await
        .map_err(storage)?;
    for row in runs {
        let run = map_run(row)?;
        if let Some(index) = positions.remove(&run.action_id) {
            actions[index].latest_run = Some(run);
        }
    }
    Ok(())
}

pub(super) async fn get_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AutomationRun, AutomationError> {
    sqlx::query("SELECT * FROM actionruns WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_run)
}

pub(super) async fn get_run(
    pool: &PgPool,
    action_id: Uuid,
    run_id: Uuid,
) -> Result<AutomationRun, AutomationError> {
    sqlx::query("SELECT * FROM actionruns WHERE id=$1 AND actionid=$2")
        .bind(run_id)
        .bind(action_id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_run)
}

pub(super) fn parse_args(value: &str) -> Result<serde_json::Value, AutomationError> {
    serde_json::from_str(value).map_err(|_| {
        AutomationError::Validation("Default arguments must be valid JSON.".to_owned())
    })
}

pub(super) fn map_action(row: sqlx::postgres::PgRow) -> Result<AutomationAction, AutomationError> {
    Ok(AutomationAction {
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
            .try_get::<Option<sqlx::types::Json<Option<RepoWebhookConfig>>>, _>("webhook")
            .map_err(storage)?
            .and_then(|sqlx::types::Json(value)| value),
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        alert_on_failure: row.try_get("alertonfailure").map_err(storage)?,
        run_as_actor_id: row.try_get("runasactorid").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        current_run_id: row.try_get("currentrunid").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
        last_scheduled_run_at: row.try_get("lastscheduledrunat").map_err(storage)?,
        tags: Vec::new(),
        latest_run: None,
    })
}

pub(super) fn map_run(row: sqlx::postgres::PgRow) -> Result<AutomationRun, AutomationError> {
    Ok(AutomationRun {
        id: row.try_get("id").map_err(storage)?,
        action_id: row.try_get("actionid").map_err(storage)?,
        action_name: row.try_get("actionname").map_err(storage)?,
        trigger: row.try_get("trigger").map_err(storage)?,
        status: row.try_get("status").map_err(storage)?,
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

pub(super) fn storage(error: impl std::fmt::Display) -> AutomationError {
    AutomationError::Storage(error.to_string())
}

pub(super) fn database(error: sqlx::Error) -> AutomationError {
    if error
        .as_database_error()
        .is_some_and(|value| value.is_unique_violation())
    {
        AutomationError::Conflict(
            "Automation Action already exists or has an active run.".to_owned(),
        )
    } else {
        storage(error)
    }
}
