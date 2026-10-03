use citadel_platforms::{RuntimeCapabilityError, RuntimeInventorySnapshot};
use sqlx::{Postgres, Transaction};

use crate::persistence::postgres::platforms::inventory::store::json;
use crate::persistence::postgres::platforms::inventory::store::storage;

/// Called inside the authenticated session's transaction. The persisted scan
/// watermark also fences late snapshots after a resource has been deleted.
pub(crate) async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    node_id: &str,
) -> Result<(), RuntimeCapabilityError> {
    if !persist_metadata(
        tx,
        snapshot.platform_id,
        node_id,
        snapshot.observed_at,
        &snapshot.info,
        true,
    )
    .await?
    {
        return Ok(());
    }

    crate::persistence::postgres::platforms::inventory::store::persist_containers(
        tx,
        snapshot,
        Some(node_id),
    )
    .await?;

    persist_images(
        tx,
        snapshot.platform_id,
        node_id,
        snapshot.observed_at,
        &snapshot.images,
    )
    .await?;
    persist_volumes(
        tx,
        snapshot.platform_id,
        node_id,
        snapshot.observed_at,
        &snapshot.volumes,
    )
    .await?;
    persist_networks(
        tx,
        snapshot.platform_id,
        node_id,
        snapshot.observed_at,
        &snapshot.networks,
    )
    .await?;
    Ok(())
}

pub(crate) async fn persist_metadata(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    info: &citadel_platforms::RuntimePlatformInfo,
    complete: bool,
) -> Result<bool, RuntimeCapabilityError> {
    let accepted = sqlx::query(
        r#"
INSERT INTO swarmnoderuntimeprojectionstates (
    platformid, dockernodeid, reconciliationstartedat, reconciliationcompletedat,
    lastsuccessfulreconciliationat, reconciliationgeneration, isstale,
    agentversion, dockerversion)
VALUES ($1, $2, $3, CASE WHEN $6 THEN now() END, CASE WHEN $6 THEN now() END, 1, NOT $6, $4, $5)
ON CONFLICT (platformid, dockernodeid) DO UPDATE
SET reconciliationstartedat=$3, reconciliationcompletedat=CASE WHEN $6 THEN now() END,
    lastsuccessfulreconciliationat=CASE WHEN $6 THEN now() ELSE swarmnoderuntimeprojectionstates.lastsuccessfulreconciliationat END,
    reconciliationgeneration=swarmnoderuntimeprojectionstates.reconciliationgeneration+1,
    isstale=NOT $6, stalesince=CASE WHEN $6 THEN NULL ELSE now() END, stalereason=CASE WHEN $6 THEN NULL ELSE 'Recovery in progress' END,
    agentversion=$4, dockerversion=$5
WHERE swarmnoderuntimeprojectionstates.reconciliationstartedat IS NULL
   OR swarmnoderuntimeprojectionstates.reconciliationstartedat < $3
"#,
    )
    .bind(platform_id)
    .bind(node_id)
    .bind(observed_at)
    .bind(&info.agent_version)
    .bind(&info.server_version)
    .bind(complete)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    Ok(accepted != 0)
}

pub(crate) async fn persist_images(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    images: &[citadel_platforms::RuntimeImageSummary],
) -> Result<bool, RuntimeCapabilityError> {
    persist_images_observations(tx, platform_id, node_id, observed_at, images, true).await
}

pub(crate) async fn persist_images_observations(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    images: &[citadel_platforms::RuntimeImageSummary],
    complete: bool,
) -> Result<bool, RuntimeCapabilityError> {
    crate::persistence::postgres::platforms::inventory::store::validate_runtime_ids(
        images.iter().map(|value| value.id.as_str()),
    )?;
    let changed: bool = sqlx::query_scalar(
        r#"
WITH incoming AS (SELECT value FROM jsonb_array_elements($3::jsonb)), upserted AS (
    INSERT INTO swarmnodeimageprojections (
        id, platformid, dockernodeid, dockerimageid, contentidentity, resource, observedat, isstale)
    SELECT gen_random_uuid(), $1, $2, value->>'id',
           COALESCE((SELECT min(digest) FROM jsonb_array_elements_text(value->'repo_digests') digest
                     WHERE digest <> ''), value->>'id'), value, $4, false
    FROM incoming
    ON CONFLICT (platformid, dockernodeid, dockerimageid) DO UPDATE
    SET contentidentity=EXCLUDED.contentidentity, resource=EXCLUDED.resource,
        observedat=EXCLUDED.observedat, isstale=false
    WHERE swarmnodeimageprojections.observedat < $4
      AND (swarmnodeimageprojections.resource::jsonb IS DISTINCT FROM EXCLUDED.resource::jsonb OR swarmnodeimageprojections.isstale)
    RETURNING dockerimageid
), deleted AS (
DELETE FROM swarmnodeimageprojections
WHERE $5::boolean AND platformid=$1 AND dockernodeid=$2 AND observedat < $4
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE value->>'id'=dockerimageid)
RETURNING dockerimageid
)
SELECT EXISTS(SELECT 1 FROM upserted) OR EXISTS(SELECT 1 FROM deleted)
"#,
    )
    .bind(platform_id)
    .bind(node_id)
    .bind(json(images)?)
    .bind(observed_at)
    .bind(complete)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;

    // Retain ordering without reporting freshness-only writes as resource changes.
    sqlx::query("UPDATE swarmnodeimageprojections SET observedat=$4 WHERE platformid=$1 AND dockernodeid=$2 AND observedat<$4 AND dockerimageid IN (SELECT value->>'id' FROM jsonb_array_elements($3::jsonb) value)")
        .bind(platform_id).bind(node_id).bind(json(images)?).bind(observed_at)
        .execute(&mut **tx).await.map_err(storage)?;
    Ok(changed)
}

