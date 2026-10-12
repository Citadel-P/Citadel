use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(crate) struct PullConfiguration {
    pub name: String,
    pub host: String,
    pub configuration: Value,
    pub status: String,
}

pub(crate) async fn load(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<PullConfiguration>, sqlx::Error> {
    let row =
        sqlx::query("SELECT name,registryhost,configuration,status FROM registries WHERE id=$1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    row.map(|row| {
        Ok(PullConfiguration {
            name: row.try_get("name")?,
            host: row.try_get("registryhost")?,
            configuration: row.try_get("configuration")?,
            status: row.try_get("status")?,
        })
    })
    .transpose()
}
