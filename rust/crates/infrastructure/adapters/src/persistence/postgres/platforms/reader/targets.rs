//! Narrow persisted identity reads; do not hydrate inventory to select a target.
use super::super::{classification, storage};
use citadel_platforms::{AuthorizedReadError, ImageIdentity, PlatformKind};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn platform_kind(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<PlatformKind>, AuthorizedReadError> {
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(platformdescriptor->>'$type','Docker') FROM platforms WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    kind.map(|kind| classification::platform_kind(&kind).map_err(storage))
        .transpose()
}
pub(super) async fn has_swarm_nodes(
    pool: &PgPool,
    platform: Uuid,
) -> Result<bool, AuthorizedReadError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1)")
        .bind(platform)
        .fetch_one(pool)
        .await
        .map_err(storage)
}
pub(super) async fn deployment_container_ids(
    pool: &PgPool,
    deployment: Uuid,
) -> Result<Vec<Uuid>, AuthorizedReadError> {
    sqlx::query_scalar("SELECT c.id FROM containers c JOIN deployments d ON d.id=c.deploymentid AND d.platformid=c.platformid WHERE d.id=$1 LIMIT 2")
        .bind(deployment).fetch_all(pool).await.map_err(storage)
}
pub(super) async fn stack_container_id(
    pool: &PgPool,
    stack: Uuid,
    reference: &str,
) -> Result<Option<Uuid>, AuthorizedReadError> {
    sqlx::query_scalar(
        "SELECT id FROM containers WHERE stackid=$1 AND (id=$2 OR dockercontainerid=$3)",
    )
    .bind(stack)
    .bind(Uuid::parse_str(reference).ok())
    .bind(reference)
    .fetch_optional(pool)
    .await
    .map_err(storage)
}
pub(super) async fn image_identity(
    pool: &PgPool,
    platform: Uuid,
    image: Uuid,
) -> Result<Option<ImageIdentity>, AuthorizedReadError> {
    let row: Option<(String, Option<String>, bool)> = sqlx::query_as("SELECT dockerimageid,NULL::text,false FROM images WHERE platformid=$1 AND id=$2 UNION ALL SELECT dockerimageid,dockernodeid,isstale FROM swarmnodeimageprojections WHERE platformid=$1 AND id=$2")
        .bind(platform).bind(image).fetch_optional(pool).await.map_err(storage)?;
    Ok(row.map(
        |(docker_image_id, docker_node_id, is_stale)| ImageIdentity {
            docker_image_id,
            docker_node_id,
            is_stale,
        },
    ))
}
pub(super) async fn image_registry_id(
    pool: &PgPool,
    platform: Uuid,
    docker_image: &str,
) -> Result<Option<Uuid>, AuthorizedReadError> {
    sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT registryid FROM images WHERE platformid=$1 AND dockerimageid=$2",
    )
    .bind(platform)
    .bind(docker_image)
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
    .map_err(storage)
}
