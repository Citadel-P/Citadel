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
        sqlx::query("SELECT * FROM instanceupdates WHERE ownsinstallation")
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .as_ref()
            .map(read_operation)
            .transpose()
    }

    pub(super) async fn operation_impl(&self, id: Uuid) -> Result<UpdateOperation, UpdateError> {
        let row = sqlx::query("SELECT * FROM instanceupdates WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(UpdateError::NotFound)?;
        read_operation(&row)
    }
}

/// Indexed claim columns and the serialized operation must describe one claim.
/// Callers retain their transaction/lock and choose whether released rows are allowed.
pub(super) fn read_operation(row: &sqlx::postgres::PgRow) -> Result<UpdateOperation, UpdateError> {
    let operation = row
        .try_get::<sqlx::types::Json<UpdateOperation>, _>("operation")
        .map_err(storage)?
        .0;
    operation.plan.validate(operation.created_at)?;
    if row.try_get::<Uuid, _>("id").map_err(storage)? != operation.id
        || row.try_get::<Uuid, _>("instanceid").map_err(storage)? != operation.instance_id
        || row.try_get::<Uuid, _>("actorid").map_err(storage)? != operation.actor_id.value()
        || row.try_get::<Uuid, _>("idempotencykey").map_err(storage)? != operation.idempotency_key
        || u64::try_from(row.try_get::<i64, _>("revision").map_err(storage)?).ok()
            != Some(operation.revision)
        || operation.id.is_nil()
        || operation.fence.is_nil()
        || operation.instance_id != operation.plan.instance_id
        || operation.actor_id.value().is_nil()
        || operation.idempotency_key.is_nil()
        || operation.updated_at < operation.created_at
    {
        return Err(UpdateError::Conflict(
            "Persisted update ownership changed or is invalid.".into(),
        ));
    }
    Ok(operation)
}
