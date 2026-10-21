//! Filesystem ownership for Git caches and temporary credentials.
use futures_util::future::BoxFuture;
use std::{
    io,
    path::{Path, PathBuf},
};

pub trait GitCredentialFile: Send + Sync {
    fn path(&self) -> &Path;
}
pub trait GitStagingDirectory: Send + Sync {
    fn path(&self) -> &Path;
    fn publish<'a>(&'a mut self, target: &'a Path) -> BoxFuture<'a, io::Result<()>>;
    fn cleanup(&mut self) -> BoxFuture<'_, io::Result<()>>;
}
pub trait GitWorkspacePort: Send + Sync {
    /// Resolve an existing hook directory, rejecting paths outside the cache (including symlinks).
    fn hook_directory<'a>(
        &'a self,
        root: &'a Path,
        relative: &'a str,
    ) -> BoxFuture<'a, io::Result<PathBuf>>;
    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<bool>>;
    /// Like `Path::is_file`: follows links and returns false for missing/unreadable paths.
    fn has_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool>;
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
