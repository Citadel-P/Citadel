use citadel_backups::BackupError;
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn instance_id(pool: &PgPool) -> Result<Uuid, BackupError> {
    let mut transaction = pool.begin().await.map_err(storage)?;
    let generated = Uuid::now_v7();
    sqlx::query("INSERT INTO citadelinstanceidentity(id,instanceid,createdat) VALUES(1,$1,CURRENT_TIMESTAMP) ON CONFLICT(id) DO NOTHING")
        .bind(generated)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
    let id = sqlx::query_scalar("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage)?;
    transaction.commit().await.map_err(storage)?;
    Ok(id)
}

fn storage(error: sqlx::Error) -> BackupError {
    BackupError::Storage(error.to_string())
}
