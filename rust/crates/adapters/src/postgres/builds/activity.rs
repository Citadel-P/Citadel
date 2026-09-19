use super::*;

pub(super) async fn record_pool_activity(
    tx: &mut Transaction<'_, Postgres>,
    pool: &BuildAgentPool,
    actor: ActorId,
    info: citadel_domain::ActivityEventInfo,
) -> Result<(), BuildError> {
    let activity = citadel_domain::ActivityEvent::new_build_pool_event(
        pool.id,
        pool.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}

pub(super) async fn record_project_activity(
    tx: &mut Transaction<'_, Postgres>,
    pool: &BuildProject,
    actor: ActorId,
    info: citadel_domain::ActivityEventInfo,
) -> Result<(), BuildError> {
    let activity = citadel_domain::ActivityEvent::new_build_event(
        pool.id,
        pool.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}

pub(super) async fn record_run_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(), BuildError> {
    use citadel_domain::ActivityEventInfo as Info;
    let row = sqlx::query("SELECT * FROM buildruns WHERE id=$1")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    let run = map_run(row)?;
    let duration_ms = run
        .started_at
        .zip(run.completed_at)
        .map(|(start, end)| (end - start).num_milliseconds().max(0));
    let info = match run.status.as_str() {
        "Queued" => Info::BuildRunQueued {
            run_id: id,
            trigger: run.trigger,
        },
        "Preparing" => Info::BuildRunStarted {
            run_id: id,
            trigger: run.trigger,
        },
        "Succeeded" => Info::BuildRunSucceeded {
            run_id: id,
            trigger: run.trigger,
            exit_code: run.exit_code,
            duration_ms,
            image_digest: run.image_digest,
        },
        "TimedOut" => Info::BuildRunTimedOut {
            run_id: id,
            trigger: run.trigger,
            duration_ms,
            error_message: run.error_message,
        },
        "Cancelled" => Info::BuildRunCancelled {
            run_id: id,
            trigger: run.trigger,
        },
        "Failed" | "Interrupted" => Info::BuildRunFailed {
            run_id: id,
            trigger: run.trigger,
            status: run.status,
            exit_code: run.exit_code,
            duration_ms,
            error_message: run.error_message,
        },
        _ => {
            return Err(BuildError::Storage(
                "Invalid Build Activity transition.".into(),
            ));
        }
    };
    let activity = citadel_domain::ActivityEvent::new_build_event(
        run.build_project_id,
        run.project_name_snapshot,
        ActorId::new(run.triggered_by_actor_id),
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}
