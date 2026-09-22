use super::*;
use citadel_activities::{ActivityEvent, ActivityEventInfo};

pub(super) async fn record_run_activity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
) -> Result<(), BackupError> {
    let row = sqlx::query("SELECT * FROM backupruns WHERE id=$1")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    let run = map_run(row)?;
    let info = match run.status.as_str() {
        "Queued" => ActivityEventInfo::BackupRunQueued {
            run_id: id,
            trigger: run.trigger,
        },
        "Running" => ActivityEventInfo::BackupRunStarted {
            run_id: id,
            trigger: run.trigger,
        },
        "Succeeded"
        | "SucceededWithWarnings"
        | "Failed"
        | "TimedOut"
        | "Cancelled"
        | "Interrupted"
        | "Rejected" => ActivityEventInfo::BackupRunCompleted {
            run_id: id,
            trigger: run.trigger,
            status: run.status,
            duration_ms: run
                .started_at
                .zip(run.completed_at)
                .map(|(start, end)| (end - start).num_milliseconds().max(0)),
            error_message: run.error_message,
        },
        _ => {
            return Err(BackupError::Storage(
                "Invalid Backup Activity transition.".into(),
            ));
        }
    };
    let activity = ActivityEvent::new_backup_policy_event(
        run.backup_policy_id,
        run.policy_name_snapshot,
        ActorId::new(run.triggered_by_actor_id),
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}
