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
