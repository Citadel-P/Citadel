use citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore;
pub use sqlx::PgPool;
use std::sync::Arc;
pub fn projections(pool: &PgPool) -> Arc<PostgresInventoryProjectionStore> {
    Arc::new(PostgresInventoryProjectionStore::new(pool.clone()))
}
pub fn edge(pool: &PgPool) -> super::edge::EdgeObservationStores {
    super::edge::EdgeObservationStores {
        edge: Arc::new(
            citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore::new(
                pool.clone(),
            ),
        ),
        projections: projections(pool),
    }
}
pub fn metrics(
    pool: &PgPool,
) -> Arc<dyn citadel_alerts::platform_metrics::PlatformAlertBatchStore> {
    Arc::new(citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore::new(pool.clone(), env!("CITADEL_BUILD_VERSION")))
}
pub fn containers(
    pool: &PgPool,
) -> Arc<dyn citadel_platforms::jobs::background::ContainerMaintenanceReader> {
    Arc::new(citadel_adapters::persistence::postgres::platforms::background::PostgresContainerMaintenanceReader(pool.clone()))
}
pub fn stats(
    pool: &PgPool,
) -> (
    Arc<
        dyn citadel_platforms::stats_ingestion::StatsBatchStore<
                citadel_platforms::stats_ingestion::ContainerStatsSample,
            >,
    >,
    Arc<
        dyn citadel_platforms::stats_ingestion::StatsBatchStore<
                citadel_platforms::stats_ingestion::PlatformStatsSample,
            >,
    >,
) {
    let store = Arc::new(citadel_adapters::persistence::postgres::platforms::statistics::batch::PostgresStatsBatchStore::new(pool.clone()));
    (store.clone(), store)
}

pub async fn listener(
    pool: &PgPool,
    channel: &str,
) -> Result<
    citadel_adapters::persistence::postgres::notifications::PostgresNotifications,
    sqlx::Error,
> {
    citadel_adapters::persistence::postgres::notifications::PostgresNotifications::connect(
        pool, channel,
    )
    .await
}
pub async fn flush_pending(
    pool: &PgPool,
    alerts: &dyn citadel_alerts::AlertEventSink,
    batch_size: i64,
) -> Result<usize, citadel_alerts::AlertError> {
    metrics(pool).flush_pending(alerts, batch_size).await
}
pub async fn observe_platform_metrics(
    pool: &PgPool,
    alerts: &dyn citadel_alerts::AlertEventSink,
    platform_id: uuid::Uuid,
) {
    let store = citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore::new(pool.clone(), env!("CITADEL_BUILD_VERSION"));
    let _ = store.observe_latest(alerts, platform_id).await;
}
