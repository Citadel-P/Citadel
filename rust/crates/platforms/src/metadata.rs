use futures_util::future::BoxFuture;
use uuid::Uuid;
#[derive(Debug, thiserror::Error)]
pub enum PlatformMetadataError {
    #[error("resource not found")]
    NotFound,
    #[error("storage failed: {0}")]
    Storage(String),
}
pub trait PlatformMetadataRepository: Send + Sync {
    fn update_platform_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), PlatformMetadataError>>;
}
