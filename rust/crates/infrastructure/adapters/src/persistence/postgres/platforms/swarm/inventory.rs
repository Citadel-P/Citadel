use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, RuntimeInventorySnapshot};
use serde_json::Value;
use sqlx::PgPool;

/// Commit a post-mutation inventory only while the platform still identifies the
/// same cluster and manager. The network requests happen before this short lock.
pub async fn refresh(
    pool: &PgPool,
    snapshot: &RuntimeInventorySnapshot,
) -> Result<(), RuntimeCapabilityError> {
    let storage = |error: sqlx::Error| {
        RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
    };
    let mut tx = pool.begin().await.map_err(storage)?;
    let saved = sqlx::query_as::<_, (Option<String>, Value)>(
        "SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1 FOR NO KEY UPDATE",
    )
    .bind(snapshot.platform_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?
    .ok_or_else(|| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::NotFound,
            "Platform no longer exists.",
            false,
        )
    })?;
    if !citadel_platforms::swarm_mutations::manager_matches(
        &snapshot.info,
        saved.0.as_deref(),
        &saved.1,
    ) {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Conflict,
            "The connected Docker manager no longer belongs to this platform.",
            false,
        ));
    }
    crate::persistence::postgres::platforms::inventory::store::persist_snapshot(
        &mut tx, snapshot, None,
    )
    .await?;
    tx.commit().await.map_err(storage)
}
