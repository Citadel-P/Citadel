use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn pool_spec(pool: &PgPool, id: Uuid) -> Result<Option<Value>, sqlx::Error> {
    sqlx::query_scalar::<_, Value>(
        "SELECT providerspec FROM buildagentpools WHERE id=$1 AND enabled AND archivedat IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Disabling/archiving scheduling must not prevent cleanup of previous work.
pub(crate) async fn pool_cleanup_spec(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<Value>, sqlx::Error> {
    sqlx::query_scalar("SELECT providerspec FROM buildagentpools WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Registrations only; disabled pools may still be inspected by the updater.
pub(crate) async fn registered_pools(pool: &PgPool) -> Result<Vec<(Uuid, Value)>, sqlx::Error> {
    sqlx::query_as("SELECT id,providerspec FROM buildagentpools WHERE archivedat IS NULL")
        .fetch_all(pool)
        .await
}

pub(crate) async fn registered_pool(pool: &PgPool, id: Uuid) -> Result<Option<Value>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT providerspec FROM buildagentpools WHERE id=$1 AND archivedat IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
