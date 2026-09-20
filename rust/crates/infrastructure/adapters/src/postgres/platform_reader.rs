use citadel_platforms::PlatformSummary;
use citadel_platforms::{AuthorizedPlatformReader, AuthorizedReadError};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use sqlx::PgPool;

const PLATFORM_RESOURCE_TYPE: i32 = 0;
const READ_PERMISSION_MASK: i32 = 1 | 2 | 4;

#[derive(Clone)]
pub struct PostgresAuthorizedPlatformReader {
    pool: PgPool,
}

impl PostgresAuthorizedPlatformReader {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AuthorizedPlatformReader for PostgresAuthorizedPlatformReader {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
    ) -> BoxFuture<'a, Result<Vec<PlatformSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            let rows = sqlx::query_file!(
                "queries/list_authorized_platforms.sql",
                actor_id.value(),
                PLATFORM_RESOURCE_TYPE,
                READ_PERMISSION_MASK,
                0_i32,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?;

            rows.into_iter()
                .map(|row| {
                    Ok(PlatformSummary {
                        id: row.id,
                        name: row.name,
                        address: row.address,
                        status: row.status,
                        connector_type: row.connectortype,
                    })
                })
                .collect()
        })
    }
}
