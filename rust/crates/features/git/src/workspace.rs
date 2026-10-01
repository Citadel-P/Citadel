//! Filesystem ownership for Git caches and temporary credentials.
use futures_util::future::BoxFuture;
use std::{io, path::Path};

pub trait GitCredentialFile: Send + Sync {
    fn path(&self) -> &Path;
}
pub trait GitStagingDirectory: Send + Sync {
    fn path(&self) -> &Path;
    fn publish<'a>(&'a mut self, target: &'a Path) -> BoxFuture<'a, io::Result<()>>;
    fn cleanup(&mut self) -> BoxFuture<'_, io::Result<()>>;
}
pub trait GitWorkspacePort: Send + Sync {
    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<bool>>;
    fn stage<'a>(
        &'a self,
        target: &'a Path,
    ) -> BoxFuture<'a, io::Result<Box<dyn GitStagingDirectory>>>;
    fn credential<'a>(
        &'a self,
        root: &'a Path,
        key: &'a str,
    ) -> BoxFuture<'a, io::Result<Box<dyn GitCredentialFile>>>;
}
