use super::*;

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
    let affected = sqlx::query("UPDATE backupruns r SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='Interrupted',errormessage='Backup interrupted before completion.' FROM backuppolicies p WHERE p.id=r.backuppolicyid AND r.status IN('Preparing','Running','ApplyingRetention') AND ($2 OR r.startedat<$1) AND ($2 OR r.startedat+make_interval(secs=>p.timeoutseconds*2+300)<CURRENT_TIMESTAMP)").bind(before).bind(startup).execute(&mut **tx).await.map_err(storage)?.rows_affected();
    let mut changed = affected;
    let affected = sqlx::query("UPDATE backuprestoreruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='Interrupted',errormessage='Restore interrupted before completion.' WHERE status IN('Preparing','Running') AND ($2 OR startedat<$1) AND ($2 OR startedat+INTERVAL '4 hours 5 minutes'<CURRENT_TIMESTAMP)").bind(before).bind(startup).execute(&mut **tx).await.map_err(storage)?.rows_affected();
    changed += affected;
    let _affected = sqlx::query("DELETE FROM backuprepositoryleases WHERE expiresat<=CURRENT_TIMESTAMP OR ownerrunid IN(SELECT id FROM backupruns WHERE status='Interrupted') OR ownerrunid IN(SELECT id FROM backuprestoreruns WHERE status='Interrupted')").execute(&mut **tx).await.map_err(storage)?.rows_affected();
    let _affected = sqlx::query("DELETE FROM backupsourceleases WHERE expiresat<=CURRENT_TIMESTAMP OR ownerrunid IN(SELECT id FROM backupruns WHERE status='Interrupted') OR ownerrunid IN(SELECT id FROM backuprestoreruns WHERE status='Interrupted')").execute(&mut **tx).await.map_err(storage)?.rows_affected();
    let affected = sqlx::query("UPDATE backuppolicies p SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE controlstate<>'Idle' AND NOT EXISTS(SELECT 1 FROM backupruns r WHERE r.id=p.currentrunid AND r.status IN('Queued','Preparing','Running','ApplyingRetention'))").execute(&mut **tx).await.map_err(storage)?.rows_affected();
    changed += affected;
    let affected = sqlx::query("UPDATE backuprepositories r SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE controlstate<>'Idle' AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.id)").execute(&mut **tx).await.map_err(storage)?.rows_affected();
    changed += affected;
    Ok(changed)
}
