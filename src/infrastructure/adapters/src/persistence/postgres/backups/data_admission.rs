use super::*;

pub struct PostgresBackupDataAdmission {
    pub(super) pool: PgPool,
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
            // Owners exist before the first reservation and after the last one.
            self.recover_storage_owners().await?;
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
            sqlx::query("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 FOR UPDATE").bind(operation.repository_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            confirm_cleanup(&mut tx, operation, host).await?;
            release_recovered_owner(
                &mut tx,
                operation.repository_id,
                operation.owner_id,
                operation.attempt_id,
                backup,
            )
            .await?;
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
            require_active_owner(&mut tx, operation.owner_id, operation.attempt_id).await?;
            let owned: Option<Uuid> = sqlx::query_scalar("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 FOR UPDATE")
                .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if owned.is_none() {
                return Err(BackupError::CleanupPending);
            }
            let live: bool = sqlx::query_scalar("SELECT expiresat>clock_timestamp() AND NOT recoveryfenced FROM backuprepositoryleases WHERE backuprepositoryid=$1")
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
            sqlx::query("UPDATE backuprepositoryleases SET cleanupconfirmed=false WHERE backuprepositoryid=$1")
                .bind(operation.repository_id).execute(&mut *tx).await.map_err(storage)?;
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
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Serialize the receipt with admission and owner recovery. Expired
            // ownership may still accept cleanup, but never new execution.
            sqlx::query("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 FOR UPDATE")
                .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            confirm_cleanup(&mut tx, operation, host).await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

// The run lock always precedes the lease lock, matching completion/renewal.
pub(super) async fn require_active_owner(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner: Uuid,
    attempt: Uuid,
) -> Result<(), BackupError> {
    if owner == attempt {
        return Ok(());
    }
    let backup: Option<String> =
        sqlx::query_scalar("SELECT status FROM backupruns WHERE id=$1 AND attemptid=$2 FOR UPDATE")
            .bind(owner)
            .bind(attempt)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;
    let restore: Option<String> = sqlx::query_scalar(
        "SELECT status FROM backuprestoreruns WHERE id=$1 AND attemptid=$2 FOR UPDATE",
    )
    .bind(owner)
    .bind(attempt)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    if backup.as_deref() != Some("Running") && restore.as_deref() != Some("Running") {
        return Err(BackupError::CleanupPending);
    }
    Ok(())
}
async fn confirm_cleanup(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operation: &BackupDataOperation,
    host: &BackupExecutionHost,
) -> Result<(), BackupError> {
    let removed = sqlx::query("DELETE FROM dataexecutorreservations WHERE operationid=$1 AND ownerid=$2 AND attemptid=$3 AND daemonid=$4 AND authorityid=$5 AND admissiongeneration=$6")
        .bind(operation.id).bind(operation.owner_id).bind(operation.attempt_id).bind(&host.daemon_id).bind(host.authority_id).bind(host.generation as i64).execute(&mut **tx).await.map_err(storage)?.rows_affected();
    if removed == 1 {
        sqlx::query("UPDATE backuprepositoryleases SET cleanupconfirmed=NOT coreworkactive AND NOT EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$2 AND attemptid=$3) WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3")
            .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}
async fn release_recovered_owner(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repository: Uuid,
    owner: Uuid,
    attempt: Uuid,
    policy: Option<Uuid>,
) -> Result<(), BackupError> {
    let safe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 AND cleanupconfirmed AND NOT coreworkactive AND NOT EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$2))")
        .bind(repository).bind(owner).bind(attempt).fetch_one(&mut **tx).await.map_err(storage)?;
    if safe {
        release(tx, repository, owner, policy, false).await?;
    }
    Ok(())
}
impl PostgresBackupDataAdmission {
    async fn recover_storage_owners(&self) -> Result<(), BackupError> {
        // Scan independently of reservations. The timestamp rotates unresolved
        // owners through bounded pages without ever expiring ownership.
        let rows: Vec<(Uuid, Uuid, Uuid)> = sqlx::query_as("SELECT backuprepositoryid,ownerrunid,attemptid FROM backuprepositoryleases l WHERE attemptid IS NOT NULL AND (EXISTS(SELECT 1 FROM backupruns b WHERE b.id=l.ownerrunid AND b.attemptid=l.attemptid AND b.status NOT IN ('Queued','Preparing','Running','ApplyingRetention')) OR EXISTS(SELECT 1 FROM backuprestoreruns b WHERE b.id=l.ownerrunid AND b.attemptid=l.attemptid AND b.status NOT IN ('Queued','Preparing','Running')) OR (ownerrunid=attemptid AND expiresat<clock_timestamp())) ORDER BY recoverycheckedat NULLS FIRST,backuprepositoryid LIMIT 16")
            .fetch_all(&self.pool).await.map_err(storage)?;
        for (repository, owner, attempt) in rows {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let backup: Option<Uuid> = sqlx::query_scalar("SELECT backuppolicyid FROM backupruns WHERE id=$1 AND attemptid=$2 AND status NOT IN ('Queued','Preparing','Running','ApplyingRetention') FOR UPDATE")
                .bind(owner).bind(attempt).fetch_optional(&mut *tx).await.map_err(storage)?;
            let restore: Option<Uuid> = sqlx::query_scalar("SELECT id FROM backuprestoreruns WHERE id=$1 AND attemptid=$2 AND status NOT IN ('Queued','Preparing','Running') FOR UPDATE")
                .bind(owner).bind(attempt).fetch_optional(&mut *tx).await.map_err(storage)?;
            if backup.is_none() && restore.is_none() && owner != attempt {
                continue;
            }
            let lease: Option<Uuid> = sqlx::query_scalar("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 FOR UPDATE")
                .bind(repository).bind(owner).bind(attempt).fetch_optional(&mut *tx).await.map_err(storage)?;
            if lease.is_none() {
                continue;
            }
            let changed=sqlx::query("UPDATE backuprepositoryleases SET recoveryfenced=true,recoverycheckedat=clock_timestamp() WHERE backuprepositoryid=$1 AND ($2<>$3 OR expiresat<clock_timestamp())")
                .bind(repository).bind(owner).bind(attempt).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed == 1 {
                release_recovered_owner(&mut tx, repository, owner, attempt, backup).await?;
            }
            tx.commit().await.map_err(storage)?;
        }
        Ok(())
    }
    pub(crate) async fn record_cleanup_pending(
        &self,
        operation: &BackupDataOperation,
        message: &str,
    ) -> Result<(), BackupError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        // Bounded, redacted infrastructure diagnostics; retries do not spam logs.
        sqlx::query("INSERT INTO backuprunlogs(id,backuprunid,createdat,stream,message) SELECT $1,id,clock_timestamp(),'system',$3 FROM backupruns WHERE id=$2 AND NOT EXISTS(SELECT 1 FROM backuprunlogs WHERE backuprunid=$2 AND stream='system' AND message=$3)")
            .bind(Uuid::now_v7()).bind(operation.owner_id).bind(message).execute(&mut *tx).await.map_err(storage)?;
        sqlx::query("INSERT INTO backuprestorerunlogs(id,backuprestorerunid,createdat,stream,message) SELECT $1,id,clock_timestamp(),'system',$3 FROM backuprestoreruns WHERE id=$2 AND NOT EXISTS(SELECT 1 FROM backuprestorerunlogs WHERE backuprestorerunid=$2 AND stream='system' AND message=$3)")
            .bind(Uuid::now_v7()).bind(operation.owner_id).bind(message).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        Ok(())
    }
    pub(crate) async fn begin_core_work(
        &self,
        repository: Uuid,
        owner: Uuid,
        attempt: Uuid,
    ) -> Result<(), BackupError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        require_active_owner(&mut tx, owner, attempt).await?;
        sqlx::query(
            "SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 FOR UPDATE",
        )
        .bind(repository)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?;
        let changed=sqlx::query("UPDATE backuprepositoryleases SET coreworkactive=true,cleanupconfirmed=false WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 AND NOT coreworkactive AND NOT recoveryfenced AND expiresat>clock_timestamp()")
            .bind(repository).bind(owner).bind(attempt).execute(&mut *tx).await.map_err(storage)?.rows_affected();
        if changed != 1 {
            return Err(BackupError::CleanupPending);
        }
        tx.commit().await.map_err(storage)?;
        Ok(())
    }
    pub(crate) async fn finish_core_work(
        &self,
        repository: Uuid,
        owner: Uuid,
        attempt: Uuid,
    ) -> Result<(), BackupError> {
        sqlx::query("UPDATE backuprepositoryleases SET coreworkactive=false,cleanupconfirmed=NOT EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$2 AND attemptid=$3) WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3")
            .bind(repository).bind(owner).bind(attempt).execute(&self.pool).await.map_err(storage)?;
        Ok(())
    }
}
