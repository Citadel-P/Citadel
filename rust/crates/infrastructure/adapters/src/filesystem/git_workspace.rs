use citadel_git::workspace::{GitCredentialFile, GitStagingDirectory, GitWorkspacePort};
use futures_util::future::BoxFuture;
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub struct LocalGitWorkspace;
struct Credential(PathBuf);
impl GitCredentialFile for Credential {
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Credential {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.0)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "failed to delete temporary Git SSH key");
        }
    }
}
struct Staging(Option<PathBuf>);
impl GitStagingDirectory for Staging {
    fn path(&self) -> &Path {
        self.0.as_deref().expect("unpublished staging directory")
    }
    fn publish<'a>(&'a mut self, target: &'a Path) -> BoxFuture<'a, io::Result<()>> {
        Box::pin(async move {
            tokio::fs::rename(self.path(), target).await?;
            self.0 = None;
            Ok(())
        })
    }
    fn cleanup(&mut self) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move {
            if let Some(path) = &self.0 {
                match tokio::fs::remove_dir_all(path).await {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e),
                }
            }
            self.0 = None;
            Ok(())
        })
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            // Exceptional cancellation cleanup; ordinary completion uses async cleanup.
            if let Err(error) = std::fs::remove_dir_all(path)
                && error.kind() != io::ErrorKind::NotFound
            {
                tracing::warn!(%error, "failed to delete Git staging directory");
            }
        }
    }
}
impl GitWorkspacePort for LocalGitWorkspace {
    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<bool>> {
        Box::pin(tokio::fs::try_exists(path))
    }
    fn stage<'a>(
        &'a self,
        target: &'a Path,
    ) -> BoxFuture<'a, io::Result<Box<dyn GitStagingDirectory>>> {
        Box::pin(async move {
            let parent = target.parent().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Repository cache path must have a parent directory.",
                )
            })?;
            tokio::fs::create_dir_all(parent).await?;
            let name = target.file_name().unwrap_or_default().to_string_lossy();
            Ok(Box::new(Staging(Some(
                parent.join(format!(".{name}.clone-{}", Uuid::now_v7())),
            ))) as Box<dyn GitStagingDirectory>)
        })
    }
    fn credential<'a>(
        &'a self,
        root: &'a Path,
        key: &'a str,
    ) -> BoxFuture<'a, io::Result<Box<dyn GitCredentialFile>>> {
        Box::pin(async move {
            let directory = root.join(".credentials");
            tokio::fs::create_dir_all(&directory).await?;
            let path = directory.join(format!("{}.key", Uuid::now_v7()));
            if path.to_string_lossy().contains(['"', '\r', '\n']) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid credential path",
                ));
            }
            let mut options = tokio::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options.open(&path).await?;
            let guard = Credential(path);
            file.write_all(key.trim_end().as_bytes()).await?;
            file.write_all(b"\n").await?;
            file.flush().await?;
            Ok(Box::new(guard) as Box<dyn GitCredentialFile>)
        })
    }
}
