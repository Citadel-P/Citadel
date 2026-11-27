use crate::MigrationError;
use citadel_primitives::postgres::require_supported;
use sqlx::{Connection, PgConnection};
use std::time::Duration;
use tokio::time::timeout;

const LIMIT: Duration = Duration::from_secs(5);

pub async fn admit_database(url: &str) -> Result<u32, MigrationError> {
    let mut connection = timeout(LIMIT, PgConnection::connect(url))
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    server_version(&mut connection).await
}

pub async fn server_version(connection: &mut PgConnection) -> Result<u32, MigrationError> {
    let raw: String = timeout(
        LIMIT,
        sqlx::query_scalar("SHOW server_version_num").fetch_one(connection),
    )
    .await
    .map_err(|_| unavailable())?
    .map_err(|_| unavailable())?;
    require_supported(raw.parse().map_err(|_| unavailable())?).map_err(MigrationError::Version)
}

fn unavailable() -> MigrationError {
    MigrationError::Version(
        "Cannot verify the connected PostgreSQL version. Citadel requires PostgreSQL 19.x.".into(),
    )
}
