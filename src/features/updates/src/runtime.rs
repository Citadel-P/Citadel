use crate::{Release, ReleaseChannel, UpdateError};
use futures_util::future::BoxFuture;

/// Transport access only; selection and persistence belong to the feature service.
pub trait ReleaseSource: Send + Sync {
    fn releases(&self, channel: ReleaseChannel)
    -> BoxFuture<'_, Result<Vec<Release>, UpdateError>>;
}

/// Resolve a completed official publication and verify its exact image digest.
pub trait ReleaseArtifactSource: Send + Sync {
    fn verify<'a>(
        &'a self,
        version: &'a str,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<crate::VerifiedReleaseArtifact, crate::UpdateError>>;
}
