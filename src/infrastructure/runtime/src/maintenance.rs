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
    /// Physical discovery has unexamined sources, queued candidates, or a
    /// productive completed scan whose remaining source-hours must be revisited.
    pub scan_pending: bool,
}

/// Discovery hints for one bounded worker invocation. Start fresh after every
/// pause so late captures and newly eligible intervals are considered again.
#[derive(Default)]
pub struct CompactionSweep {
    pub containers_exhausted: bool,
    pub platform_exhausted: bool,
    pub coarse_exhausted: bool,
}

pub trait MaintenanceStore: Send + Sync {
    fn reconcile(&self) -> BoxFuture<'_, Result<Vec<&'static str>, MaintenanceError>>;
    fn expire_leases(&self) -> BoxFuture<'_, Result<(), MaintenanceError>>;
    /// At most one container source-hour and one host/node-hour, or bounded discovery.
    fn compact(&self) -> BoxFuture<'_, Result<CompactionProgress, MaintenanceError>> {
        Box::pin(async { self.compact_next(&mut CompactionSweep::default()).await })
    }
    fn compact_next<'a>(
        &'a self,
        sweep: &'a mut CompactionSweep,
    ) -> BoxFuture<'a, Result<CompactionProgress, MaintenanceError>>;
    /// One retention tick, with bounded short batches and invocation-local progress.
    fn cleanup(
        &self,
        build_retention_days: Option<i32>,
    ) -> BoxFuture<'_, Result<u64, MaintenanceError>>;
}
