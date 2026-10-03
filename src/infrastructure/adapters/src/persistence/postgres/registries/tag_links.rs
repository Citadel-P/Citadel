use super::*;
pub(crate) async fn validate_tag_ids(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), RegistryError> {
    crate::persistence::postgres::tags::validate_tag_ids(tx, ids)
        .await
        .map_err(tag_error)
}
pub(crate) async fn insert_resource_tags(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    t: TaggableResourceType,
    id: Uuid,
    ids: &[Uuid],
) -> Result<(), RegistryError> {
    crate::persistence::postgres::tags::insert_resource_tags(tx, actor, t, id, ids)
        .await
        .map_err(tag_error)
}
pub(crate) async fn replace_resource_tags_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    resource_type: TaggableResourceType,
    resource_id: Uuid,
    ids: &[Uuid],
) -> Result<(), RegistryError> {
    crate::persistence::postgres::tags::replace_resource_tags_tx(
        tx,
        actor,
        resource_type,
        resource_id,
        ids,
    )
    .await
    .map_err(tag_error)
}
fn tag_error(error: citadel_tags::TagError) -> RegistryError {
    match error {
        citadel_tags::TagError::Validation(e) => RegistryError::Validation(e),
        citadel_tags::TagError::NotFound => RegistryError::NotFound,
        citadel_tags::TagError::Conflict(e) => RegistryError::Conflict(e),
        citadel_tags::TagError::Storage(e) => RegistryError::Storage(e),
    }
}
