use crate::{Release, ReleaseChannel, UpdateError};
use futures_util::future::BoxFuture;

/// A prepared external executor and a one-way durable shutdown grant.
/// The implementation rechecks live ownership before publishing the grant.
pub trait UpdateHandoffRuntime: Send + Sync {
    fn journal<'a>(
        &'a self,
        operation: &'a crate::UpdateOperation,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<crate::UpdateJournal, UpdateError>>;

    fn acknowledged<'a>(
        &'a self,
        operation: &'a crate::UpdateOperation,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;
    fn authorize_shutdown<'a>(
        &'a self,
        operation: &'a crate::UpdateOperation,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;
}

pub trait UpdateInstallationRuntime: Send + Sync {
    fn source<'a>(
        &'a self,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<crate::InstallationSnapshot, UpdateError>>;
    fn launch<'a>(
        &'a self,
        operation: &'a crate::UpdateOperation,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<std::sync::Arc<dyn UpdateHandoffRuntime>, UpdateError>>;
}

/// Process-owned admission and detached work, independent of the requesting browser.
/// Closing never cancels admitted work. Resume requires a separate verified outcome.
pub trait UpdateDrainRuntime: Send + Sync {
    fn close(&self, fence: uuid::Uuid) -> Result<(), UpdateError>;
    fn blockers(&self, fence: uuid::Uuid) -> Result<Vec<crate::UpdateDrainBlocker>, UpdateError>;
}

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

/// Read the embedded descriptor from an already verified image in an isolated
/// runtime. Implementations must await cleanup even after cancellation.
pub trait ReleaseInspectionRuntime: Send + Sync {
    fn inspect<'a>(
        &'a self,
        artifact: &'a crate::VerifiedReleaseArtifact,
        architecture: &'a str,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<crate::UpgradeDescriptor, UpdateError>>;
}
