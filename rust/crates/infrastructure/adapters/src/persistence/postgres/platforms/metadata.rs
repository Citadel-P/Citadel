use super::*;
use citadel_platforms::{PlatformMetadataError, PlatformMetadataRepository};
#[derive(Clone)]
pub struct PostgresPlatformMetadataRepository {
    pool: PgPool,
}
impl PostgresPlatformMetadataRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl PlatformMetadataRepository for PostgresPlatformMetadataRepository {
    fn update_platform_description<'a>(
        &'a self,
        platform_id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), PlatformMetadataError>> {
        Box::pin(async move {
            let affected = sqlx::query("UPDATE platforms SET description=$2 WHERE id=$1")
                .bind(platform_id)
                .bind(description)
                .execute(&self.pool)
                .await
                .map_err(|e| PlatformMetadataError::Storage(e.to_string()))?
                .rows_affected();
            match affected {
                1 => Ok(()),
                0 => Err(PlatformMetadataError::NotFound),
                _ => Err(PlatformMetadataError::Storage(
                    "Mutation affected an unexpected number of rows.".into(),
                )),
            }
        })
    }
}
