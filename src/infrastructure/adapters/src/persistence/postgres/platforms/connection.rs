//! Persisted routing metadata. Connector spellings and descriptor keys stay here.
use citadel_platforms::ConnectorKind;
use citadel_primitives::PlatformStatus;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(crate) struct PlatformConnection {
    pub id: Uuid,
    pub connector: ConnectorKind,
    pub address: String,
    pub status: PlatformStatus,
    pub node_id: Option<String>,
}

pub(crate) async fn load(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<PlatformConnection>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT connectortype,address,status,platformdescriptor FROM platforms WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(PlatformConnection {
            id,
            connector: super::classification::connector_kind(row.try_get("connectortype")?)?,
            address: row.try_get("address")?,
            status: row
                .try_get::<String, _>("status")?
                .parse()
                .map_err(|error: String| sqlx::Error::Decode(error.into()))?,
            node_id: super::descriptor::decode(row.try_get("platformdescriptor")?)?
                .routing
                .node_id,
        })
    })
    .transpose()
}

pub(crate) async fn node_stale(
    pool: &PgPool,
    platform: Uuid,
    node: &str,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT isstale FROM swarmnodeprojections WHERE platformid=$1 AND dockernodeid=$2",
    )
    .bind(platform)
    .bind(node)
    .fetch_optional(pool)
    .await
}

/// Raw execution connection; callers retain their existing availability policy.
pub(crate) async fn execution_target(
    pool: &PgPool,
    platform_id: Uuid,
) -> Result<Option<(String, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String, String)>(
        "SELECT connectortype,address,status FROM platforms WHERE id=$1",
    )
    .bind(platform_id)
    .fetch_optional(pool)
    .await
}

pub(crate) struct RegisteredTarget {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub connector: ConnectorKind,
    pub kind: citadel_platforms::PlatformKind,
}

pub(crate) async fn registered_targets(
    pool: &PgPool,
) -> Result<Vec<RegisteredTarget>, sqlx::Error> {
    let rows = sqlx::query("SELECT id,name,address,connectortype,COALESCE(platformdescriptor->>'$type','Docker') AS platformtype FROM platforms WHERE connectortype IN ('Local','Agent','EdgeAgent') ORDER BY id").fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(RegisteredTarget {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                address: row.try_get("address")?,
                connector: super::classification::connector_kind(row.try_get("connectortype")?)?,
                kind: super::classification::platform_kind(row.try_get("platformtype")?)?,
            })
        })
        .collect()
}

pub(crate) async fn swarm_identity(
    pool: &PgPool,
    platform: Uuid,
) -> Result<
    (Option<String>, citadel_platforms::PlatformRoutingMetadata),
    citadel_swarm_services::SwarmServiceError,
> {
    use citadel_swarm_services::SwarmServiceError;
    let storage = |error: sqlx::Error| SwarmServiceError::Storage(error.to_string());
    let pinned: (Option<String>, serde_json::Value) =
        sqlx::query_as("SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_optional(pool)
            .await
            .map_err(storage)?
            .ok_or(SwarmServiceError::NotFound)?;
    Ok((
        pinned.0,
        super::descriptor::decode(pinned.1)
            .map_err(storage)?
            .routing,
    ))
}
