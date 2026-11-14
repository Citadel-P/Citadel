//! Instance release-check service construction.
use citadel_adapters::persistence::postgres::updates::PostgresUpdateRepository;
use citadel_server::realtime::{RealtimeHub, notifiers::UpdatesRealtimeNotifier};
use citadel_updates::ReleaseCheckService;
use sqlx::PgPool;
use std::sync::Arc;

pub(crate) fn build(
    pool: &PgPool,
    realtime: Option<RealtimeHub>,
) -> Result<ReleaseCheckService, citadel_updates::UpdateError> {
    let checker = ReleaseCheckService::new(
        env!("CITADEL_BUILD_VERSION").to_owned(),
        super::releases::source()?,
        Arc::new(PostgresUpdateRepository::new(pool.clone())),
    )
    .with_notifier(Arc::new(UpdatesRealtimeNotifier::new(realtime)));
    #[cfg(target_os = "linux")]
    let checker = checker.with_database(Arc::new(
        citadel_adapters::external::updates::DockerDatabaseInstallation::new(
            Arc::new(citadel_processes::SystemProcess),
            pool.clone(),
        ),
    ));
    Ok(checker)
}
