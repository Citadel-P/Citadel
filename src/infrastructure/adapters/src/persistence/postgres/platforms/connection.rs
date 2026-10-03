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
