use citadel_primitives::{ActorId, ResourceType};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Known committed IDs use the same bounded cache as individual authorization.
pub(crate) async fn levels_for_resources(
    pool: &sqlx::PgPool,
    actor: ActorId,
    kind: ResourceType,
    ids: &[Uuid],
) -> Result<BTreeMap<Uuid, citadel_primitives::PermissionLevel>, sqlx::Error> {
    let grants = super::identity::authorization_cache::AuthorizationCache::attach(pool)
        .resources(pool, actor, kind, ids)
        .await
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    Ok(grants
        .into_iter()
        .map(|(id, grant)| {
            (
                id,
                grant.map_or(citadel_primitives::PermissionLevel::None, |grant| {
                    grant.level
                }),
            )
        })
        .collect())
}
