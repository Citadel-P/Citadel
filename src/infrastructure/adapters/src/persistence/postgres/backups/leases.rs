use super::*;

impl PostgresBackupPersistence {
    pub(super) fn renew_leases_impl(
        &self,
        repository: Uuid,
        owner: Uuid,
        attempt: Uuid,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Run -> repository lease -> sorted source leases is also the finish order.
            // Maintenance operations have no run and use their operation UUID as attempt.
            let backup: Option<String> = sqlx::query_scalar(
                "SELECT status FROM backupruns WHERE id=$1 AND attemptid=$2 FOR UPDATE",
            )
            .bind(owner)
            .bind(attempt)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
            let restore: Option<String> = sqlx::query_scalar(
                "SELECT status FROM backuprestoreruns WHERE id=$1 AND attemptid=$2 FOR UPDATE",
            )
            .bind(owner)
            .bind(attempt)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
            if owner != attempt
                && backup.as_deref() != Some("Running")
                && restore.as_deref() != Some("Running")
            {
                return Ok(false);
            }
            // Lock first, then check wall-clock expiry in a new statement.
            // A predicate evaluated before a lock wait is not expiry evidence.
            sqlx::query("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 FOR UPDATE")
                .bind(repository).bind(owner).bind(attempt).fetch_optional(&mut *tx).await.map_err(storage)?;
            let renewed = sqlx::query("UPDATE backuprepositoryleases SET expiresat=clock_timestamp()+make_interval(secs=>$4::integer) WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 AND expiresat>clock_timestamp()")
                .bind(repository).bind(owner).bind(attempt).bind(self.repository_lease_seconds).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if renewed != 1 {
                return Ok(false);
            }
            // Never revive an expired source lease, even if the repository still lives.
            sqlx::query("SELECT sourcekey FROM backupsourceleases WHERE ownerrunid=$1 ORDER BY sourcekey FOR UPDATE")
                .bind(owner).fetch_all(&mut *tx).await.map_err(storage)?;
            let (sources, expired): (i64, bool) = sqlx::query_as("SELECT count(*),coalesce(bool_or(attemptid IS DISTINCT FROM $2 OR expiresat<=clock_timestamp()),false) FROM backupsourceleases WHERE ownerrunid=$1")
                .bind(owner).bind(attempt).fetch_one(&mut *tx).await.map_err(storage)?;
            if expired {
                return Ok(false);
            }
            let renewed_sources = sqlx::query("UPDATE backupsourceleases SET expiresat=clock_timestamp()+make_interval(secs=>$3::integer) WHERE ownerrunid=$1 AND attemptid=$2 AND expiresat>clock_timestamp()")
                .bind(owner).bind(attempt).bind(self.source_lease_seconds).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if renewed_sources != sources as u64 {
                return Ok(false);
            }
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    pub(super) fn defer_backup_impl<'a>(
        &'a self,
        claim: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Only a proven pre-execution refusal can defer; any uncertain host
            // reservation prevents deleting the immutable item rows.
            let changed = sqlx::query("UPDATE backupruns SET status='Queued',startedat=NULL,nextattemptat=CURRENT_TIMESTAMP+INTERVAL '15 seconds',errorcode='CapacityBusy',errormessage='Waiting for source storage or execution host capacity.' WHERE id=$1 AND attemptid=$2 AND status='Running' AND NOT EXISTS(SELECT 1 FROM backuprunitems WHERE backuprunid=$1 AND status<>'Queued') AND NOT EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$1)")
                .bind(claim.run.id).bind(claim.attempt_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Ok(false);
            }
            sqlx::query("DELETE FROM backuprunitems WHERE backuprunid=$1 AND status='Queued'")
                .bind(claim.run.id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            // Keep policy.currentrunid: enqueue must not create a second run.
            release(&mut tx, claim.repository.id, claim.run.id, None, false).await?;
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn defer_restore_impl<'a>(
        &'a self,
        claim: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<bool, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let changed = sqlx::query("UPDATE backuprestoreruns SET status='Queued',startedat=NULL,nextattemptat=CURRENT_TIMESTAMP+INTERVAL '15 seconds',errorcode='HostBusy',errormessage='Waiting for an available execution host.' WHERE id=$1 AND attemptid=$2 AND status='Running' AND NOT EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$1)")
                .bind(claim.run.id).bind(claim.attempt_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Ok(false);
            }
            release(&mut tx, claim.repository.id, claim.run.id, None, false).await?;
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }
}
