use futures_util::future::BoxFuture;
#[derive(Debug, thiserror::Error)]
#[error("Maintenance storage failed: {0}")]
pub struct MaintenanceError(pub String);
pub trait MaintenanceStore: Send + Sync {
    fn reconcile(&self) -> BoxFuture<'_, Result<Vec<&'static str>, MaintenanceError>>;
    fn expire_leases(&self) -> BoxFuture<'_, Result<(), MaintenanceError>>;
    fn cleanup(
        &self,
        build_retention_days: Option<i32>,
    ) -> BoxFuture<'_, Result<u64, MaintenanceError>>;
}
