use crate::{AgentUpdateRelease, AgentUpdateTarget};
use crate::{Release, ReleaseChannel, UpdateError};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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
    /// Advisory engine availability for status reads. Must not acquire update
    /// ownership or run installation preflight; review/approval call source().
    fn database_update<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<crate::DatabaseUpdatePlan>, UpdateError>> {
        Box::pin(async { Ok(None) })
    }

    /// Fence the old executor, verify the unchanged source and archive its evidence.
    fn recover_source<'a>(
        &'a self,
        operation: &'a crate::UpdateOperation,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;

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
    fn reopen(&self, fence: uuid::Uuid) -> Result<(), UpdateError>;
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

pub trait AgentUpdateRuntime: Send + Sync {
    /// Restore admission after an attempted rollout. Failure retains the durable operation
    /// so the worker can retry cleanup after reconnect, without repeating replacement.
    fn resume<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        operation: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;
    fn targets<'a>(
        &'a self,
        selected: Option<&'a [String]>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<AgentUpdateTarget>, UpdateError>>;
    fn release<'a>(
        &'a self,
        version: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentUpdateRelease, UpdateError>>;
    /// Owns bounded target drain, immutable replacement and authenticated reconnect verification.
    fn install<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        operation: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;
    /// A restarted worker observes an ambiguous attempt; it must never repeat replacement.
    fn verify<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        operation: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, UpdateError>>;
}

pub trait AgentUpdateEntitlements: Send + Sync {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, UpdateError>>;
}
