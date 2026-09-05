use std::collections::HashMap;

use chrono::{DateTime, Utc};
use citadel_backups::*;
use citadel_domain::{ActorId, ResourceType};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool, Row};
use uuid::Uuid;

use crate::resource_tags;

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
pub struct PostgresBackupStore {
    pool: PgPool,
}
impl PostgresBackupStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl BackupStore for PostgresBackupStore {
    fn create_repository<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupRepositoryInput,
    ) -> BoxFuture<'a, Result<BackupRepositoryView, BackupError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let kind = input
                .spec
                .get("$type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,description,type,spec,passwordsecretid,status,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,'Unknown',$8)").bind(id).bind(&input.name).bind(input.name.to_uppercase()).bind(&input.description).bind(kind).bind(&input.spec).bind(input.password_secret_id).bind(actor.value()).execute(&self.pool).await.map_err(storage)?;
            self.get_repository(id).await
        })
    }
    fn list_repositories(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupRepositoryView>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT repository.* FROM backuprepositories repository
WHERE repository.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=repository.id
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY repository.name,repository.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupRepository as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_repository)
                .collect()
        })
    }
    fn get_repository(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRepositoryView, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuprepositories WHERE id=$1 AND archivedat IS NULL")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BackupError::NotFound)
                .and_then(map_repository)
        })
    }
    fn archive_repository(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        Box::pin(async move {
            let n=sqlx::query("UPDATE backuprepositories SET archivedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND controlstate='Idle' AND NOT EXISTS(SELECT 1 FROM backuppolicies WHERE backuprepositoryid=$1 AND archivedat IS NULL)").bind(id).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if n == 0 {
                Err(BackupError::Conflict(
                    "Backup Repository is active, missing, or still referenced.".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
    fn record_repository_operation<'a>(
        &'a self,
        id: Uuid,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        succeeded: bool,
        message: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(BackupRepositoryView, BackupRepositoryValidationView), BackupError>>
    {
        Box::pin(async move {
            let status = if succeeded { "Ready" } else { "Unavailable" };
            let now = Utc::now();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let changed = sqlx::query("UPDATE backuprepositories SET status=$2,lastcheckedat=CASE WHEN $3 IN('Validate','Initialize','Check') THEN $5 ELSE lastcheckedat END,lastprunedat=CASE WHEN $3='Prune' AND $4 THEN $5 ELSE lastprunedat END,updatedat=$5,rowversion=rowversion+1,controlstate='Idle',currentrunid=NULL,controlstartedat=NULL WHERE id=$1 AND archivedat IS NULL")
                .bind(id).bind(status).bind(operation).bind(succeeded).bind(now).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed == 0 {
                return Err(BackupError::NotFound);
            }
            let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM backuprepositoryvalidations WHERE backuprepositoryid=$1 AND location=$2 AND (($3::uuid IS NULL AND platformid IS NULL) OR platformid=$3) FOR UPDATE")
                .bind(id).bind(location).bind(platform_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let validation_id = existing.unwrap_or_else(Uuid::now_v7);
            let error_code = (!succeeded).then_some("backup.repository_unavailable");
            if existing.is_some() {
                sqlx::query("UPDATE backuprepositoryvalidations SET status=$2,lastvalidatedat=$3,lasterrorcode=$4,lasterrormessage=$5 WHERE id=$1")
                    .bind(validation_id).bind(status).bind(now).bind(error_code).bind(message).execute(&mut *tx).await.map_err(storage)?;
            } else {
                sqlx::query("INSERT INTO backuprepositoryvalidations(id,backuprepositoryid,location,platformid,status,lastvalidatedat,lasterrorcode,lasterrormessage) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
                    .bind(validation_id).bind(id).bind(location).bind(platform_id).bind(status).bind(now).bind(error_code).bind(message).execute(&mut *tx).await.map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            if !succeeded {
                tracing::warn!(%id, %operation, error=message.unwrap_or("unknown"), "Backup Repository operation failed");
            }
            Ok((
                self.get_repository(id).await?,
                BackupRepositoryValidationView {
                    id: validation_id,
                    backup_repository_id: id,
                    location: location.to_owned(),
                    platform_id,
                    status: status.to_owned(),
                    last_validated_at: now,
                    last_error_code: error_code.map(str::to_owned),
                    last_error_message: message.map(str::to_owned),
                },
            ))
        })
    }
    fn acquire_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
        operation: &str,
        expires_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        let operation = operation.to_owned();
        Box::pin(async move {
            let affected = sqlx::query("INSERT INTO backuprepositoryleases(backuprepositoryid,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,$3,CURRENT_TIMESTAMP,$4) ON CONFLICT(backuprepositoryid) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype=EXCLUDED.operationtype,createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backuprepositoryleases.expiresat<=CURRENT_TIMESTAMP")
                .bind(repository_id)
                .bind(operation_id)
                .bind(operation)
                .bind(expires_at)
                .execute(&self.pool)
                .await
                .map_err(storage)?
                .rows_affected();
            Ok(affected == 1)
        })
    }
    fn release_repository_operation(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>> {
        Box::pin(async move {
            sqlx::query(
                "DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2",
            )
            .bind(repository_id)
            .bind(operation_id)
            .execute(&self.pool)
            .await
            .map_err(storage)?;
            Ok(())
        })
    }
    fn create_policy<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupPolicyInput,
    ) -> BoxFuture<'a, Result<BackupPolicyView, BackupError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,description,source,backuprepositoryid,enabled,cron,timezone,webhook,keeplastsuccessful,timeoutseconds,alertonfailure,runasactorid,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)").bind(id).bind(&input.name).bind(input.name.to_uppercase()).bind(&input.description).bind(&input.source).bind(input.backup_repository_id).bind(input.enabled).bind(&input.cron).bind(&input.time_zone).bind(&input.webhook).bind(input.keep_last_successful.unwrap_or(14)).bind(input.timeout_seconds.unwrap_or(14400)).bind(input.alert_on_failure).bind(input.run_as_actor_id.unwrap_or(actor.value())).bind(actor.value()).execute(&mut *transaction).await.map_err(storage)?;
            resource_tags::insert(
                &mut transaction,
                "BackupPolicy",
                id,
                &input.tag_ids,
                actor.value(),
            )
            .await
            .map_err(backup_tag_error)?;
            transaction.commit().await.map_err(storage)?;
            self.get_policy(id).await
        })
    }
    fn list_policies(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicyView>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT policy.* FROM backuppolicies policy
WHERE policy.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=policy.id
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY policy.name,policy.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_policy)
                .collect()
        })
    }
    fn get_policy(&self, id: Uuid) -> BoxFuture<'_, Result<BackupPolicyView, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuppolicies WHERE id=$1 AND archivedat IS NULL")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BackupError::NotFound)
                .and_then(map_policy)
        })
    }
    fn archive_policy(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        Box::pin(async move {
            let n=sqlx::query("UPDATE backuppolicies SET archivedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND controlstate='Idle'").bind(id).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if n == 0 {
                Err(BackupError::Conflict(
                    "Backup Policy is active or missing.".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
    fn enqueue_backup(
        &self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
    ) -> BoxFuture<'_, Result<BackupRunView, BackupError>> {
        let trigger = trigger.to_owned();
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let p=sqlx::query("SELECT p.name,p.source,p.backuprepositoryid,p.enabled,p.controlstate,p.runasactorid,r.type FROM backuppolicies p JOIN backuprepositories r ON r.id=p.backuprepositoryid WHERE p.id=$1 AND p.archivedat IS NULL AND r.archivedat IS NULL FOR UPDATE OF p,r").bind(policy_id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(BackupError::NotFound)?;
            if !p.try_get::<bool, _>("enabled").map_err(storage)?
                || p.try_get::<String, _>("controlstate").map_err(storage)? != "Idle"
            {
                return Err(BackupError::Conflict(
                    "Backup Policy is disabled or already active.".into(),
                ));
            }
            let id = Uuid::now_v7();
            let repo: Uuid = p.try_get("backuprepositoryid").map_err(storage)?;
            sqlx::query("INSERT INTO backupruns(id,backuppolicyid,policynamesnapshot,backuprepositoryid,repositorytypesnapshot,sourcesnapshot,trigger,status,snapshotavailability,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,'Queued','Pending',$8)").bind(id).bind(policy_id).bind(p.try_get::<String,_>("name").map_err(storage)?).bind(repo).bind(p.try_get::<String,_>("type").map_err(storage)?).bind(p.try_get::<Value,_>("source").map_err(storage)?).bind(trigger).bind(actor.value()).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE backuppolicies SET controlstate='Processing',currentrunid=$2,controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1").bind(policy_id).bind(id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            self.get_run(id).await
        })
    }
    fn list_scheduled_policies(&self) -> BoxFuture<'_, Result<Vec<BackupPolicyView>, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuppolicies WHERE enabled=true AND archivedat IS NULL AND cron IS NOT NULL ORDER BY id")
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_policy)
                .collect()
        })
    }
    fn enqueue_scheduled_backup(
        &self,
        policy_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let Some(policy) = sqlx::query("SELECT p.name,p.source,p.backuprepositoryid,p.controlstate,p.lastscheduledrunat,p.runasactorid,r.type FROM backuppolicies p JOIN backuprepositories r ON r.id=p.backuprepositoryid WHERE p.id=$1 AND p.enabled=true AND p.archivedat IS NULL AND p.cron IS NOT NULL AND r.archivedat IS NULL FOR UPDATE OF p,r")
                .bind(policy_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
            else {
                tx.rollback().await.map_err(storage)?;
                return Ok(false);
            };
            let state: String = policy.try_get("controlstate").map_err(storage)?;
            let last: Option<DateTime<Utc>> =
                policy.try_get("lastscheduledrunat").map_err(storage)?;
            if state != "Idle" || last.is_some_and(|value| value >= scheduled_minute) {
                tx.rollback().await.map_err(storage)?;
                return Ok(false);
            }
            let id = Uuid::now_v7();
            let repository_id: Uuid = policy.try_get("backuprepositoryid").map_err(storage)?;
            let run_as_actor_id: Uuid = policy.try_get("runasactorid").map_err(storage)?;
            sqlx::query("INSERT INTO backupruns(id,backuppolicyid,policynamesnapshot,backuprepositoryid,repositorytypesnapshot,sourcesnapshot,trigger,status,snapshotavailability,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,'Schedule','Queued','Pending',$7)")
                .bind(id)
                .bind(policy_id)
                .bind(policy.try_get::<String, _>("name").map_err(storage)?)
                .bind(repository_id)
                .bind(policy.try_get::<String, _>("type").map_err(storage)?)
                .bind(policy.try_get::<Value, _>("source").map_err(storage)?)
                .bind(run_as_actor_id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let updated = sqlx::query("UPDATE backuppolicies SET controlstate='Processing',currentrunid=$2,controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,lastscheduledrunat=$3,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND controlstate='Idle'")
                .bind(policy_id)
                .bind(id)
                .bind(scheduled_minute)
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
            if updated != 1 {
                tx.rollback().await.map_err(storage)?;
                return Ok(false);
            }
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }
    fn claim_backup(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<BackupClaim>, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            recover_stale(&mut tx, stale_before).await?;
            let row=sqlx::query("SELECT r.id,r.backuppolicyid,r.backuprepositoryid,p.timeoutseconds,p.source FROM backupruns r JOIN backuppolicies p ON p.id=r.backuppolicyid JOIN backuprepositories br ON br.id=r.backuprepositoryid WHERE r.status='Queued' AND p.archivedat IS NULL AND br.archivedat IS NULL AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.backuprepositoryid AND l.expiresat>CURRENT_TIMESTAMP) ORDER BY r.queuedat,r.id FOR UPDATE OF r,p,br SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                tx.commit().await.map_err(storage)?;
                return Ok(None);
            };
            let id: Uuid = row.try_get("id").map_err(storage)?;
            let policy_id: Uuid = row.try_get("backuppolicyid").map_err(storage)?;
            let repo_id: Uuid = row.try_get("backuprepositoryid").map_err(storage)?;
            let timeout: i32 = row.try_get("timeoutseconds").map_err(storage)?;
            let source: Value = row.try_get("source").map_err(storage)?;
            let source_key = backup_source_key(&source)?;
            let leased = sqlx::query("INSERT INTO backuprepositoryleases(backuprepositoryid,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Backup',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+make_interval(secs=>$3*2+300)) ON CONFLICT(backuprepositoryid) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Backup',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backuprepositoryleases.expiresat<=CURRENT_TIMESTAMP").bind(repo_id).bind(id).bind(timeout).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let source_leased = sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Backup',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+make_interval(secs=>$3*2+300)) ON CONFLICT(sourcekey) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Backup',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backupsourceleases.expiresat<=CURRENT_TIMESTAMP")
                .bind(source_key).bind(id).bind(timeout).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if source_leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let claimed = sqlx::query(
                "UPDATE backupruns SET status='Running',startedat=CURRENT_TIMESTAMP WHERE id=$1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?
            .rows_affected();
            if claimed != 1 {
                return Err(BackupError::Storage(
                    "Backup Run could not be claimed atomically.".into(),
                ));
            }
            sqlx::query("UPDATE backuprepositories SET controlstate='Processing',currentrunid=$2,controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1").bind(repo_id).bind(id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(BackupClaim {
                policy: self.get_policy(policy_id).await?,
                repository: self.get_repository(repo_id).await?,
                run: self.get_run(id).await?,
            }))
        })
    }
    fn prepare_backup_items<'a>(
        &'a self,
        claim: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            if plan.items.is_empty() && plan.local_directory.is_none() {
                return Err(BackupError::Validation(format!(
                    "{} has no Docker named Volumes to back up.",
                    plan.display_name
                )));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let status = sqlx::query_scalar::<_, String>(
                "SELECT status FROM backupruns WHERE id=$1 FOR UPDATE",
            )
            .bind(claim.run.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)?;
            if status != "Running" {
                tx.rollback().await.map_err(storage)?;
                return Err(BackupError::Conflict(
                    "Backup Run is no longer active.".into(),
                ));
            }
            let existing = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM backuprunitems WHERE backuprunid=$1",
            )
            .bind(claim.run.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if existing != 0 {
                tx.rollback().await.map_err(storage)?;
                return Err(BackupError::Conflict(
                    "Backup Run already has an immutable source plan.".into(),
                ));
            }
            let aggregate_key = backup_source_key(&claim.run.source_snapshot)?;
            for item in &plan.items {
                let source_key = docker_volume_key(
                    item.platform_id,
                    item.docker_node_id.as_deref(),
                    &item.volume_name,
                );
                if source_key != aggregate_key {
                    let leased = sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Backup',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+make_interval(secs=>$3*2+300)) ON CONFLICT(sourcekey) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Backup',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backupsourceleases.expiresat<=CURRENT_TIMESTAMP")
                        .bind(&source_key)
                        .bind(claim.run.id)
                        .bind(claim.policy.timeout_seconds)
                        .execute(&mut *tx)
                        .await
                        .map_err(storage)?
                        .rows_affected();
                    if leased == 0 {
                        tx.rollback().await.map_err(storage)?;
                        return Err(BackupError::Conflict(format!(
                            "Backup source {source_key} already has an active operation."
                        )));
                    }
                }
                sqlx::query("INSERT INTO backuprunitems(id,backuprunid,platformid,volumename,dockernodeid,nodehostname,status) VALUES($1,$2,$3,$4,$5,$6,'Queued')")
                    .bind(item.id)
                    .bind(claim.run.id)
                    .bind(item.platform_id)
                    .bind(&item.volume_name)
                    .bind(&item.docker_node_id)
                    .bind(&item.node_hostname)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            let mut warnings = claim.run.warnings.clone();
            warnings.extend(plan.warnings.iter().cloned());
            warnings.sort();
            warnings.dedup();
            sqlx::query("UPDATE backupruns SET warnings=$2 WHERE id=$1 AND status='Running'")
                .bind(claim.run.id)
                .bind(sqlx::types::Json(warnings))
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn finish_backup<'a>(
        &'a self,
        claim: &'a BackupClaim,
        result: &'a BackupExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let finished = sqlx::query("UPDATE backupruns SET status=$2,snapshotavailability=$3,resticsnapshotid=$4,parentsnapshotid=$5,filesprocessed=$6,bytesprocessed=$7,bytesadded=$8,exitcode=$9,errorcode=$10,errormessage=$11,warnings=$12,completedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Running'").bind(claim.run.id).bind(result.status).bind(result.snapshot_availability).bind(&result.restic_snapshot_id).bind(&result.parent_snapshot_id).bind(result.files_processed).bind(result.bytes_processed).bind(result.bytes_added).bind(result.exit_code).bind(&result.error_code).bind(&result.error_message).bind(sqlx::types::Json(&result.warnings)).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if finished == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(());
            }
            for item in &result.items {
                sqlx::query("UPDATE backuprunitems SET status=$3,resticsnapshotid=$4,parentsnapshotid=$5,filesprocessed=$6,bytesprocessed=$7,bytesadded=$8,exitcode=$9,errorcode=$10,errormessage=$11,startedat=COALESCE(startedat,CURRENT_TIMESTAMP),completedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND backuprunid=$2 AND status IN ('Queued','Running')")
                    .bind(item.id)
                    .bind(claim.run.id)
                    .bind(item.status)
                    .bind(&item.restic_snapshot_id)
                    .bind(&item.parent_snapshot_id)
                    .bind(item.files_processed)
                    .bind(item.bytes_processed)
                    .bind(item.bytes_added)
                    .bind(item.exit_code)
                    .bind(&item.error_code)
                    .bind(&item.error_message)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            sqlx::query("UPDATE backuprunitems SET status='Failed',errorcode=COALESCE($2,'BackupFailed'),errormessage=COALESCE($3,'Backup did not execute this planned Volume.'),completedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP WHERE backuprunid=$1 AND status IN ('Queued','Running')")
                .bind(claim.run.id)
                .bind(&result.error_code)
                .bind(&result.error_message)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            write_logs(
                &mut tx,
                "backuprunlogs",
                "backuprunid",
                claim.run.id,
                &result.logs,
            )
            .await?;
            if matches!(result.status, "Succeeded" | "SucceededWithWarnings") {
                sqlx::query("WITH retained AS (SELECT id FROM backupruns WHERE backuppolicyid=$1 AND snapshotavailability='Available' AND status IN ('Succeeded','SucceededWithWarnings') ORDER BY completedat DESC NULLS LAST,id DESC LIMIT $2) UPDATE backupruns SET snapshotavailability='Expired' WHERE backuppolicyid=$1 AND snapshotavailability='Available' AND status IN ('Succeeded','SucceededWithWarnings') AND id NOT IN (SELECT id FROM retained)")
                    .bind(claim.policy.id)
                    .bind(i64::from(claim.policy.keep_last_successful.max(1)))
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            release(
                &mut tx,
                claim.repository.id,
                claim.run.id,
                Some(claim.policy.id),
                matches!(result.status, "Succeeded" | "SucceededWithWarnings"),
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn list_runs(
        &self,
        actor: ActorId,
        administrator: bool,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRunView>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT run.* FROM backupruns run
WHERE ($4::uuid IS NULL OR run.backuppolicyid=$4)
  AND ($5 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=run.backuppolicyid
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY run.queuedat DESC,run.id DESC LIMIT $6"#
            );
            let mut runs = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(READ_MASK)
                .bind(policy_id)
                .bind(administrator)
                .bind(i64::try_from(limit.clamp(1, 1000)).unwrap_or(1000))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_run)
                .collect::<Result<Vec<_>, _>>()?;
            attach_run_items(&self.pool, &mut runs).await?;
            Ok(runs)
        })
    }
    fn get_run(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRunView, BackupError>> {
        Box::pin(async move {
            let mut run = sqlx::query("SELECT * FROM backupruns WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BackupError::NotFound)
                .and_then(map_run)?;
            attach_run_items(&self.pool, std::slice::from_mut(&mut run)).await?;
            Ok(run)
        })
    }
    fn backup_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        logs(&self.pool, "backuprunlogs", "backuprunid", id)
    }
    fn cancel_backup(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row=sqlx::query("UPDATE backupruns SET status='Cancelled',completedat=CURRENT_TIMESTAMP,errorcode='Cancelled',errormessage='Backup cancelled.' WHERE id=$1 AND status='Queued' RETURNING backuprepositoryid,backuppolicyid").bind(id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if let Some(row) = row {
                release(
                    &mut tx,
                    row.try_get("backuprepositoryid").map_err(storage)?,
                    id,
                    Some(row.try_get("backuppolicyid").map_err(storage)?),
                    false,
                )
                .await?;
                tx.commit().await.map_err(storage)?;
                Ok(true)
            } else {
                tx.rollback().await.map_err(storage)?;
                Ok(false)
            }
        })
    }
    fn enqueue_restore(
        &self,
        request: BackupRestoreRequest,
    ) -> BoxFuture<'_, Result<BackupRestoreRunView, BackupError>> {
        let name = request.target_volume_name.trim().to_owned();
        Box::pin(async move {
            if name.is_empty() {
                return Err(BackupError::Validation(
                    "Target Volume name is required.".into(),
                ));
            }
            let source = self.get_run(request.backup_run_id).await?;
            if source.status != "Succeeded" || source.snapshot_availability != "Available" {
                return Err(BackupError::Conflict(
                    "Backup snapshot is not available for restore.".into(),
                ));
            }
            let selected_item = match (source.items.as_slice(), request.source_backup_run_item_id) {
                ([], None) => None,
                ([item], None) => Some(item),
                (_, Some(id)) => Some(source.items.iter().find(|item| item.id == id).ok_or_else(
                    || {
                        BackupError::Validation(
                            "Source Backup Run Item does not belong to this Backup Run.".into(),
                        )
                    },
                )?),
                (_, None) => {
                    return Err(BackupError::Validation(
                        "A source Backup Run Item is required for a multi-Volume backup.".into(),
                    ));
                }
            };
            if selected_item
                .is_some_and(|item| item.status != "Succeeded" || item.restic_snapshot_id.is_none())
            {
                return Err(BackupError::Conflict(
                    "The selected Backup Run Item has no available snapshot.".into(),
                ));
            }
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO backuprestoreruns(id,backuprunid,backuprepositoryid,sourcebackuprunitemid,targetplatformid,targetdockernodeid,targetvolumename,overwriteexisting,targetvolumecreatedbycitadel,status,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,false,'Queued',$9)").bind(id).bind(request.backup_run_id).bind(source.backup_repository_id).bind(selected_item.map(|item| item.id)).bind(request.target_platform_id).bind(request.target_docker_node_id).bind(name).bind(request.overwrite_existing).bind(request.actor.value()).execute(&self.pool).await.map_err(storage)?;
            self.get_restore(id).await
        })
    }
    fn claim_restore(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<RestoreClaim>, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            recover_stale(&mut tx, stale_before).await?;
            let row=sqlx::query("SELECT r.id,r.backuprunid,r.backuprepositoryid,r.sourcebackuprunitemid,r.targetplatformid,r.targetdockernodeid,r.targetvolumename FROM backuprestoreruns r JOIN backuprepositories br ON br.id=r.backuprepositoryid WHERE r.status='Queued' AND br.archivedat IS NULL AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.backuprepositoryid AND l.expiresat>CURRENT_TIMESTAMP) ORDER BY r.queuedat,r.id FOR UPDATE OF r,br SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                tx.commit().await.map_err(storage)?;
                return Ok(None);
            };
            let id: Uuid = row.try_get("id").map_err(storage)?;
            let source_id: Uuid = row.try_get("backuprunid").map_err(storage)?;
            let source_item_id: Option<Uuid> =
                row.try_get("sourcebackuprunitemid").map_err(storage)?;
            let repo_id: Uuid = row.try_get("backuprepositoryid").map_err(storage)?;
            let target_platform: Uuid = row.try_get("targetplatformid").map_err(storage)?;
            let target_node: Option<String> = row.try_get("targetdockernodeid").map_err(storage)?;
            let target_volume: String = row.try_get("targetvolumename").map_err(storage)?;
            let source_key =
                docker_volume_key(target_platform, target_node.as_deref(), &target_volume);
            let leased = sqlx::query("INSERT INTO backuprepositoryleases(backuprepositoryid,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Restore',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+INTERVAL '4 hours') ON CONFLICT(backuprepositoryid) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Restore',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backuprepositoryleases.expiresat<=CURRENT_TIMESTAMP").bind(repo_id).bind(id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let source_leased = sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Restore',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+INTERVAL '4 hours 5 minutes') ON CONFLICT(sourcekey) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Restore',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backupsourceleases.expiresat<=CURRENT_TIMESTAMP")
                .bind(source_key).bind(id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if source_leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let claimed = sqlx::query("UPDATE backuprestoreruns SET status='Running',startedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Queued'").bind(id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if claimed != 1 {
                return Err(BackupError::Storage(
                    "Backup Restore Run could not be claimed atomically.".into(),
                ));
            }
            sqlx::query("UPDATE backuprepositories SET controlstate='Processing',currentrunid=$2,controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1").bind(repo_id).bind(id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            let source = self.get_run(source_id).await?;
            let source_item = source_item_id
                .map(|id| {
                    source
                        .items
                        .iter()
                        .find(|item| item.id == id)
                        .cloned()
                        .ok_or_else(|| {
                            BackupError::Storage(
                                "Restore source Backup Run Item is missing.".into(),
                            )
                        })
                })
                .transpose()?;
            Ok(Some(RestoreClaim {
                repository: self.get_repository(repo_id).await?,
                run: self.get_restore(id).await?,
                source,
                source_item,
            }))
        })
    }
    fn finish_restore<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        result: &'a RestoreExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let finished = sqlx::query("UPDATE backuprestoreruns SET status=$2,exitcode=$3,errorcode=$4,errormessage=$5,completedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Running'").bind(claim.run.id).bind(result.status).bind(result.exit_code).bind(&result.error_code).bind(&result.error_message).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if finished == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(());
            }
            write_logs(
                &mut tx,
                "backuprestorerunlogs",
                "backuprestorerunid",
                claim.run.id,
                &result.logs,
            )
            .await?;
            release(&mut tx, claim.repository.id, claim.run.id, None, false).await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn list_restores(
        &self,
        actor: ActorId,
        administrator: bool,
        backup_run_id: Option<Uuid>,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRestoreRunView>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT restore.* FROM backuprestoreruns restore
JOIN backupruns backup ON backup.id=restore.backuprunid
WHERE ($4::uuid IS NULL OR restore.backuprunid=$4)
  AND ($5::uuid IS NULL OR backup.backuppolicyid=$5)
  AND ($6 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=backup.backuppolicyid
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY restore.queuedat DESC,restore.id DESC LIMIT $7"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(READ_MASK)
                .bind(backup_run_id)
                .bind(policy_id)
                .bind(administrator)
                .bind(i64::try_from(limit.clamp(1, 1000)).unwrap_or(1000))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_restore)
                .collect()
        })
    }
    fn get_restore(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRestoreRunView, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuprestoreruns WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BackupError::NotFound)
                .and_then(map_restore)
        })
    }
    fn restore_logs(&self, id: Uuid) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        logs(&self.pool, "backuprestorerunlogs", "backuprestorerunid", id)
    }
    fn cancel_restore(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async move {
            let row=sqlx::query("UPDATE backuprestoreruns SET status='Cancelled',completedat=CURRENT_TIMESTAMP,errorcode='Cancelled',errormessage='Restore cancelled.' WHERE id=$1 AND status='Queued' RETURNING backuprepositoryid").bind(id).fetch_optional(&self.pool).await.map_err(storage)?;
            Ok(row.is_some())
        })
    }
}

