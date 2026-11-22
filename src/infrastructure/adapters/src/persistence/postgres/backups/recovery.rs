use super::*;

/// Uncached hints only. Recovery includes orphaned control state even when both
/// queues are empty; valid leases still fence active work and retained storage.
pub(super) async fn claim_work(
    pool: &PgPool,
    before: DateTime<Utc>,
    restores: bool,
) -> Result<(bool, bool), BackupError> {
    sqlx::query_as(
        r#"SELECT
        CASE WHEN $2 THEN EXISTS(SELECT 1 FROM backuprestoreruns WHERE status='Queued')
             ELSE EXISTS(SELECT 1 FROM backupruns WHERE status='Queued') END,
        EXISTS(SELECT 1 FROM backupruns r
            WHERE r.status IN('Preparing','Running','ApplyingRetention') AND r.startedat<$1
            AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l
                WHERE l.ownerrunid=r.id AND l.attemptid=r.attemptid AND l.expiresat>CURRENT_TIMESTAMP))
        OR EXISTS(SELECT 1 FROM backuprestoreruns r
            WHERE r.status IN('Preparing','Running') AND r.startedat<$1
            AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l
                WHERE l.ownerrunid=r.id AND l.attemptid=r.attemptid AND l.expiresat>CURRENT_TIMESTAMP))
        OR EXISTS(SELECT 1 FROM backuppolicies p WHERE p.controlstate<>'Idle'
            AND NOT EXISTS(SELECT 1 FROM backupruns r WHERE r.id=p.currentrunid
                AND r.status IN('Queued','Preparing','Running','ApplyingRetention')))
        OR EXISTS(SELECT 1 FROM backuprepositories r WHERE r.controlstate<>'Idle'
            AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.id))"#,
    )
    .bind(before)
    .bind(restores)
    .fetch_one(pool)
    .await
    .map_err(storage)
}

pub(crate) async fn recover_stale(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    before: DateTime<Utc>,
) -> Result<u64, BackupError> {
    recover_stale_with_mode(tx, before, false).await
}

pub(crate) async fn recover_stale_with_mode(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    before: DateTime<Utc>,
    startup: bool,
) -> Result<u64, BackupError> {
    let interrupted = sqlx::query("UPDATE backupruns r SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='Interrupted',errormessage='Backup ownership was lost. Repository and source storage remain reserved until cleanup is verified.' FROM backuppolicies p WHERE p.id=r.backuppolicyid AND r.status IN('Preparing','Running','ApplyingRetention') AND ($2 OR r.startedat<$1) AND ($2 OR NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.ownerrunid=r.id AND l.attemptid=r.attemptid AND l.expiresat>CURRENT_TIMESTAMP)) RETURNING r.id").bind(before).bind(startup).fetch_all(&mut **tx).await.map_err(storage)?;
    let mut changed = interrupted.len() as u64;
    for row in interrupted {
        super::activity::record_run_activity(tx, row.try_get("id").map_err(storage)?).await?;
    }
    let affected = sqlx::query("UPDATE backuprestoreruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='Interrupted',errormessage='Restore ownership was lost. Repository and target storage remain reserved until cleanup is verified.' WHERE status IN('Preparing','Running') AND ($2 OR startedat<$1) AND ($2 OR NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.ownerrunid=backuprestoreruns.id AND l.attemptid=backuprestoreruns.attemptid AND l.expiresat>CURRENT_TIMESTAMP))").bind(before).bind(startup).execute(&mut **tx).await.map_err(storage)?.rows_affected();
    changed += affected;
    let affected = sqlx::query("UPDATE backuppolicies p SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE controlstate<>'Idle' AND NOT EXISTS(SELECT 1 FROM backupruns r WHERE r.id=p.currentrunid AND r.status IN('Queued','Preparing','Running','ApplyingRetention'))").execute(&mut **tx).await.map_err(storage)?.rows_affected();
    changed += affected;
    let affected = sqlx::query("UPDATE backuprepositories r SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE controlstate<>'Idle' AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.id)").execute(&mut **tx).await.map_err(storage)?.rows_affected();
    changed += affected;
    if changed > 0 {
        sqlx::query("SELECT pg_notify('citadel_resource_recovery','backup')")
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
    }
    Ok(changed)
}
