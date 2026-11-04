use super::*;

pub(super) async fn operation_activity(
    tx: &mut Transaction<'_, Postgres>,
    op: &UpdateOperation,
) -> Result<(), UpdateError> {
    use UpdatePhase::*;
    let (key, info, status) = match op.phase {
        Preparing => (
            "started",
            ActivityEventInfo::InstanceUpdateStarted {
                operation_id: op.id,
                source_version: op
                    .plan
                    .database
                    .as_ref()
                    .map(|db| {
                        format!(
                            "PostgreSQL {}.{}",
                            db.source_version / 10000,
                            db.source_version % 10000
                        )
                    })
                    .unwrap_or_else(|| op.plan.source.version.clone()),
                target_version: op
                    .plan
                    .database
                    .as_ref()
                    .map(|db| format!("PostgreSQL {}", db.target_display()))
                    .unwrap_or_else(|| op.plan.target.version.clone()),
            },
            ActivityStatus::Information,
        ),
        Succeeded => (
            "outcome",
            ActivityEventInfo::InstanceUpdateCompleted {
                operation_id: op.id,
                source_version: op
                    .plan
                    .database
                    .as_ref()
                    .map(|db| {
                        format!(
                            "PostgreSQL {}.{}",
                            db.source_version / 10000,
                            db.source_version % 10000
                        )
                    })
                    .unwrap_or_else(|| op.plan.source.version.clone()),
                target_version: op
                    .plan
                    .database
                    .as_ref()
                    .map(|db| format!("PostgreSQL {}", db.target_display()))
                    .unwrap_or_else(|| op.plan.target.version.clone()),
            },
            ActivityStatus::Success,
        ),
        Postponed => (
            "outcome",
            ActivityEventInfo::InstanceUpdatePostponed {
                operation_id: op.id,
                message: "The update was postponed before replacement.".into(),
            },
            ActivityStatus::Warning,
        ),
        Failed | RecoveryRequired => (
            "outcome",
            ActivityEventInfo::InstanceUpdateFailed {
                operation_id: op.id,
                phase: format!("{:?}", op.phase),
                message: if op.phase == RecoveryRequired {
                    "Recovery is required. Inspect the protected host update status."
                } else {
                    "The update failed before replacement."
                }
                .into(),
            },
            ActivityStatus::Failure,
        ),
        _ => return Ok(()),
    };
    if sqlx::query("INSERT INTO instanceupdateeventreceipts(operationid,eventkey) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(op.id).bind(key).execute(&mut **tx).await.map_err(storage)?.rows_affected() == 1 {
        activity(tx, op.instance_id, op.actor_id, info, status, op.updated_at).await?;
    }
    Ok(())
}

pub(super) async fn activity(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
    actor: ActorId,
    info: ActivityEventInfo,
    status: ActivityStatus,
    now: DateTime<Utc>,
) -> Result<(), UpdateError> {
    let event =
        ActivityEvent::new_updates_event(instance, actor, info, status, now).map_err(storage)?;
    insert_activity(tx, &event).await.map_err(storage)
}
