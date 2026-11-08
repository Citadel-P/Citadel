use crate::persistence::postgres::identity::authorization_cache::AuthorizationCache;
use chrono::{DateTime, Utc};
use citadel_identity::ActorPrincipal;
use citadel_instance::{
    InstallationSettings, InstanceError, InstanceRepository, PatchInstallationSettings,
};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct PostgresInstanceRepository {
    pub(super) pool: PgPool,
    pub(super) authorization: Arc<AuthorizationCache>,
}
impl PostgresInstanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            authorization: AuthorizationCache::attach(&pool),
            pool,
        }
    }
}
impl InstanceRepository for PostgresInstanceRepository {
    fn installation_settings(&self) -> BoxFuture<'_, Result<InstallationSettings, InstanceError>> {
        Box::pin(super::settings::load(&self.pool))
    }
    fn patch_installation_settings<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        patch: &'a PatchInstallationSettings,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<InstallationSettings, InstanceError>> {
        Box::pin(super::settings::patch(self, principal, patch, now))
    }
}
