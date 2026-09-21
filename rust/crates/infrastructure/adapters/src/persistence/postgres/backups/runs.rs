use super::*;

impl PostgresBackupPersistence {
    pub(super) fn enqueue_run<'a>(
        &'a self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
        expected_webhook: Option<&'a Value>,
    ) -> BoxFuture<'a, Result<BackupRun, BackupError>> {
        let trigger = trigger.to_owned();
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let p=sqlx::query("SELECT p.name,p.source,p.backuprepositoryid,p.enabled,p.controlstate,p.runasactorid,p.webhook,r.type FROM backuppolicies p JOIN backuprepositories r ON r.id=p.backuprepositoryid WHERE p.id=$1 AND p.archivedat IS NULL AND r.archivedat IS NULL FOR UPDATE OF p,r").bind(policy_id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(BackupError::NotFound)?;
            if let Some(expected) = expected_webhook {
                let current: Option<Value> = p.try_get("webhook").map_err(storage)?;
                if current.as_ref() != Some(expected) {
                    return Err(BackupError::Conflict(
                        "Webhook configuration changed.".into(),
                    ));
                }
            }
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
            sqlx::query("SELECT pg_notify('citadel_backup_work','')")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("UPDATE backuppolicies SET controlstate='Processing',currentrunid=$2,controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1").bind(policy_id).bind(id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            self.get_run(id).await
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn platform_summaries_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<summaries::PlatformBackupSummary>, BackupError>> {
        Box::pin(async move {
            if platform_ids.is_empty() {
                return Ok(vec![]);
            }
            let query = format!(
                "{AUTHORIZED_CTE}{}",
                include_str!("backup_platform_summaries.sql")
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .bind(platform_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            rows.into_iter()
                .map(|row| {
                    Ok(summaries::PlatformBackupSummary {
                        platform_id: row.try_get("platformid").map_err(storage)?,
                        policy_count: row.try_get("policycount").map_err(storage)?,
                        enabled_policy_count: row.try_get("enabledpolicycount").map_err(storage)?,
                        docker_volume_policy_count: row
                            .try_get("dockervolumepolicycount")
                            .map_err(storage)?,
                        stack_policy_count: row.try_get("stackpolicycount").map_err(storage)?,
                        deployment_policy_count: row
                            .try_get("deploymentpolicycount")
                            .map_err(storage)?,
                        swarm_service_policy_count: row
                            .try_get("swarmservicepolicycount")
                            .map_err(storage)?,
                        attention_policy_count: row
                            .try_get("attentionpolicycount")
                            .map_err(storage)?,
                        last_run_status: row.try_get("lastrunstatus").map_err(storage)?,
                        last_run_at: row.try_get("lastrunat").map_err(storage)?,
                    })
                })
                .collect()
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn enqueue_backup_impl(
        &self,
        actor: ActorId,
        policy_id: Uuid,
        trigger: &str,
    ) -> BoxFuture<'_, Result<BackupRun, BackupError>> {
        self.enqueue_run(actor, policy_id, trigger, None)
    }
}

impl PostgresBackupPersistence {
    pub(super) fn enqueue_webhook_impl<'a>(
        &'a self,
        policy_id: Uuid,
        expected_webhook: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRun, BackupError>> {
        self.enqueue_run(
            ActorId::new(Uuid::from_u128(1)),
            policy_id,
            "Webhook",
            Some(expected_webhook),
        )
    }
}

impl PostgresBackupPersistence {
    pub(super) fn enqueue_scheduled_backup_impl(
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
            sqlx::query("SELECT pg_notify('citadel_backup_work','')")
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
}

impl PostgresBackupPersistence {
    pub(super) fn claim_backup_impl(
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
            let leased = sqlx::query("INSERT INTO backuprepositoryleases(backuprepositoryid,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Backup',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+make_interval(secs=>$3*2+$4)) ON CONFLICT(backuprepositoryid) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Backup',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backuprepositoryleases.expiresat<=CURRENT_TIMESTAMP").bind(repo_id).bind(id).bind(timeout).bind(self.repository_lease_seconds).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let source_leased = sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Backup',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+make_interval(secs=>$3*2+$4)) ON CONFLICT(sourcekey) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Backup',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backupsourceleases.expiresat<=CURRENT_TIMESTAMP")
                .bind(source_key).bind(id).bind(timeout).bind(self.source_lease_seconds).execute(&mut *tx).await.map_err(storage)?.rows_affected();
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
}

impl PostgresBackupPersistence {
    pub(super) fn prepare_backup_items_impl<'a>(
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
                    let leased = sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Backup',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+make_interval(secs=>$3*2+$4)) ON CONFLICT(sourcekey) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Backup',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backupsourceleases.expiresat<=CURRENT_TIMESTAMP")
                        .bind(&source_key)
                        .bind(claim.run.id)
                        .bind(claim.policy.timeout_seconds)
                        .bind(self.source_lease_seconds)
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
}

impl PostgresBackupPersistence {
    pub(super) fn finish_backup_impl<'a>(
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
}

impl PostgresBackupPersistence {
    pub(super) fn list_runs_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRun>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT run.* FROM backupruns run
WHERE ($4::uuid IS NULL OR run.backuppolicyid=$4)
  AND ($5 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=run.backuppolicyid
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY run.queuedat DESC,run.id DESC LIMIT $6"#
            );
            let mut runs = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
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
}

impl PostgresBackupPersistence {
    pub(super) fn get_run_impl(&self, id: Uuid) -> BoxFuture<'_, Result<BackupRun, BackupError>> {
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
}

impl PostgresBackupPersistence {
    pub(super) fn backup_logs_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        logs(&self.pool, "backuprunlogs", "backuprunid", id)
    }
}

impl PostgresBackupPersistence {
    pub(super) fn cancel_backup_impl(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
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
}
