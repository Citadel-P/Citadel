use chrono::{DateTime, Utc};
use citadel_automation::{
    AutomationActionInput, AutomationActionView, AutomationError, AutomationRunClaim,
    AutomationRunResult, AutomationRunView, AutomationStore, code_hash,
};
use citadel_domain::{ActivityEvent, ActivityEventInfo, ActorId, ResourceType};
use futures_util::future::BoxFuture;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

const READ_MASK: i32 = 1 | 2 | 4;
const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid FROM actors actor
    WHERE actor.id=$1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid=$1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid=scope.actorid
        JOIN permissions permission ON permission.roleid=assignment.roleid
        WHERE permission.resourcetype=$2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
"#;

#[derive(Clone)]
pub struct PostgresAutomationStore {
    pool: PgPool,
}

impl PostgresAutomationStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AutomationStore for PostgresAutomationStore {
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AutomationActionInput,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO actions(id,alertonfailure,code,createdbyactorid,defaultargsjson,description,enabled,name,runasactorid,schedulecron,scheduleenabled,scheduletimezone,timeoutseconds,webhook) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
                .bind(id).bind(input.alert_on_failure).bind(&input.code).bind(actor.value())
                .bind(parse_args(input.default_args_json.as_deref().unwrap_or("{}"))?)
                .bind(input.description.as_deref()).bind(input.enabled).bind(&input.name)
                .bind(input.run_as_actor_id.ok_or_else(|| AutomationError::Validation("Run-as Actor is required.".to_owned()))?)
                .bind(input.schedule_cron.as_deref()).bind(input.schedule_enabled)
                .bind(input.schedule_time_zone.as_deref().unwrap_or("UTC"))
                .bind(input.timeout_seconds.unwrap_or(60)).bind(input.webhook.as_ref())
                .execute(&mut *tx).await.map_err(database)?;
            let action = get_action_tx(&mut tx, id).await?;
            add_activity(
                &mut tx,
                &action,
                actor,
                ActivityEventInfo::ActionCreated {
                    action: action.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(action)
        })
    }

    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AutomationActionView>, AutomationError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT action.* FROM actions action
