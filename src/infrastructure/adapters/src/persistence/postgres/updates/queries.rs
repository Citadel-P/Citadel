use super::*;

/// Local updater diagnostic only. Never creates identity, initializes setup or
/// applies migrations; a registration must bind to an already initialized Core.
pub async fn read_instance_identity(pool: &PgPool) -> Result<Uuid, UpdateError> {
    let id: Option<Uuid> = sqlx::query_scalar("SELECT instanceid FROM citadelinstanceidentity WHERE id=1 AND EXISTS(SELECT 1 FROM instancesetupstates WHERE id=1 AND initializedat IS NOT NULL)")
        .fetch_optional(pool).await.map_err(storage)?;
    id.filter(|id| !id.is_nil()).ok_or_else(|| {
        UpdateError::Unavailable(
            "Citadel instance identity is unavailable or setup is incomplete.".into(),
        )
    })
}

impl PostgresUpdateRepository {
    pub(super) async fn active_impl(&self) -> Result<Option<UpdateOperation>, UpdateError> {
        Ok(sqlx::query_scalar::<_, sqlx::types::Json<UpdateOperation>>(
            "SELECT operation FROM instanceupdates WHERE ownsinstallation",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .map(|v| v.0))
    }

    pub(super) async fn operation_impl(&self, id: Uuid) -> Result<UpdateOperation, UpdateError> {
        Ok(sqlx::query_scalar::<_, sqlx::types::Json<UpdateOperation>>(
            "SELECT operation FROM instanceupdates WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or(UpdateError::NotFound)?
        .0)
    }
}
