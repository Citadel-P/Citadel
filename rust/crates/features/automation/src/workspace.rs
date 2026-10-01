use futures_util::future::BoxFuture;
use std::path::Path;

/// A private, temporary script directory. Dropping it removes credential-bearing source.
pub trait AutomationWorkspaceLease: Send + Sync {
    fn directory(&self) -> &Path;
    fn script(&self) -> &Path;
    fn cleanup(&mut self) -> BoxFuture<'_, Result<(), String>>;
}
pub trait AutomationWorkspacePort: Send + Sync {
    fn prepare<'a>(
        &'a self,
        directory: &'a Path,
        cache: Option<&'a Path>,
        source: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn AutomationWorkspaceLease>, String>>;
}