pub(crate) async fn persist_volumes(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    volumes: &[citadel_platforms::RuntimeVolumeSummary],
) -> Result<bool, RuntimeCapabilityError> {
    persist_volumes_observations(tx, platform_id, node_id, observed_at, volumes, true).await
}

pub(crate) async fn persist_volumes_observations(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    volumes: &[citadel_platforms::RuntimeVolumeSummary],
    complete: bool,
) -> Result<bool, RuntimeCapabilityError> {
    crate::persistence::postgres::platforms::inventory::store::validate_runtime_ids(
        volumes.iter().map(|value| value.name.as_str()),
    )?;
    let changed: bool = sqlx::query_scalar(
        r#"
WITH incoming AS (SELECT value FROM jsonb_array_elements($3::jsonb)), upserted AS (
    INSERT INTO swarmnodevolumeprojections (
        platformid, dockernodeid, volumename, resource, observedat, isstale)
    SELECT $1, $2, value->>'name', value, $4, false FROM incoming
    ON CONFLICT (platformid, dockernodeid, volumename) DO UPDATE
    SET resource=EXCLUDED.resource, observedat=EXCLUDED.observedat, isstale=false
    WHERE swarmnodevolumeprojections.observedat < $4
      AND (swarmnodevolumeprojections.resource::jsonb IS DISTINCT FROM EXCLUDED.resource::jsonb OR swarmnodevolumeprojections.isstale)
    RETURNING volumename
), deleted AS (
DELETE FROM swarmnodevolumeprojections
WHERE $5::boolean AND platformid=$1 AND dockernodeid=$2 AND observedat < $4
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE value->>'name'=volumename)
RETURNING volumename
)
SELECT EXISTS(SELECT 1 FROM upserted) OR EXISTS(SELECT 1 FROM deleted)
"#,
    )
    .bind(platform_id)
    .bind(node_id)
    .bind(json(volumes)?)
    .bind(observed_at)
    .bind(complete)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;

    // Retain ordering without reporting freshness-only writes as resource changes.
    sqlx::query("UPDATE swarmnodevolumeprojections SET observedat=$4 WHERE platformid=$1 AND dockernodeid=$2 AND observedat<$4 AND volumename IN (SELECT value->>'name' FROM jsonb_array_elements($3::jsonb) value)")
        .bind(platform_id).bind(node_id).bind(json(volumes)?).bind(observed_at)
        .execute(&mut **tx).await.map_err(storage)?;
    Ok(changed)
}

pub(crate) async fn persist_networks(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    networks: &[citadel_platforms::RuntimeNetworkSummary],
) -> Result<bool, RuntimeCapabilityError> {
    persist_networks_observations(tx, platform_id, node_id, observed_at, networks, true).await
}

pub(crate) async fn persist_networks_observations(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: &str,
    observed_at: chrono::DateTime<chrono::Utc>,
    networks: &[citadel_platforms::RuntimeNetworkSummary],
    complete: bool,
) -> Result<bool, RuntimeCapabilityError> {
    crate::persistence::postgres::platforms::inventory::store::validate_runtime_ids(
        networks.iter().map(|value| value.id.as_str()),
    )?;
    let changed: bool = sqlx::query_scalar(
        r#"
WITH incoming AS (
    SELECT value FROM jsonb_array_elements($3::jsonb)
    WHERE lower(COALESCE(value->>'scope', '')) <> 'swarm'
), upserted AS (
    INSERT INTO swarmnodenetworkprojections (
        platformid, dockernodeid, dockernetworkid, resource, observedat, isstale)
    SELECT $1, $2, value->>'id', value, $4, false FROM incoming
    ON CONFLICT (platformid, dockernodeid, dockernetworkid) DO UPDATE
    SET resource=EXCLUDED.resource, observedat=EXCLUDED.observedat, isstale=false
    WHERE swarmnodenetworkprojections.observedat < $4
      AND (swarmnodenetworkprojections.resource::jsonb IS DISTINCT FROM EXCLUDED.resource::jsonb OR swarmnodenetworkprojections.isstale)
    RETURNING dockernetworkid
), deleted AS (
DELETE FROM swarmnodenetworkprojections
WHERE $5::boolean AND platformid=$1 AND dockernodeid=$2 AND observedat < $4
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE value->>'id'=dockernetworkid)
RETURNING dockernetworkid
)
SELECT EXISTS(SELECT 1 FROM upserted) OR EXISTS(SELECT 1 FROM deleted)
"#,
    )
    .bind(platform_id)
    .bind(node_id)
    .bind(json(networks)?)
    .bind(observed_at)
    .bind(complete)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    // Retain ordering without reporting freshness-only writes as resource changes.
    sqlx::query("UPDATE swarmnodenetworkprojections SET observedat=$4 WHERE platformid=$1 AND dockernodeid=$2 AND observedat<$4 AND dockernetworkid IN (SELECT value->>'id' FROM jsonb_array_elements($3::jsonb) value)")
        .bind(platform_id).bind(node_id).bind(json(networks)?).bind(observed_at)
        .execute(&mut **tx).await.map_err(storage)?;
    Ok(changed)
}
