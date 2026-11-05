use super::*;
use citadel_updates::{AgentUpdateReview, UpdateSchedule, UpdateSchedulePhase, UpdateScheduleRun};

pub(super) async fn latest(pool: &PgPool) -> Result<Option<UpdateScheduleRun>, UpdateError> {
    Ok(
        sqlx::query_scalar::<_, sqlx::types::Json<UpdateScheduleRun>>(
            "SELECT run FROM updatescheduleruns ORDER BY minute DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(|v| v.0),
    )
}

pub(super) async fn reserve(
    pool: &PgPool,
    schedule: &UpdateSchedule,
    minute: DateTime<Utc>,
) -> Result<Option<UpdateScheduleRun>, UpdateError> {
    schedule.validate()?;
    if !schedule.enabled
        || !citadel_primitives::schedule::schedule_is_due(
            Some(&schedule.cron),
            &schedule.time_zone,
            minute,
        )
    {
        return Ok(None);
    }
    let mut tx = pool.begin().await.map_err(storage)?;
    lock_identity_mutations(&mut tx).await.map_err(storage)?;
    let changed = sqlx::query("UPDATE agentupdatesettings SET lastscheduledminute=$2 WHERE id=1 AND schedule=$1 AND (lastscheduledminute IS NULL OR lastscheduledminute<$2) AND date_trunc('minute',now())=$2")
        .bind(sqlx::types::Json(schedule)).bind(minute).execute(&mut *tx).await.map_err(storage)?.rows_affected();
    if changed == 0 {
        return Ok(None);
    }
    if super::agent_operations::busy(&mut tx, None).await? {
        tx.commit().await.map_err(storage)?;
        return Ok(None);
    }
    let run = UpdateScheduleRun::new(schedule.clone(), minute);
    sqlx::query("INSERT INTO updatescheduleruns(id,minute,run) VALUES($1,$2,$3)")
        .bind(run.id)
        .bind(minute)
        .bind(sqlx::types::Json(&run))
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    Ok(Some(run))
}

async fn owned(
    tx: &mut Transaction<'_, Postgres>,
    run: &UpdateScheduleRun,
) -> Result<UpdateScheduleRun, UpdateError> {
    lock_identity_mutations(tx).await.map_err(storage)?;
    let saved = sqlx::query_scalar::<_, sqlx::types::Json<UpdateScheduleRun>>("SELECT run FROM updatescheduleruns WHERE id=$1 AND revision=$2 AND NOT finished FOR UPDATE")
        .bind(run.id).bind(run.revision as i64).fetch_optional(&mut **tx).await.map_err(storage)?
        .ok_or_else(|| UpdateError::Conflict("Scheduled update ownership changed.".into()))?.0;
    if saved.schedule != run.schedule || saved.minute != run.minute {
        return Err(UpdateError::Conflict(
            "Scheduled update scope changed.".into(),
        ));
    }
    Ok(saved)
}
async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    run: &UpdateScheduleRun,
) -> Result<UpdateScheduleRun, UpdateError> {
    let mut next = run.clone();
    next.revision += 1;
    sqlx::query("UPDATE updatescheduleruns SET revision=$2,run=$3,finished=$4 WHERE id=$1")
        .bind(next.id)
        .bind(next.revision as i64)
        .bind(sqlx::types::Json(&next))
        .bind(next.phase == UpdateSchedulePhase::Finished)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(next)
}
pub(super) async fn save(
    pool: &PgPool,
    run: &UpdateScheduleRun,
) -> Result<UpdateScheduleRun, UpdateError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    let saved = owned(&mut tx, run).await?;
    use UpdateSchedulePhase::*;
    if !matches!(
        (saved.phase, run.phase),
        (Preparing, Core | Agents | Finished) | (Core, Agents | Finished) | (Agents, Finished)
    ) {
        return Err(UpdateError::Conflict(
            "Invalid scheduled update transition.".into(),
        ));
    }
    if saved.phase == Preparing && run.phase != Finished {
        let selected: std::collections::BTreeSet<_> =
            run.targets.iter().map(|target| &target.id).collect();
        if selected != run.schedule.targets.iter().collect()
            || selected.len() != run.targets.len()
            || (run.phase == Core && (!run.schedule.include_core || run.core_plan.is_none()))
            || (!run.targets.is_empty() && run.release.is_none())
        {
            return Err(UpdateError::Conflict(
                "Scheduled update preparation does not match its scope.".into(),
            ));
        }
    }
    if saved.phase == Core && run.phase == Agents {
        let id = saved
            .core_operation
            .ok_or_else(|| UpdateError::Conflict("Core has not completed.".into()))?;
        let verified: bool = sqlx::query_scalar("SELECT NOT ownsinstallation AND operation->>'phase'='Succeeded' FROM instanceupdates WHERE id=$1")
            .bind(id).fetch_one(&mut *tx).await.map_err(storage)?;
        if !verified {
            return Err(UpdateError::Conflict(
                "Core verification and cleanup are incomplete.".into(),
            ));
        }
    }
    if saved.core_operation != run.core_operation
        || saved.agent_operation != run.agent_operation
        || (saved.phase != UpdateSchedulePhase::Preparing
            && (saved.core_plan != run.core_plan
                || saved.release != run.release
                || saved.targets != run.targets))
    {
        return Err(UpdateError::Conflict(
            "Scheduled update identity changed.".into(),
        ));
    }
    if run.phase == UpdateSchedulePhase::Finished
        && super::agent_operations::busy(&mut tx, Some(run.id)).await?
    {
        return Err(UpdateError::Conflict(
            "Scheduled execution still requires recovery.".into(),
        ));
    }
    let next = persist(&mut tx, run).await?;
    if run.phase == UpdateSchedulePhase::Finished {
        let instance_id = instance(&mut tx).await?;
        activity(
            &mut tx,
            instance_id,
            ActorId::new(SYSTEM_ACTOR_ID),
            ActivityEventInfo::UpdateScheduleFinished {
                run_id: run.id,
                message: run
                    .message
                    .clone()
                    .unwrap_or_else(|| "Scheduled updates completed.".into()),
            },
            if run.message.is_some() {
                ActivityStatus::Warning
            } else {
                ActivityStatus::Information
            },
            Utc::now(),
        )
        .await?;
    }
    tx.commit().await.map_err(storage)?;
    Ok(next)
}
pub(super) async fn claim_core(
    pool: &PgPool,
    run: &UpdateScheduleRun,
) -> Result<UpdateOperation, UpdateError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    let mut saved = owned(&mut tx, run).await?;
    if saved != *run
        || saved.phase != UpdateSchedulePhase::Core
        || !saved.schedule.include_core
        || saved.core_operation.is_some()
        || super::agent_operations::busy(&mut tx, Some(run.id)).await?
    {
        return Err(UpdateError::Conflict(
            "Scheduled Core update is already owned or blocked.".into(),
        ));
    }
    let plan = run
        .core_plan
        .as_ref()
        .ok_or_else(|| UpdateError::Conflict("Missing scheduled Core review.".into()))?;
    if instance(&mut tx).await? != plan.instance_id {
        return Err(UpdateError::Conflict(
            "Installation identity changed.".into(),
        ));
    }
    let op = UpdateOperation::approve(
        plan.clone(),
        ActorId::new(SYSTEM_ACTOR_ID),
        run.id,
        Utc::now(),
    )?;
    sqlx::query("INSERT INTO instanceupdates(id,instanceid,actorid,idempotencykey,revision,operation) VALUES($1,$2,$3,$4,0,$5)")
        .bind(op.id).bind(op.instance_id).bind(op.actor_id.value()).bind(op.idempotency_key).bind(sqlx::types::Json(&op)).execute(&mut *tx).await.map_err(storage)?;
    saved.core_operation = Some(op.id);
    persist(&mut tx, &saved).await?;
    operation_activity(&mut tx, &op).await?;
    tx.commit().await.map_err(storage)?;
    Ok(op)
}
pub(super) async fn claim_agents(
    pool: &PgPool,
    run: &UpdateScheduleRun,
) -> Result<(), UpdateError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    let mut saved = owned(&mut tx, run).await?;
    if saved != *run
        || saved.phase != UpdateSchedulePhase::Agents
        || saved.agent_operation.is_some()
    {
        return Err(UpdateError::Conflict(
            "Scheduled Agent update is already owned.".into(),
        ));
    }
    let release = run
        .release
        .clone()
        .ok_or_else(|| UpdateError::Conflict("Missing pinned Agent release.".into()))?;
    let review = AgentUpdateReview {
        id: run.id,
        actor_id: ActorId::new(SYSTEM_ACTOR_ID),
        release,
        targets: run.targets.clone(),
        expires_at: Utc::now() + chrono::Duration::minutes(5),
    };
    let op = super::agent_operations::claim(&mut tx, &review, run.id, true, Some(run.id)).await?;
    saved.agent_operation = Some(op.id);
    persist(&mut tx, &saved).await?;
    tx.commit().await.map_err(storage)
}
