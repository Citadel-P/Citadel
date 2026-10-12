//! Hosting diagnostics without exposing database driver handles to presentation.
use futures_util::future::BoxFuture;
#[derive(Clone, Copy)]
pub struct DatabasePoolUsage {
    pub connections: u32,
    pub idle: usize,
}
pub trait DatabaseHealth: Send + Sync {
    fn healthy(&self) -> BoxFuture<'_, bool>;
    fn pool_usage(&self) -> DatabasePoolUsage;
}
