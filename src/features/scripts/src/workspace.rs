use futures_util::future::BoxFuture;
use std::path::Path;

/// A private, temporary script directory. Dropping it transfers removal of credential-bearing source to tracked cleanup.
pub trait ScriptWorkspaceLease: Send + Sync {
    fn directory(&self) -> &Path;
    /// Trusted entry module which dynamically loads the Script under Deno permissions.
    fn entrypoint(&self) -> &Path;
    fn cleanup(&mut self) -> BoxFuture<'_, Result<(), String>>;
}
pub trait ScriptWorkspacePort: Send + Sync {
    fn prepare<'a>(
        &'a self,
        directory: &'a Path,
        cache: Option<&'a Path>,
        source: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn ScriptWorkspaceLease>, String>>;
}