async fn recover_stale(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    before: DateTime<Utc>,
) -> Result<(), BackupError> {
    sqlx::query("UPDATE backupruns r SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='Interrupted',errormessage='Backup interrupted before completion.' FROM backuppolicies p WHERE p.id=r.backuppolicyid AND r.status IN('Preparing','Running','ApplyingRetention') AND r.startedat<$1 AND r.startedat+make_interval(secs=>p.timeoutseconds*2+300)<CURRENT_TIMESTAMP").bind(before).execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("UPDATE backuprestoreruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='Interrupted',errormessage='Restore interrupted before completion.' WHERE status IN('Preparing','Running') AND startedat<$1 AND startedat+INTERVAL '4 hours 5 minutes'<CURRENT_TIMESTAMP").bind(before).execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("DELETE FROM backuprepositoryleases WHERE expiresat<=CURRENT_TIMESTAMP OR ownerrunid IN(SELECT id FROM backupruns WHERE status='Interrupted') OR ownerrunid IN(SELECT id FROM backuprestoreruns WHERE status='Interrupted')").execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("DELETE FROM backupsourceleases WHERE expiresat<=CURRENT_TIMESTAMP OR ownerrunid IN(SELECT id FROM backupruns WHERE status='Interrupted') OR ownerrunid IN(SELECT id FROM backuprestoreruns WHERE status='Interrupted')").execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("UPDATE backuppolicies p SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE controlstate<>'Idle' AND NOT EXISTS(SELECT 1 FROM backupruns r WHERE r.id=p.currentrunid AND r.status IN('Queued','Preparing','Running','ApplyingRetention'))").execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("UPDATE backuprepositories r SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE controlstate<>'Idle' AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.id)").execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
async fn release(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repo: Uuid,
    run: Uuid,
    policy: Option<Uuid>,
    success: bool,
) -> Result<(), BackupError> {
    sqlx::query("DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2")
        .bind(repo)
        .bind(run)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    sqlx::query("DELETE FROM backupsourceleases WHERE ownerrunid=$1")
        .bind(run)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    sqlx::query("UPDATE backuprepositories SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(repo).bind(run).execute(&mut **tx).await.map_err(storage)?;
    if let Some(policy) = policy {
        sqlx::query("UPDATE backuppolicies SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,firstsuccessfulrunat=CASE WHEN $3 THEN COALESCE(firstsuccessfulrunat,CURRENT_TIMESTAMP) ELSE firstsuccessfulrunat END,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(policy).bind(run).bind(success).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}
async fn write_logs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    table: &str,
    column: &str,
    id: Uuid,
    logs: &[BackupLog],
) -> Result<(), BackupError> {
    for log in logs {
        let sql = match (table, column) {
            ("backuprunlogs", "backuprunid") => {
                "INSERT INTO backuprunlogs(id,backuprunid,createdat,stream,message) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4)"
            }
            ("backuprestorerunlogs", "backuprestorerunid") => {
                "INSERT INTO backuprestorerunlogs(id,backuprestorerunid,createdat,stream,message) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4)"
            }
            _ => return Err(BackupError::Storage("invalid Backup log target".into())),
        };
        sqlx::query(sql)
            .bind(Uuid::now_v7())
            .bind(id)
            .bind(&log.stream)
            .bind(&log.message)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
    }
    Ok(())
}
fn logs<'a>(
    pool: &'a PgPool,
    table: &str,
    column: &str,
    id: Uuid,
) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
    let sql = match (table, column) {
        ("backuprunlogs", "backuprunid") => {
            "SELECT stream,message FROM backuprunlogs WHERE backuprunid=$1 ORDER BY createdat,id"
        }
        ("backuprestorerunlogs", "backuprestorerunid") => {
            "SELECT stream,message FROM backuprestorerunlogs WHERE backuprestorerunid=$1 ORDER BY createdat,id"
        }
        _ => {
            return Box::pin(async {
                Err(BackupError::Storage("invalid Backup log target".into()))
            });
        }
    };
    Box::pin(async move {
        sqlx::query(sql)
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|r| {
                Ok(BackupLog {
                    stream: r.try_get("stream").map_err(storage)?,
                    message: r.try_get("message").map_err(storage)?,
                })
            })
            .collect()
    })
}
fn map_repository(r: sqlx::postgres::PgRow) -> Result<BackupRepositoryView, BackupError> {
    Ok(BackupRepositoryView {
        id: r.try_get("id").map_err(storage)?,
        name: r.try_get("name").map_err(storage)?,
        normalized_name: r.try_get("normalizedname").map_err(storage)?,
        description: r.try_get("description").map_err(storage)?,
        repository_type: r.try_get("type").map_err(storage)?,
        spec: r.try_get("spec").map_err(storage)?,
        password_secret_id: r.try_get("passwordsecretid").map_err(storage)?,
        status: r.try_get("status").map_err(storage)?,
        control_state: r.try_get("controlstate").map_err(storage)?,
        current_run_id: r.try_get("currentrunid").map_err(storage)?,
        control_started_at: r.try_get("controlstartedat").map_err(storage)?,
        last_pruned_at: r.try_get("lastprunedat").map_err(storage)?,
        last_checked_at: r.try_get("lastcheckedat").map_err(storage)?,
        created_by_actor_id: r.try_get("createdbyactorid").map_err(storage)?,
        created_at: r.try_get("createdat").map_err(storage)?,
        updated_at: r.try_get("updatedat").map_err(storage)?,
        archived_at: r.try_get("archivedat").map_err(storage)?,
        row_version: r.try_get("rowversion").map_err(storage)?,
    })
}
fn map_policy(r: sqlx::postgres::PgRow) -> Result<BackupPolicyView, BackupError> {
    Ok(BackupPolicyView {
        id: r.try_get("id").map_err(storage)?,
        name: r.try_get("name").map_err(storage)?,
        normalized_name: r.try_get("normalizedname").map_err(storage)?,
        description: r.try_get("description").map_err(storage)?,
        source: r.try_get("source").map_err(storage)?,
        backup_repository_id: r.try_get("backuprepositoryid").map_err(storage)?,
        enabled: r.try_get("enabled").map_err(storage)?,
        cron: r.try_get("cron").map_err(storage)?,
        time_zone: r.try_get("timezone").map_err(storage)?,
        webhook: r.try_get("webhook").map_err(storage)?,
        keep_last_successful: r.try_get("keeplastsuccessful").map_err(storage)?,
        timeout_seconds: r.try_get("timeoutseconds").map_err(storage)?,
        alert_on_failure: r.try_get("alertonfailure").map_err(storage)?,
        run_as_actor_id: r.try_get("runasactorid").map_err(storage)?,
        control_state: r.try_get("controlstate").map_err(storage)?,
        current_run_id: r.try_get("currentrunid").map_err(storage)?,
        last_scheduled_run_at: r.try_get("lastscheduledrunat").map_err(storage)?,
        first_successful_run_at: r.try_get("firstsuccessfulrunat").map_err(storage)?,
        created_by_actor_id: r.try_get("createdbyactorid").map_err(storage)?,
        created_at: r.try_get("createdat").map_err(storage)?,
        updated_at: r.try_get("updatedat").map_err(storage)?,
        archived_at: r.try_get("archivedat").map_err(storage)?,
        row_version: r.try_get("rowversion").map_err(storage)?,
    })
}
fn map_run(r: sqlx::postgres::PgRow) -> Result<BackupRunView, BackupError> {
    Ok(BackupRunView {
        id: r.try_get("id").map_err(storage)?,
        backup_policy_id: r.try_get("backuppolicyid").map_err(storage)?,
        policy_name_snapshot: r.try_get("policynamesnapshot").map_err(storage)?,
        backup_repository_id: r.try_get("backuprepositoryid").map_err(storage)?,
        repository_type_snapshot: r.try_get("repositorytypesnapshot").map_err(storage)?,
        source_snapshot: r.try_get("sourcesnapshot").map_err(storage)?,
        trigger: r.try_get("trigger").map_err(storage)?,
        status: r.try_get("status").map_err(storage)?,
        snapshot_availability: r.try_get("snapshotavailability").map_err(storage)?,
        restic_snapshot_id: r.try_get("resticsnapshotid").map_err(storage)?,
        parent_snapshot_id: r.try_get("parentsnapshotid").map_err(storage)?,
        files_processed: r.try_get("filesprocessed").map_err(storage)?,
        bytes_processed: r.try_get("bytesprocessed").map_err(storage)?,
        bytes_added: r.try_get("bytesadded").map_err(storage)?,
        warnings: r
            .try_get::<sqlx::types::Json<Vec<String>>, _>("warnings")
            .map_err(storage)?
            .0,
        queued_at: r.try_get("queuedat").map_err(storage)?,
        started_at: r.try_get("startedat").map_err(storage)?,
        completed_at: r.try_get("completedat").map_err(storage)?,
        exit_code: r.try_get("exitcode").map_err(storage)?,
        error_code: r.try_get("errorcode").map_err(storage)?,
        error_message: r.try_get("errormessage").map_err(storage)?,
        triggered_by_actor_id: r.try_get("triggeredbyactorid").map_err(storage)?,
        items: vec![],
    })
}

async fn attach_run_items(pool: &PgPool, runs: &mut [BackupRunView]) -> Result<(), BackupError> {
    if runs.is_empty() {
        return Ok(());
    }
    let ids = runs.iter().map(|run| run.id).collect::<Vec<_>>();
    let rows =
        sqlx::query("SELECT * FROM backuprunitems WHERE backuprunid=ANY($1) ORDER BY createdat,id")
            .bind(&ids)
            .fetch_all(pool)
            .await
            .map_err(storage)?;
    let mut grouped = HashMap::<Uuid, Vec<BackupRunItemView>>::new();
    for row in rows {
        let item = map_run_item(row)?;
        grouped.entry(item.backup_run_id).or_default().push(item);
    }
    for run in runs {
        run.items = grouped.remove(&run.id).unwrap_or_default();
    }
    Ok(())
}

fn map_run_item(row: sqlx::postgres::PgRow) -> Result<BackupRunItemView, BackupError> {
    Ok(BackupRunItemView {
        id: row.try_get("id").map_err(storage)?,
        backup_run_id: row.try_get("backuprunid").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        volume_name: row.try_get("volumename").map_err(storage)?,
        docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
        node_hostname: row.try_get("nodehostname").map_err(storage)?,
        status: row.try_get("status").map_err(storage)?,
        restic_snapshot_id: row.try_get("resticsnapshotid").map_err(storage)?,
        parent_snapshot_id: row.try_get("parentsnapshotid").map_err(storage)?,
        files_processed: row.try_get("filesprocessed").map_err(storage)?,
        bytes_processed: row.try_get("bytesprocessed").map_err(storage)?,
        bytes_added: row.try_get("bytesadded").map_err(storage)?,
        started_at: row.try_get("startedat").map_err(storage)?,
        completed_at: row.try_get("completedat").map_err(storage)?,
        exit_code: row.try_get("exitcode").map_err(storage)?,
        error_code: row.try_get("errorcode").map_err(storage)?,
        error_message: row.try_get("errormessage").map_err(storage)?,
    })
}
fn map_restore(r: sqlx::postgres::PgRow) -> Result<BackupRestoreRunView, BackupError> {
    Ok(BackupRestoreRunView {
        id: r.try_get("id").map_err(storage)?,
        backup_run_id: r.try_get("backuprunid").map_err(storage)?,
        backup_repository_id: r.try_get("backuprepositoryid").map_err(storage)?,
        source_backup_run_item_id: r.try_get("sourcebackuprunitemid").map_err(storage)?,
        target_platform_id: r.try_get("targetplatformid").map_err(storage)?,
        target_docker_node_id: r.try_get("targetdockernodeid").map_err(storage)?,
        target_volume_name: r.try_get("targetvolumename").map_err(storage)?,
        overwrite_existing: r.try_get("overwriteexisting").map_err(storage)?,
        status: r.try_get("status").map_err(storage)?,
        queued_at: r.try_get("queuedat").map_err(storage)?,
        started_at: r.try_get("startedat").map_err(storage)?,
        completed_at: r.try_get("completedat").map_err(storage)?,
        exit_code: r.try_get("exitcode").map_err(storage)?,
        error_code: r.try_get("errorcode").map_err(storage)?,
        error_message: r.try_get("errormessage").map_err(storage)?,
        triggered_by_actor_id: r.try_get("triggeredbyactorid").map_err(storage)?,
    })
}
fn backup_source_key(source: &Value) -> Result<String, BackupError> {
    let kind = source
        .get("$type")
        .and_then(Value::as_str)
        .ok_or_else(|| BackupError::Validation("Backup source type is missing.".into()))?;
    let uuid = |field: &str| {
        source
            .get(field)
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| BackupError::Validation(format!("Backup source '{field}' is invalid.")))
    };
    match kind {
        "DockerVolume" => {
            let platform = uuid("platformId")?;
            let volume = source
                .get("volumeName")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| BackupError::Validation("Backup Volume name is missing.".into()))?;
            Ok(docker_volume_key(
                platform,
                source.get("dockerNodeId").and_then(Value::as_str),
                volume,
            ))
        }
        "CitadelSystem" => Ok("citadel-system".into()),
        "Stack" => Ok(format!("stack:{}", uuid("stackId")?)),
        "Deployment" => Ok(format!("deployment:{}", uuid("deploymentId")?)),
        "SwarmService" => Ok(format!("swarm-service:{}", uuid("swarmServiceId")?)),
        _ => Err(BackupError::Validation(
            "Backup source type is unsupported.".into(),
        )),
    }
}
fn docker_volume_key(platform: Uuid, node: Option<&str>, volume: &str) -> String {
    match node.filter(|value| !value.is_empty()) {
        Some(node) => format!("{platform}:{node}:{volume}"),
        None => format!("{platform}:{volume}"),
    }
}
fn storage(e: impl std::fmt::Display) -> BackupError {
    BackupError::Storage(e.to_string())
}
fn backup_tag_error(error: resource_tags::ResourceTagError) -> BackupError {
    match error {
        resource_tags::ResourceTagError::Missing => {
            BackupError::Validation("One or more selected Tags do not exist.".to_owned())
        }
        resource_tags::ResourceTagError::Database(error) => storage(error),
    }
}
