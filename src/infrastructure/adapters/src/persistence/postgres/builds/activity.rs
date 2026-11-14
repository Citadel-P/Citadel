use super::*;

pub(super) async fn record_pool_activity(
    tx: &mut Transaction<'_, Postgres>,
    pool: &BuildAgentPool,
    actor: ActorId,
    info: citadel_activities::ActivityEventInfo,
) -> Result<(), BuildError> {
    let activity = citadel_activities::ActivityEvent::new_build_pool_event(
        pool.id,
        pool.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}

pub(super) async fn record_project_activity(
    tx: &mut Transaction<'_, Postgres>,
    pool: &BuildProject,
    actor: ActorId,
    info: citadel_activities::ActivityEventInfo,
) -> Result<(), BuildError> {
    let activity = citadel_activities::ActivityEvent::new_build_event(
        pool.id,
        pool.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}

pub(super) async fn record_run_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(), BuildError> {
    use citadel_activities::ActivityEventInfo as Info;
    let row = sqlx::query("SELECT buildprojectid,projectnamesnapshot,triggeredbyactorid,status,trigger,startedat,completedat,exitcode,imagedigest,errormessage FROM buildruns WHERE id=$1")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    let status: citadel_builds::BuildRunStatus = row
        .try_get::<String, _>("status")
        .map_err(storage)?
        .parse()
        .map_err(storage)?;
    let trigger = row.try_get("trigger").map_err(storage)?;
    let started_at: Option<DateTime<Utc>> = row.try_get("startedat").map_err(storage)?;
    let completed_at: Option<DateTime<Utc>> = row.try_get("completedat").map_err(storage)?;
    let duration_ms = started_at
        .zip(completed_at)
        .map(|(start, end)| (end - start).num_milliseconds().max(0));
    let info = match status {
        citadel_builds::BuildRunStatus::Queued => Info::BuildRunQueued {
            run_id: id,
            trigger,
        },
        citadel_builds::BuildRunStatus::Preparing => Info::BuildRunStarted {
            run_id: id,
            trigger,
        },
        citadel_builds::BuildRunStatus::Succeeded => Info::BuildRunSucceeded {
            run_id: id,
            trigger,
            exit_code: row.try_get("exitcode").map_err(storage)?,
            duration_ms,
            image_digest: row.try_get("imagedigest").map_err(storage)?,
        },
        citadel_builds::BuildRunStatus::TimedOut => Info::BuildRunTimedOut {
            run_id: id,
            trigger,
            duration_ms,
            error_message: row.try_get("errormessage").map_err(storage)?,
        },
        citadel_builds::BuildRunStatus::Cancelled => Info::BuildRunCancelled {
            run_id: id,
            trigger,
        },
        citadel_builds::BuildRunStatus::Failed | citadel_builds::BuildRunStatus::Interrupted => {
            Info::BuildRunFailed {
                run_id: id,
                trigger,
                status: status.to_string(),
                exit_code: row.try_get("exitcode").map_err(storage)?,
                duration_ms,
                error_message: row.try_get("errormessage").map_err(storage)?,
            }
        }
        citadel_builds::BuildRunStatus::Running | citadel_builds::BuildRunStatus::Rejected => {
            return Err(BuildError::Storage(
                "Invalid Build Activity transition.".into(),
            ));
        }
    };
    let activity = citadel_activities::ActivityEvent::new_build_event(
        row.try_get("buildprojectid").map_err(storage)?,
        row.try_get("projectnamesnapshot").map_err(storage)?,
        ActorId::new(row.try_get("triggeredbyactorid").map_err(storage)?),
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}
