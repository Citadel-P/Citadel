use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, RuntimeInventorySnapshot};
use serde_json::Value;
use sqlx::PgPool;

/// Commit a post-mutation inventory only while the platform still identifies the
/// same cluster and manager. The network requests happen before this short lock.
pub async fn refresh(
    pool: &PgPool,
    snapshot: &RuntimeInventorySnapshot,
    policy: &citadel_platforms::node_agents::NodeAgentReconciliationPolicy,
) -> Result<(), RuntimeCapabilityError> {
    let storage = |error: sqlx::Error| {
        RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
    };
    let metadata =
        ProjectionWrite::begin(snapshot.platform_id, None, ProjectionKind::Platform).await;
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
        &super::super::descriptor::decode(saved.1)
            .map_err(storage)?
            .routing,
    ) {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Conflict,
            "The connected Docker manager no longer belongs to this platform.",
            false,
        ));
    }
    crate::persistence::postgres::platforms::inventory::resource::persist_swarm_snapshot(
        &mut tx, snapshot, policy,
    )
    .await?;
    tx.commit().await.map_err(storage)?;
    metadata.committed();
    Ok(())
}
