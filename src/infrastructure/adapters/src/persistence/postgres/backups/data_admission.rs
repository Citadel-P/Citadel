use super::*;

pub struct PostgresBackupDataAdmission {
    pool: PgPool,
}
impl PostgresBackupDataAdmission {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl BackupDataAdmission for PostgresBackupDataAdmission {
    fn pending_cleanup(
        &self,
        after: Option<Uuid>,
    ) -> BoxFuture<'_, Result<Vec<(BackupDataOperation, BackupExecutionHost)>, BackupError>> {
        Box::pin(async move {
            let rows = sqlx::query("SELECT r.* FROM dataexecutorreservations r WHERE ($1::uuid IS NULL OR r.operationid>$1) AND (EXISTS(SELECT 1 FROM backupruns b WHERE b.id=r.ownerid AND b.status NOT IN ('Queued','Preparing','Running','ApplyingRetention')) OR EXISTS(SELECT 1 FROM backuprestoreruns b WHERE b.id=r.ownerid AND b.status NOT IN ('Queued','Preparing','Running')) OR (r.ownerid=r.attemptid AND EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.ownerrunid=r.ownerid AND l.expiresat<CURRENT_TIMESTAMP))) ORDER BY r.operationid LIMIT 16")
                .bind(after).fetch_all(&self.pool).await.map_err(storage)?;
            rows.into_iter()
                .map(|row| {
                    Ok((
                        BackupDataOperation {
                            id: row.try_get("operationid").map_err(storage)?,
                            repository_id: row.try_get("repositoryid").map_err(storage)?,
                            owner_id: row.try_get("ownerid").map_err(storage)?,
                            attempt_id: row.try_get("attemptid").map_err(storage)?,
                            platform_id: row.try_get("platformid").map_err(storage)?,
                            node_id: row.try_get("nodeid").map_err(storage)?,
                        },
                        BackupExecutionHost {
                            daemon_id: row.try_get("daemonid").map_err(storage)?,
                            authority_id: row.try_get("authorityid").map_err(storage)?,
                            generation: row
                                .try_get::<i64, _>("admissiongeneration")
                                .map_err(storage)? as u64,
                            data_slots: 1,
                            heavy_slots: 1,
                        },
                    ))
                })
                .collect()
        })
    }
    fn finish_cleanup<'a>(
        &'a self,
        operation: &'a BackupDataOperation,
        host: &'a BackupExecutionHost,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let backup: Option<Uuid> = sqlx::query_scalar("SELECT backuppolicyid FROM backupruns WHERE id=$1 AND attemptid=$2 AND status NOT IN ('Queued','Preparing','Running','ApplyingRetention') FOR UPDATE")
                .bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let restore: Option<Uuid> = sqlx::query_scalar("SELECT id FROM backuprestoreruns WHERE id=$1 AND attemptid=$2 AND status NOT IN ('Queued','Preparing','Running') FOR UPDATE")
                .bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if backup.is_none() && restore.is_none() && operation.owner_id != operation.attempt_id {
                return Ok(());
            }
            if backup.is_none() && restore.is_none() {
                let expired: Option<Uuid> = sqlx::query_scalar("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 AND expiresat<clock_timestamp() FOR UPDATE")
                    .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
                if expired.is_none() {
                    return Ok(());
                }
            }
            let removed = sqlx::query("DELETE FROM dataexecutorreservations WHERE operationid=$1 AND ownerid=$2 AND attemptid=$3 AND authorityid=$4 AND daemonid=$5 AND admissiongeneration=$6")
                .bind(operation.id).bind(operation.owner_id).bind(operation.attempt_id).bind(host.authority_id).bind(&host.daemon_id).bind(host.generation as i64).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            let remaining: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$1)",
            )
            .bind(operation.owner_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if removed == 1 && !remaining {
                let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3)")
                    .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).fetch_one(&mut *tx).await.map_err(storage)?;
                if owned {
                    release(
                        &mut tx,
                        operation.repository_id,
                        operation.owner_id,
                        backup,
                        false,
                    )
                    .await?;
                    let logs = [BackupLog { stream: "system".into(), message: "Executor confirmed helper cleanup. Host and storage reservations were released; this operation was not replayed.".into() }];
                    if backup.is_some() {
                        write_logs(
                            &mut tx,
                            "backuprunlogs",
                            "backuprunid",
                            operation.owner_id,
                            &logs,
                        )
                        .await?;
                    }
                    if restore.is_some() {
                        write_logs(
                            &mut tx,
                            "backuprestorerunlogs",
                            "backuprestorerunid",
                            operation.owner_id,
                            &logs,
                        )
                        .await?;
                    }
                }
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn reserve<'a>(
        &'a self,
        operation: &'a BackupDataOperation,
        host: &'a BackupExecutionHost,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        Box::pin(async move {
            if host.daemon_id.is_empty()
                || host.daemon_id.len() > 256
                || host.authority_id.is_nil()
                || host.generation == 0
                || host.generation > i64::MAX as u64
                || !(1..=64).contains(&host.data_slots)
                || !(1..=64).contains(&host.heavy_slots)
            {
                return Err(BackupError::Validation(
                    "Invalid execution host identity.".into(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let owned: Option<Uuid> = sqlx::query_scalar("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 FOR UPDATE")
                .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if owned.is_none() {
                return Err(BackupError::CleanupPending);
            }
            let live: bool = sqlx::query_scalar("SELECT expiresat>clock_timestamp() FROM backuprepositoryleases WHERE backuprepositoryid=$1")
                .bind(operation.repository_id).fetch_one(&mut *tx).await.map_err(storage)?;
            if !live {
                return Err(BackupError::CleanupPending);
            }
            sqlx::query("INSERT INTO buildexecutors(daemonid,authorityid,buildcapacity) VALUES($1,$2,1) ON CONFLICT(daemonid) DO NOTHING")
                .bind(&host.daemon_id).bind(host.authority_id).execute(&mut *tx).await.map_err(storage)?;
            let authority: Uuid = sqlx::query_scalar(
                "SELECT authorityid FROM buildexecutors WHERE daemonid=$1 FOR UPDATE",
            )
            .bind(&host.daemon_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            let (builds, data): (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM buildexecutorreservations WHERE daemonid=$1),(SELECT count(*) FROM dataexecutorreservations WHERE daemonid=$1)")
                .bind(&host.daemon_id).fetch_one(&mut *tx).await.map_err(storage)?;
            let automation: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM automationexecutorreservations WHERE daemonid=$1",
            )
            .bind(&host.daemon_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if (builds + data + automation > 0 && authority != host.authority_id)
                || data >= i64::from(host.data_slots)
                || builds + data + automation >= i64::from(host.heavy_slots)
            {
                return Ok(false);
            }
            sqlx::query(
                "UPDATE buildexecutors SET authorityid=$2,heavycapacity=$3 WHERE daemonid=$1",
            )
            .bind(&host.daemon_id)
            .bind(host.authority_id)
            .bind(host.heavy_slots as i32)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            sqlx::query("INSERT INTO dataexecutorreservations(operationid,repositoryid,ownerid,attemptid,daemonid,authorityid,admissiongeneration,platformid,nodeid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                .bind(operation.id).bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).bind(&host.daemon_id).bind(host.authority_id).bind(host.generation as i64).bind(operation.platform_id).bind(&operation.node_id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }
    fn release<'a>(
        &'a self,
        operation: &'a BackupDataOperation,
        host: &'a BackupExecutionHost,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM dataexecutorreservations WHERE operationid=$1 AND ownerid=$2 AND attemptid=$3 AND daemonid=$4 AND authorityid=$5 AND admissiongeneration=$6")
                .bind(operation.id).bind(operation.owner_id).bind(operation.attempt_id).bind(&host.daemon_id).bind(host.authority_id).bind(host.generation as i64).execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }
}
