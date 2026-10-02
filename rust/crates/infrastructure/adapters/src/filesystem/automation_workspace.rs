use super::cleanup::DirectoryCleanup;
use citadel_automation::workspace::{AutomationWorkspaceLease, AutomationWorkspacePort};
use futures_util::future::BoxFuture;
use std::{
    io,
    path::{Path, PathBuf},
};
pub struct LocalAutomationWorkspace {
    tasks: citadel_runtime::DynamicTasks,
}
impl LocalAutomationWorkspace {
    pub fn new(tasks: citadel_runtime::DynamicTasks) -> Self {
        Self { tasks }
    }
}
struct Workspace {
    directory: PathBuf,
    script: PathBuf,
    cleanup: DirectoryCleanup,
}
impl AutomationWorkspaceLease for Workspace {
    fn directory(&self) -> &Path {
        &self.directory
    }
    fn script(&self) -> &Path {
        &self.script
    }
    fn cleanup(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move { self.cleanup.cleanup().await.map_err(|e| e.to_string()) })
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
            let reservation = DirectoryCleanup::reserve(&self.tasks).map_err(|e| e.to_string())?;
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
            let path = directory.to_owned();
            let cleanup = tokio::task::spawn_blocking(move || {
                let mut builder = std::fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                builder.create(&path)?;
                Ok::<_, io::Error>(DirectoryCleanup::new(path, reservation))
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("Could not prepare run: {e}"))?;
            let workspace = Workspace {
                directory: directory.into(),
                script: directory.join("action.ts"),
                cleanup,
            };
            tokio::fs::write(&workspace.script, source.as_bytes())
                .await
                .map_err(|e| format!("Could not write action: {e}"))?;
            Ok(Box::new(workspace) as Box<dyn AutomationWorkspaceLease>)
        })
    }
}
