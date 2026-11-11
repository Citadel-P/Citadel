use crate::{InstallationSettings, InstanceError, PatchInstallationSettings};
use chrono::{DateTime, Utc};
use citadel_identity::ActorPrincipal;
use futures_util::future::BoxFuture;

pub trait InstanceRepository: Send + Sync {
    fn installation_settings(&self) -> BoxFuture<'_, Result<InstallationSettings, InstanceError>>;
    fn patch_installation_settings<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        patch: &'a PatchInstallationSettings,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<InstallationSettings, InstanceError>>;
}

use uuid::Uuid;
pub trait InstanceRestartRepository: Send + Sync {
    fn request<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        boot_id: Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<(), InstanceError>>;
    fn pending(&self, boot_id: Uuid) -> BoxFuture<'_, Result<bool, InstanceError>>;
    fn complete(&self, boot_id: Uuid) -> BoxFuture<'_, Result<(), InstanceError>>;
}
