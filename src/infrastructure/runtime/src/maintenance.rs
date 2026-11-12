use futures_util::future::BoxFuture;
#[derive(Debug, thiserror::Error)]
#[error("Maintenance storage failed: {0}")]
pub struct MaintenanceError(pub String);
/// Committed progress and an optional source-local input-budget stall.
/// A stall is not a database failure and must not erase physical-source progress.
#[derive(Debug, Default, Clone, Copy)]
pub struct CompactionProgress {
    pub processed: u64,
    pub stalled_source: Option<i64>,
}

pub trait MaintenanceStore: Send + Sync {
    fn reconcile(&self) -> BoxFuture<'_, Result<Vec<&'static str>, MaintenanceError>>;
    fn expire_leases(&self) -> BoxFuture<'_, Result<(), MaintenanceError>>;
    /// One bounded container source-hour and one bounded host/node-hour.
    fn compact(&self) -> BoxFuture<'_, Result<CompactionProgress, MaintenanceError>>;
    fn cleanup(
        &self,
        build_retention_days: Option<i32>,
    ) -> BoxFuture<'_, Result<u64, MaintenanceError>>;
}
