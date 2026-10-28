use crate::workspace_cleanup::DirectoryCleanup;
use citadel_scripts::workspace::{ScriptWorkspaceLease, ScriptWorkspacePort};
use futures_util::future::BoxFuture;
use std::{
    io,
    path::{Path, PathBuf},
};
pub struct LocalScriptWorkspace {
    tasks: citadel_runtime::DynamicTasks,
}
impl LocalScriptWorkspace {
    pub fn new(tasks: citadel_runtime::DynamicTasks) -> Self {
        Self { tasks }
    }
}
struct Workspace {
    directory: PathBuf,
    entrypoint: PathBuf,
    cleanup: DirectoryCleanup,
}
impl ScriptWorkspaceLease for Workspace {
    fn directory(&self) -> &Path {
        &self.directory
    }
    fn entrypoint(&self) -> &Path {
        &self.entrypoint
    }
    fn cleanup(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move { self.cleanup.cleanup().await.map_err(|e| e.to_string()) })
    }
}
impl ScriptWorkspacePort for LocalScriptWorkspace {
    fn prepare<'a>(
        &'a self,
        directory: &'a Path,
        cache: Option<&'a Path>,
        source: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn ScriptWorkspaceLease>, String>> {
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
                entrypoint: directory.join("bootstrap.ts"),
                cleanup,
            };
            tokio::fs::write(directory.join("script.ts"), source.as_bytes())
                .await
                .map_err(|e| format!("Could not write action: {e}"))?;
            // Deno's initial static module graph bypasses read permissions. Only
            // this fixed module may be the entrypoint. A computed dynamic import
            // checks the script AND its transitive imports against permissions.
            tokio::fs::write(
                &workspace.entrypoint,
                "const action = new URL('./script.ts', import.meta.url).href;\nawait import(action);\n",
            )
            .await
            .map_err(|e| format!("Could not write action bootstrap: {e}"))?;
            Ok(Box::new(workspace) as Box<dyn ScriptWorkspaceLease>)
        })
    }
}