WHERE $4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=action.id
      AND (access.permissionlevel & $3) <> 0
)
ORDER BY action.name,action.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::AutomationAction as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_action)
                .collect()
        })
    }

    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM actions WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(AutomationError::NotFound)
                .and_then(map_action)
        })
    }

    fn update<'a>(
        &'a self,
        current: &'a AutomationActionView,
        input: &'a AutomationActionInput,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE actions SET alertonfailure=$2,code=$3,defaultargsjson=$4,description=$5,enabled=$6,runasactorid=$7,schedulecron=$8,scheduleenabled=$9,scheduletimezone=$10,timeoutseconds=$11,webhook=$12,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND rowversion=$13")
                .bind(current.id).bind(input.alert_on_failure).bind(&input.code)
                .bind(parse_args(input.default_args_json.as_deref().unwrap_or("{}"))?)
                .bind(input.description.as_deref()).bind(input.enabled)
                .bind(input.run_as_actor_id.ok_or_else(|| AutomationError::Validation("Run-as Actor is required.".to_owned()))?)
                .bind(input.schedule_cron.as_deref()).bind(input.schedule_enabled)
                .bind(input.schedule_time_zone.as_deref().unwrap_or("UTC"))
                .bind(input.timeout_seconds.unwrap_or(60)).bind(input.webhook.as_ref())
                .bind(current.row_version)
                .execute(&mut *tx).await.map_err(database)?.rows_affected();
            if affected == 0 {
                return Err(AutomationError::Conflict(
                    "The Automation Action has changed. Reload before saving.".into(),
                ));
            }
            let action = get_action_tx(&mut tx, current.id).await?;
            if !metadata_only {
                add_activity(
                    &mut tx,
                    &action,
                    actor,
                    ActivityEventInfo::ActionUpdated {
                        old_action: current.snapshot(),
                        new_action: action.snapshot(),
                    },
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(action)
        })
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<AutomationActionView, AutomationError>> {
        Box::pin(async move {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 128 {
                return Err(AutomationError::Validation(
                    "Automation Action name must contain between 1 and 128 characters.".to_owned(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let current = lock_action(&mut tx, id).await?;
            sqlx::query("UPDATE actions SET name=$2,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1")
                .bind(id).bind(name).execute(&mut *tx).await.map_err(database)?;
            let action = get_action_tx(&mut tx, id).await?;
            add_activity(
                &mut tx,
                &action,
                actor,
                ActivityEventInfo::ActionRenamed {
                    old_name: current.name,
                    new_name: action.name.clone(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(action)
        })
    }

    fn delete<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let action = lock_action(&mut tx, id).await?;
            if action.control_state != "Idle" {
                return Err(AutomationError::Conflict(
                    "Automation Action has an active run.".to_owned(),
                ));
            }
            sqlx::query("DELETE FROM actions WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            add_activity(
                &mut tx,
                &action,
                actor,
                ActivityEventInfo::ActionDeleted {
                    action: action.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<AutomationRunView, AutomationError>> {
        Box::pin(async move {
            if !args.is_object() {
                return Err(AutomationError::Validation(
                    "Action arguments must be a JSON object.".to_owned(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let action = sqlx::query("SELECT * FROM actions WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(AutomationError::NotFound)?;
            let enabled: bool = action.try_get("enabled").map_err(storage)?;
            if !enabled && trigger != "Test" {
                return Err(AutomationError::Conflict(
                    "Automation Action is disabled.".to_owned(),
                ));
            }
            let state: String = action.try_get("controlstate").map_err(storage)?;
            let run_id = Uuid::now_v7();
            let code: String = action.try_get("code").map_err(storage)?;
            let name: String = action.try_get("name").map_err(storage)?;
            let run_as: Uuid = action.try_get("runasactorid").map_err(storage)?;
            let configured_timeout: i32 = action.try_get("timeoutseconds").map_err(storage)?;
            let timeout = timeout_seconds.unwrap_or(configured_timeout);
            if !(1..=86_400).contains(&timeout) {
                return Err(AutomationError::Validation(
                    "Automation timeout must be between 1 and 86400 seconds.".to_owned(),
                ));
            }
            if state != "Idle" {
                let reason = "Another run for this Action is already queued or running.";
                let row = sqlx::query("INSERT INTO actionruns(id,actionid,actionname,argsjson,codehash,codesnapshot,queuedat,finishedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid,errormessage) VALUES($1,$2,$3,$4,$5,$6,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,$7,'Rejected',$8,$9,$10,$11) RETURNING *")
                    .bind(run_id).bind(id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                    .bind(run_as).bind(timeout).bind(trigger).bind(actor.value()).bind(reason)
                    .fetch_one(&mut *tx).await.map_err(database)?;
                add_run_activity(&mut tx, &map_run(row)?).await?;
                tx.commit().await.map_err(storage)?;
                return Err(AutomationError::Conflict(reason.into()));
            }
            sqlx::query("INSERT INTO actionruns(id,actionid,actionname,argsjson,codehash,codesnapshot,queuedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'Queued',$9,$10,$11)")
                .bind(run_id).bind(id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                .bind(Utc::now()).bind(run_as).bind(timeout).bind(trigger).bind(actor.value())
                .execute(&mut *tx).await.map_err(database)?;
            sqlx::query("UPDATE actions SET controlstate='Queued',currentrunid=$2,rowversion=rowversion+1 WHERE id=$1").bind(id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            let run = get_run_tx(&mut tx, run_id).await?;
            add_run_activity(&mut tx, &run).await?;
            tx.commit().await.map_err(storage)?;
            Ok(run)
        })
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunClaim>, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            recover_interrupted(&mut tx, stale_before).await?;
            let row = sqlx::query("SELECT run.id,run.actionid FROM actionruns run JOIN actions action ON action.id=run.actionid WHERE run.status='Queued' AND action.currentrunid=run.id ORDER BY run.queuedat,run.id FOR UPDATE OF run,action SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                tx.commit().await.map_err(storage)?;
                return Ok(None);
            };
            let run_id: Uuid = row.try_get("id").map_err(storage)?;
            let action_id: Uuid = row.try_get("actionid").map_err(storage)?;
            let started = Utc::now();
            sqlx::query("UPDATE actionruns SET status='Running',startedat=$2 WHERE id=$1 AND status='Queued'").bind(run_id).bind(started).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE actions SET controlstate='Processing',controlstartedat=$2,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$3").bind(action_id).bind(started.timestamp()).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            let run = get_run_tx(&mut tx, run_id).await?;
            add_run_activity(&mut tx, &run).await?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(AutomationRunClaim { run }))
        })
    }

    fn finish<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        result: &'a AutomationRunResult,
    ) -> BoxFuture<'a, Result<bool, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let finished = Utc::now();
            let affected = sqlx::query("UPDATE actionruns SET status=$2,finishedat=$3,durationms=GREATEST(0,(EXTRACT(EPOCH FROM ($3-startedat))*1000)::bigint),exitcode=$4,logs=$5,errormessage=$6 WHERE id=$1 AND status='Running'")
                .bind(claim.run.id).bind(result.status).bind(finished).bind(result.exit_code).bind(&result.logs).bind(result.error.as_deref()).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                sqlx::query("UPDATE actions SET controlstate='Idle',controlstartedat=NULL,currentrunid=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(claim.run.action_id).bind(claim.run.id).execute(&mut *tx).await.map_err(storage)?;
                let run = get_run_tx(&mut tx, claim.run.id).await?;
                add_run_activity(&mut tx, &run).await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(affected == 1)
        })
    }

    fn list_runs<'a>(
        &'a self,
        action_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationRunView>, AutomationError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM actionruns WHERE actionid=$1 ORDER BY queuedat DESC,id DESC LIMIT $2").bind(action_id).bind(i64::try_from(limit.clamp(1,100)).unwrap_or(100)).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(map_run).collect()
        })
    }

    fn get_run<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<AutomationRunView, AutomationError>> {
        Box::pin(async move { get_run(&self.pool, action_id, run_id).await })
    }

    fn list_scheduled<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<AutomationActionView>, AutomationError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM actions WHERE enabled=true AND scheduleenabled=true AND schedulecron IS NOT NULL ORDER BY id")
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_action)
                .collect()
        })
    }

    fn enqueue_scheduled<'a>(
        &'a self,
        action_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunView>, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let Some(action) = sqlx::query("SELECT * FROM actions WHERE id=$1 AND enabled=true AND scheduleenabled=true FOR UPDATE")
                .bind(action_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)? else {
                    tx.rollback().await.map_err(storage)?;
                    return Ok(None);
                };
            let last: Option<DateTime<Utc>> =
                action.try_get("lastscheduledrunat").map_err(storage)?;
            let state: String = action.try_get("controlstate").map_err(storage)?;
            if last.is_some_and(|value| value >= scheduled_minute) || state != "Idle" {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let run_id = Uuid::now_v7();
            let code: String = action.try_get("code").map_err(storage)?;
            let name: String = action.try_get("name").map_err(storage)?;
            let run_as: Uuid = action.try_get("runasactorid").map_err(storage)?;
            let timeout: i32 = action.try_get("timeoutseconds").map_err(storage)?;
            let args: serde_json::Value = action.try_get("defaultargsjson").map_err(storage)?;
            sqlx::query("INSERT INTO actionruns(id,actionid,actionname,argsjson,codehash,codesnapshot,queuedat,runasactorid,status,timeoutseconds,trigger,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'Queued',$9,'Schedule',NULL)")
                .bind(run_id).bind(action_id).bind(name).bind(args).bind(code_hash(&code)).bind(code)
                .bind(Utc::now()).bind(run_as).bind(timeout)
                .execute(&mut *tx).await.map_err(database)?;
            let affected = sqlx::query("UPDATE actions SET controlstate='Queued',currentrunid=$2,lastscheduledrunat=$3,rowversion=rowversion+1 WHERE id=$1 AND controlstate='Idle'")
                .bind(action_id).bind(run_id).bind(scheduled_minute)
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

    fn cancel<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE actionruns SET status='Cancelled',finishedat=CURRENT_TIMESTAMP,errormessage='Automation run was cancelled.' WHERE id=$1 AND actionid=$2 AND status='Queued'").bind(run_id).bind(action_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                sqlx::query("UPDATE actions SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(action_id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
                let run = get_run_tx(&mut tx, run_id).await?;
                add_run_activity(&mut tx, &run).await?;
                tx.commit().await.map_err(storage)?;
                Ok(())
            } else {
                tx.rollback().await.map_err(storage)?;
                Err(AutomationError::Conflict(
                    "Only a queued Automation run can be cancelled by this endpoint.".to_owned(),
                ))
            }
        })
    }
}

async fn recover_interrupted(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
) -> Result<(), AutomationError> {
    let rows = sqlx::query("UPDATE actionruns SET status='Failed',finishedat=CURRENT_TIMESTAMP,errormessage='Automation run was interrupted by a Core restart.' WHERE status='Running' AND startedat<$1 AND startedat+make_interval(secs=>timeoutseconds+300)<CURRENT_TIMESTAMP RETURNING actionid,id").bind(stale_before).fetch_all(&mut **tx).await.map_err(storage)?;
    for row in rows {
        let action: Uuid = row.try_get("actionid").map_err(storage)?;
        let run: Uuid = row.try_get("id").map_err(storage)?;
        sqlx::query("UPDATE actions SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(action).bind(run).execute(&mut **tx).await.map_err(storage)?;
        let run = get_run_tx(tx, run).await?;
        add_run_activity(tx, &run).await?;
    }
    Ok(())
}

async fn lock_action(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AutomationActionView, AutomationError> {
    sqlx::query("SELECT * FROM actions WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_action)
}

async fn get_action_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AutomationActionView, AutomationError> {
    sqlx::query("SELECT * FROM actions WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_action)
}

async fn get_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AutomationRunView, AutomationError> {
    sqlx::query("SELECT * FROM actionruns WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_run)
}

async fn add_activity(
    tx: &mut Transaction<'_, Postgres>,
    action: &AutomationActionView,
    actor: ActorId,
    info: ActivityEventInfo,
) -> Result<(), AutomationError> {
    let event = ActivityEvent::new_automation_event(
        action.id,
        action.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}

async fn add_run_activity(
    tx: &mut Transaction<'_, Postgres>,
    run: &AutomationRunView,
) -> Result<(), AutomationError> {
    let run_id = run.id;
    let trigger = run.trigger.clone();
    let duration_ms = run.duration_ms;
    let exit_code = run.exit_code;
    let info = match run.status.as_str() {
        "Queued" => ActivityEventInfo::ActionRunQueued { run_id, trigger },
        "Running" => ActivityEventInfo::ActionRunStarted { run_id, trigger },
        "Succeeded" => ActivityEventInfo::ActionRunSucceeded {
            run_id,
            trigger,
            exit_code,
            duration_ms,
        },
        "Failed" => ActivityEventInfo::ActionRunFailed {
            run_id,
            trigger,
            exit_code,
            duration_ms,
            error_message: run.error_message.clone(),
        },
        "TimedOut" => ActivityEventInfo::ActionRunTimedOut {
            run_id,
            trigger,
            duration_ms,
            error_message: run.error_message.clone(),
        },
        "Cancelled" => ActivityEventInfo::ActionRunCancelled { run_id, trigger },
        "Rejected" => ActivityEventInfo::ActionRunRejected {
            run_id,
            trigger,
            reason: run
                .error_message
                .clone()
                .unwrap_or_else(|| "Run rejected.".into()),
        },
        _ => {
            return Err(AutomationError::Storage(
                "Unknown Automation run status.".into(),
            ));
        }
    };
    let event = ActivityEvent::new_automation_event(
        run.action_id,
        run.action_name.clone(),
        ActorId::new(run.triggered_by_actor_id.unwrap_or(run.run_as_actor_id)),
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &event)
        .await
        .map_err(storage)
}

async fn get_run(
    pool: &PgPool,
    action_id: Uuid,
    run_id: Uuid,
) -> Result<AutomationRunView, AutomationError> {
    sqlx::query("SELECT * FROM actionruns WHERE id=$1 AND actionid=$2")
        .bind(run_id)
        .bind(action_id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(AutomationError::NotFound)
        .and_then(map_run)
}
fn parse_args(value: &str) -> Result<serde_json::Value, AutomationError> {
    serde_json::from_str(value).map_err(|_| {
        AutomationError::Validation("Default arguments must be valid JSON.".to_owned())
    })
}
fn map_action(row: sqlx::postgres::PgRow) -> Result<AutomationActionView, AutomationError> {
    Ok(AutomationActionView {
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
        webhook: row.try_get("webhook").map_err(storage)?,
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        alert_on_failure: row.try_get("alertonfailure").map_err(storage)?,
        run_as_actor_id: row.try_get("runasactorid").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        current_run_id: row.try_get("currentrunid").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
    })
}
fn map_run(row: sqlx::postgres::PgRow) -> Result<AutomationRunView, AutomationError> {
    Ok(AutomationRunView {
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
fn storage(error: impl std::fmt::Display) -> AutomationError {
    AutomationError::Storage(error.to_string())
}
fn database(error: sqlx::Error) -> AutomationError {
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
