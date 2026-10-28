use super::data_admission::PostgresBackupDataAdmission;
use super::*;

pub(crate) struct LocalDataHelper {
    pub name: String,
    pub container_id: Option<String>,
    pub cleaned: bool,
}
impl PostgresBackupDataAdmission {
    pub(crate) async fn begin_helper(
        &self,
        operation: &BackupDataOperation,
        name: &str,
    ) -> Result<(), BackupError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        super::data_admission::require_active_owner(
            &mut tx,
            operation.owner_id,
            operation.attempt_id,
        )
        .await?;
        let lease: Option<Uuid> = sqlx::query_scalar("SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND attemptid=$3 FOR UPDATE")
            .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
        if lease.is_none() {
            return Err(BackupError::CleanupPending);
        }
        let inserted=sqlx::query("INSERT INTO backupdatahelpers(operationid,name) SELECT $1,$2 FROM dataexecutorreservations r JOIN backuprepositoryleases l ON l.backuprepositoryid=r.repositoryid AND l.ownerrunid=r.ownerid AND l.attemptid=r.attemptid WHERE r.operationid=$1 AND r.ownerid=$3 AND r.attemptid=$4 AND NOT l.recoveryfenced AND l.expiresat>clock_timestamp() AND (SELECT count(*) FROM backupdatahelpers WHERE operationid=$1)<32 ON CONFLICT DO NOTHING")
            .bind(operation.id).bind(name).bind(operation.owner_id).bind(operation.attempt_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
        if inserted != 1 {
            return Err(BackupError::CleanupPending);
        }
        tx.commit().await.map_err(storage)?;
        Ok(())
    }
    pub(crate) async fn register_helper(
        &self,
        operation: &BackupDataOperation,
        name: &str,
        id: &str,
    ) -> Result<(), BackupError> {
        let changed=sqlx::query("UPDATE backupdatahelpers SET containerid=$3 WHERE operationid=$1 AND name=$2 AND (containerid IS NULL OR containerid=$3) AND NOT cleaned")
            .bind(operation.id).bind(name).bind(id).execute(&self.pool).await.map_err(storage)?.rows_affected();
        if changed != 1 {
            return Err(BackupError::CleanupPending);
        }
        Ok(())
    }
    pub(crate) async fn authorize_helper_start(
        &self,
        operation: &BackupDataOperation,
        id: &str,
    ) -> Result<(), BackupError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        super::data_admission::require_active_owner(
            &mut tx,
            operation.owner_id,
            operation.attempt_id,
        )
        .await?;
        sqlx::query(
            "SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1 FOR UPDATE",
        )
        .bind(operation.repository_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?;
        let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprepositoryleases l JOIN backupdatahelpers h ON h.operationid=$4 WHERE l.backuprepositoryid=$1 AND l.ownerrunid=$2 AND l.attemptid=$3 AND NOT l.recoveryfenced AND l.expiresat>clock_timestamp() AND h.containerid=$5 AND NOT h.cleaned)")
            .bind(operation.repository_id).bind(operation.owner_id).bind(operation.attempt_id).bind(operation.id).bind(id).fetch_one(&mut *tx).await.map_err(storage)?;
        if !allowed {
            return Err(BackupError::CleanupPending);
        }
        tx.commit().await.map_err(storage)?;
        Ok(())
    }
    pub(crate) async fn local_helpers(
        &self,
        operation: &BackupDataOperation,
    ) -> Result<Vec<LocalDataHelper>, BackupError> {
        let rows=sqlx::query("SELECT name,containerid,cleaned FROM backupdatahelpers WHERE operationid=$1 ORDER BY name LIMIT 33").bind(operation.id).fetch_all(&self.pool).await.map_err(storage)?;
        if rows.len() > 32 {
            return Err(BackupError::CleanupPending);
        }
        rows.into_iter()
            .map(|r| {
                Ok(LocalDataHelper {
                    name: r.try_get("name").map_err(storage)?,
                    container_id: r.try_get("containerid").map_err(storage)?,
                    cleaned: r.try_get("cleaned").map_err(storage)?,
                })
            })
            .collect()
    }
    pub(crate) async fn helper_cleaned(
        &self,
        operation: &BackupDataOperation,
        name: &str,
    ) -> Result<(), BackupError> {
        sqlx::query("UPDATE backupdatahelpers SET cleaned=true WHERE operationid=$1 AND name=$2")
            .bind(operation.id)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(storage)?;
        Ok(())
    }
}
