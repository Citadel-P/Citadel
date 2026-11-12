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
    Arc::new(citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore::new(pool.clone(), env!("CITADEL_BUILD_VERSION"), Arc::new(citadel_adapters::persistence::postgres::alerts::PostgresAlertRepository::new(pool.clone()))))
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
    batch_size: i64,
) -> Result<usize, citadel_alerts::AlertError> {
    metrics(pool).flush_pending(batch_size).await
}
pub async fn observe_platform_metrics(
    pool: &PgPool,
    alerts: &dyn citadel_alerts::AlertEventSink,
    platform_id: uuid::Uuid,
) {
    let store = citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore::new(pool.clone(), env!("CITADEL_BUILD_VERSION"), Arc::new(citadel_adapters::persistence::postgres::alerts::PostgresAlertRepository::new(pool.clone())));
    let _ = store.observe_latest(alerts, platform_id).await;
}

/// A scoped built-in-style rule keeps native metric tests independent of licensing.
pub async fn metric_rule(pool: &PgPool, platform: uuid::Uuid) -> uuid::Uuid {
    let rule = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO alertrules(id,name,type,severity,status,threshold,requiredmatches,createdbyactorid,limitedto,quiethours) VALUES($1,'metric fixture','PlatformCpuHigh','Critical','Enabled',1,1,'00000000-0000-0000-0000-000000000001',$2,'[]')")
        .bind(rule).bind(serde_json::json!([{"resourceId":platform,"resourceType":"Platform"}]))
        .execute(pool).await.unwrap();
    rule
}
