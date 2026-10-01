use citadel_automation::workspace::{AutomationWorkspaceLease, AutomationWorkspacePort};
use futures_util::future::BoxFuture;
use std::{
    io,
    path::{Path, PathBuf},
};
pub struct LocalAutomationWorkspace;
struct Workspace {
    directory: PathBuf,
    script: PathBuf,
    cleaned: bool,
}
impl AutomationWorkspaceLease for Workspace {
    fn directory(&self) -> &Path {
        &self.directory
    }
    fn script(&self) -> &Path {
        &self.script
    }
    fn cleanup(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            match tokio::fs::remove_dir_all(&self.directory).await {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
            self.cleaned = true;
            Ok(())
        })
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.cleaned
            && let Err(error) = std::fs::remove_dir_all(&self.directory)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "failed to remove Automation workspace");
        }
    }
}
impl AutomationWorkspacePort for LocalAutomationWorkspace {
    fn prepare<'a>(
        &'a self,
        directory: &'a Path,
        cache: Option<&'a Path>,
        source: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn AutomationWorkspaceLease>, String>> {
        Box::pin(async move {
            if let Some(cache) = cache {
                tokio::fs::create_dir_all(cache)
                    .await
                    .map_err(|e| format!("Could not prepare Deno cache: {e}"))?;
            }
            let parent = directory
                .parent()
                .ok_or("Could not prepare run: invalid directory")?;
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Could not prepare run: {e}"))?;
            // Create exclusively and privately: never follow an existing run-directory symlink.
            let mut builder = tokio::fs::DirBuilder::new();
            #[cfg(unix)]
            builder.mode(0o700);
            builder
                .create(directory)
                .await
                .map_err(|e| format!("Could not prepare run: {e}"))?;
            let workspace = Workspace {
                directory: directory.into(),
                script: directory.join("action.ts"),
                cleaned: false,
            };
            tokio::fs::write(&workspace.script, source.as_bytes())
                .await
                .map_err(|e| format!("Could not write action: {e}"))?;
            Ok(Box::new(workspace) as Box<dyn AutomationWorkspaceLease>)
        })
    }
}
