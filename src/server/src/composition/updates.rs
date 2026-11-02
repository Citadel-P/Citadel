//! Instance release-check service construction.
use citadel_adapters::{
    external::updates::GithubReleaseSource,
    persistence::postgres::updates::PostgresUpdateRepository,
};
use citadel_server::realtime::{RealtimeHub, notifiers::UpdatesRealtimeNotifier};
use citadel_updates::ReleaseCheckService;
use sqlx::PgPool;
use std::sync::Arc;

pub(super) fn build(pool: &PgPool, realtime: Option<RealtimeHub>) -> ReleaseCheckService {
    ReleaseCheckService::new(
        env!("CITADEL_BUILD_VERSION").to_owned(),
        Arc::new(GithubReleaseSource),
        Arc::new(PostgresUpdateRepository::new(pool.clone())),
    )
    .with_notifier(Arc::new(UpdatesRealtimeNotifier::new(realtime)))
}
