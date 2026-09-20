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
    let accepted = sqlx::query(
        r#"
INSERT INTO swarmnoderuntimeprojectionstates (
    platformid, dockernodeid, reconciliationstartedat, reconciliationcompletedat,
    lastsuccessfulreconciliationat, reconciliationgeneration, isstale,
    agentversion, dockerversion)
VALUES ($1, $2, $3, now(), now(), 1, false, $4, $5)
ON CONFLICT (platformid, dockernodeid) DO UPDATE
SET reconciliationstartedat=$3, reconciliationcompletedat=now(),
    lastsuccessfulreconciliationat=now(),
    reconciliationgeneration=swarmnoderuntimeprojectionstates.reconciliationgeneration+1,
    isstale=false, stalesince=NULL, stalereason=NULL,
    agentversion=$4, dockerversion=$5
WHERE swarmnoderuntimeprojectionstates.reconciliationstartedat IS NULL
   OR swarmnoderuntimeprojectionstates.reconciliationstartedat < $3
"#,
    )
    .bind(snapshot.platform_id)
    .bind(node_id)
    .bind(snapshot.observed_at)
    .bind(&snapshot.info.agent_version)
    .bind(&snapshot.info.server_version)
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if accepted == 0 {
        return Ok(());
    }

    crate::persistence::postgres::platforms::inventory::store::persist_containers(
        tx,
        snapshot,
        Some(node_id),
    )
    .await?;

    sqlx::query(
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
)
DELETE FROM swarmnodeimageprojections
WHERE platformid=$1 AND dockernodeid=$2 AND observedat < $4
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE value->>'id'=dockerimageid)
"#,
    )
    .bind(snapshot.platform_id)
    .bind(node_id)
    .bind(json(&snapshot.images)?)
    .bind(snapshot.observed_at)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    sqlx::query(
        r#"
WITH incoming AS (SELECT value FROM jsonb_array_elements($3::jsonb)), upserted AS (
    INSERT INTO swarmnodevolumeprojections (
        platformid, dockernodeid, volumename, resource, observedat, isstale)
    SELECT $1, $2, value->>'name', value, $4, false FROM incoming
    ON CONFLICT (platformid, dockernodeid, volumename) DO UPDATE
    SET resource=EXCLUDED.resource, observedat=EXCLUDED.observedat, isstale=false
    WHERE swarmnodevolumeprojections.observedat < $4
)
DELETE FROM swarmnodevolumeprojections
WHERE platformid=$1 AND dockernodeid=$2 AND observedat < $4
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE value->>'name'=volumename)
"#,
    )
    .bind(snapshot.platform_id)
    .bind(node_id)
    .bind(json(&snapshot.volumes)?)
    .bind(snapshot.observed_at)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;

    sqlx::query(
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
)
DELETE FROM swarmnodenetworkprojections
WHERE platformid=$1 AND dockernodeid=$2 AND observedat < $4
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE value->>'id'=dockernetworkid)
"#,
    )
    .bind(snapshot.platform_id)
    .bind(node_id)
    .bind(json(&snapshot.networks)?)
    .bind(snapshot.observed_at)
    .execute(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(())
}
