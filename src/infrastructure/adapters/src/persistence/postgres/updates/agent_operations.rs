use super::*;
use chrono::Utc;
use citadel_updates::{
    AgentUpdateOperation, AgentUpdatePhase, AgentUpdateRelease, AgentUpdateRepository,
    AgentUpdateResult, AgentUpdateReview, AgentUpdateSchedule, AgentUpdateState,
};

impl AgentUpdateRepository for PostgresUpdateRepository {
    fn operation(&self, id: Uuid) -> BoxFuture<'_, Result<AgentUpdateOperation, UpdateError>> {
        Box::pin(async move {
            sqlx::query_scalar::<_, sqlx::types::Json<AgentUpdateOperation>>(
                "SELECT operation FROM agentupdateoperations WHERE id=$1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(|v| v.0)
            .ok_or(UpdateError::NotFound)
        })
    }
    fn state(&self) -> BoxFuture<'_, Result<AgentUpdateState, UpdateError>> {
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT schedule,release,checkerror FROM agentupdatesettings WHERE id=1",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?;
            let operation = sqlx::query_scalar::<_, sqlx::types::Json<AgentUpdateOperation>>("SELECT operation FROM agentupdateoperations ORDER BY createdat DESC,id DESC LIMIT 1").fetch_optional(&self.pool).await.map_err(storage)?.map(|v| v.0);
            Ok(AgentUpdateState {
                schedule: row
                    .try_get::<sqlx::types::Json<AgentUpdateSchedule>, _>("schedule")
                    .map_err(storage)?
                    .0,
                release: row
                    .try_get::<Option<sqlx::types::Json<AgentUpdateRelease>>, _>("release")
                    .map_err(storage)?
                    .map(|v| v.0),
                check_error: row.try_get("checkerror").map_err(storage)?,
                operation,
            })
        })
    }
    fn save_release<'a>(
        &'a self,
        release: Option<&'a AgentUpdateRelease>,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            sqlx::query("UPDATE agentupdatesettings SET release=COALESCE($1,release),checkerror=$2 WHERE id=1").bind(release.map(sqlx::types::Json)).bind(error).execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }
    fn save_schedule<'a>(
        &'a self,
        actor: UpdateActor,
        schedule: &'a AgentUpdateSchedule,
        allow_configuration: bool,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let _fence = self.authorization.read_fence().await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            authorize(&mut tx, actor).await?;
            let previous = sqlx::query_scalar::<_, sqlx::types::Json<AgentUpdateSchedule>>(
                "SELECT schedule FROM agentupdatesettings WHERE id=1 FOR UPDATE",
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?
            .0;
            if !allow_configuration
                && (schedule.enabled
                    || *schedule
                        != AgentUpdateSchedule {
                            enabled: false,
                            ..previous.clone()
                        })
            {
                return Err(UpdateError::Forbidden);
            }
            if previous != *schedule {
                sqlx::query("UPDATE agentupdatesettings SET schedule=$1,lastscheduledminute=date_trunc('minute',now()) WHERE id=1").bind(sqlx::types::Json(schedule)).execute(&mut *tx).await.map_err(storage)?;
                let id = instance(&mut tx).await?;
                activity(
                    &mut tx,
                    id,
                    actor.actor_id,
                    ActivityEventInfo::AgentUpdateScheduleChanged {
                        enabled: schedule.enabled,
                        target_count: schedule.targets.len(),
                    },
                    ActivityStatus::Information,
                    Utc::now(),
                )
                .await?;
            }
            tx.commit().await.map_err(storage)
        })
    }
    fn save_review<'a>(
        &'a self,
        actor: UpdateActor,
        review: &'a AgentUpdateReview,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let _fence = self.authorization.read_fence().await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            authorize(&mut tx, actor).await?;
            sqlx::query("DELETE FROM instanceupdateplans WHERE expiresat<now()")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query(
                "INSERT INTO instanceupdateplans(id,actorid,expiresat,plan) VALUES($1,$2,$3,$4)",
            )
            .bind(review.id)
            .bind(actor.actor_id.value())
            .bind(review.expires_at)
            .bind(sqlx::types::Json(review))
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            tx.commit().await.map_err(storage)
        })
    }
    fn approve(
        &self,
        actor: UpdateActor,
        review: Uuid,
        request: Uuid,
    ) -> BoxFuture<'_, Result<AgentUpdateOperation, UpdateError>> {
        Box::pin(async move {
            let _fence = self.authorization.read_fence().await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            authorize(&mut tx, actor).await?;
            if let Some(previous) =
                sqlx::query_scalar::<_, sqlx::types::Json<AgentUpdateOperation>>(
                    "SELECT operation FROM agentupdateoperations WHERE requestid=$1",
                )
                .bind(request)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
            {
                if previous.0.actor_id != actor.actor_id || previous.0.review_id != review {
                    return Err(UpdateError::Conflict(
                        "Request ID belongs to another approval.".into(),
                    ));
                }
                return Ok(previous.0);
            }
            let review=sqlx::query_scalar::<_,sqlx::types::Json<AgentUpdateReview>>("SELECT plan FROM instanceupdateplans WHERE id=$1 AND actorid=$2 AND expiresat>now() FOR UPDATE").bind(review).bind(actor.actor_id.value()).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(|| UpdateError::Conflict("Review expired. Review the Agent selection again.".into()))?.0;
            let operation = claim(&mut tx, &review, request, false).await?;
            tx.commit().await.map_err(storage)?;
            Ok(operation)
        })
    }
    fn scheduled<'a>(
        &'a self,
        schedule: &'a AgentUpdateSchedule,
        review: &'a AgentUpdateReview,
        minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut tx).await.map_err(storage)?;
            let changed=sqlx::query("UPDATE agentupdatesettings SET lastscheduledminute=$2 WHERE id=1 AND schedule=$1 AND (lastscheduledminute IS NULL OR lastscheduledminute<$2) AND date_trunc('minute',now())=$2").bind(sqlx::types::Json(schedule)).bind(minute).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed == 0 {
                return Ok(false);
            }
            if busy(&mut tx).await? {
                tx.commit().await.map_err(storage)?;
                return Ok(false);
            }
            // A check that finds nothing to install is not a successful update Activity.
            if review.targets.iter().all(|t| {
                t.eligibility(Some(&review.release))
                    == citadel_updates::AgentUpdateEligibility::Current
            }) {
                tx.commit().await.map_err(storage)?;
                return Ok(false);
            }
            let mut review = review.clone();
            review.actor_id = ActorId::new(SYSTEM_ACTOR_ID);
            claim(&mut tx, &review, Uuid::now_v7(), true).await?;
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }
    fn authorize(&self, actor: ActorId) -> BoxFuture<'_, Result<(), UpdateError>> {
        Box::pin(async move {
            let _fence = self.authorization.read_fence().await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let user: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE actorid=$1")
                .bind(actor.value())
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(UpdateError::Forbidden)?;
            authorize(
                &mut tx,
                UpdateActor {
                    subject_id: user,
                    actor_id: actor,
                },
            )
            .await
        })
    }
    fn save_operation<'a>(
        &'a self,
        op: &'a AgentUpdateOperation,
    ) -> BoxFuture<'a, Result<AgentUpdateOperation, UpdateError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let previous=sqlx::query_scalar::<_,sqlx::types::Json<AgentUpdateOperation>>("SELECT operation FROM agentupdateoperations WHERE id=$1 AND revision=$2 AND NOT finished FOR UPDATE").bind(op.id).bind(op.revision as i64).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(|| UpdateError::Conflict("Agent operation ownership changed.".into()))?.0;
            if previous.request_id != op.request_id
                || previous.release != op.release
                || previous.actor_id != op.actor_id
                || previous.targets.len() != op.targets.len()
            {
                return Err(UpdateError::Conflict(
                    "Agent operation identity changed.".into(),
                ));
            }
            let mut next = op.clone();
            next.revision += 1;
            sqlx::query(
                "UPDATE agentupdateoperations SET revision=$2,operation=$3,finished=$4 WHERE id=$1",
            )
            .bind(next.id)
            .bind(next.revision as i64)
            .bind(sqlx::types::Json(&next))
            .bind(next.finished)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            let instance = instance(&mut tx).await?;
            for (old, result) in previous.targets.iter().zip(&next.targets) {
                if old.target != result.target {
                    return Err(UpdateError::Conflict(
                        "Agent operation target changed.".into(),
                    ));
                }
                if old.phase == result.phase {
                    continue;
                }
                let status = match result.phase {
                    AgentUpdatePhase::Succeeded => ActivityStatus::Success,
                    AgentUpdatePhase::Failed => ActivityStatus::Failure,
                    AgentUpdatePhase::Skipped | AgentUpdatePhase::NotStarted => {
                        ActivityStatus::Warning
                    }
                    _ => ActivityStatus::Information,
                };
                activity(
                    &mut tx,
                    instance,
                    next.actor_id,
                    ActivityEventInfo::AgentUpdate {
                        operation_id: next.id,
                        agent: result.target.name.clone(),
                        source_version: result.target.installed_version.clone(),
                        target_version: next.release.version.clone(),
                        phase: format!("{:?}", result.phase),
                        message: result.message.clone(),
                    },
                    status,
                    next.updated_at,
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(next)
        })
    }
}
async fn busy(tx: &mut Transaction<'_, Postgres>) -> Result<bool, UpdateError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agentupdateoperations WHERE NOT finished) OR EXISTS(SELECT 1 FROM instanceupdates WHERE ownsinstallation)").fetch_one(&mut **tx).await.map_err(storage)
}
async fn claim(
    tx: &mut Transaction<'_, Postgres>,
    review: &AgentUpdateReview,
    request: Uuid,
    scheduled: bool,
) -> Result<AgentUpdateOperation, UpdateError> {
    if busy(tx).await? {
        return Err(UpdateError::Conflict(
            "Another update is running or requires recovery.".into(),
        ));
    }
    let now = Utc::now();
    let operation = AgentUpdateOperation {
        id: Uuid::now_v7(),
        request_id: request,
        review_id: review.id,
        actor_id: review.actor_id,
        scheduled,
        release: review.release.clone(),
        targets: review
            .targets
            .iter()
            .map(|target| AgentUpdateResult {
                target: target.clone(),
                phase: AgentUpdatePhase::Pending,
                message: None,
            })
            .collect(),
        revision: 0,
        created_at: now,
        updated_at: now,
        finished: false,
    };
    sqlx::query(
        "INSERT INTO agentupdateoperations(id,requestid,actorid,operation) VALUES($1,$2,$3,$4)",
    )
    .bind(operation.id)
    .bind(request)
    .bind(operation.actor_id.value())
    .bind(sqlx::types::Json(&operation))
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(operation)
}
