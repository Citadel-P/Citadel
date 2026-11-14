use crate::workspace_cleanup::DirectoryCleanup;
use citadel_scripts::workspace::{ScriptWorkspaceLease, ScriptWorkspacePort};
use futures_util::future::BoxFuture;
use std::{
    io,
    path::{Path, PathBuf},
};
pub struct LocalScriptWorkspace {
    tasks: citadel_runtime::DynamicTasks,
    sdk: Option<PathBuf>,
}
impl LocalScriptWorkspace {
    pub fn new(tasks: citadel_runtime::DynamicTasks) -> Self {
        Self { tasks, sdk: None }
    }
    /// Supply an asset location for embedded executors or disposable fixtures.
    /// The sandbox still excludes it from the writable workspace.
    pub fn with_sdk(tasks: citadel_runtime::DynamicTasks, sdk: PathBuf) -> Self {
        Self {
            tasks,
            sdk: Some(sdk),
        }
    }
}
struct Workspace {
    directory: PathBuf,
    entrypoint: PathBuf,
    sdk: PathBuf,
    import_map: PathBuf,
    cleanup: DirectoryCleanup,
}
impl ScriptWorkspaceLease for Workspace {
    fn directory(&self) -> &Path {
        &self.directory
    }
    fn entrypoint(&self) -> &Path {
        &self.entrypoint
    }
    fn sdk(&self) -> &Path {
        &self.sdk
    }
    fn import_map(&self) -> &Path {
        &self.import_map
    }
    fn cleanup(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move { self.cleanup.cleanup().await.map_err(|e| e.to_string()) })
    }
}
impl ScriptWorkspacePort for LocalScriptWorkspace {
    fn prepare<'a>(
        &'a self,
        directory: &'a Path,
        source: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn ScriptWorkspaceLease>, String>> {
        Box::pin(async move {
            let sdk = match &self.sdk {
                Some(path) => crate::sdk::resolve(path),
                None => crate::sdk::locate(),
            }
            .map_err(|e| format!("Could not locate script SDK: {e}"))?;
            let reservation = DirectoryCleanup::reserve(&self.tasks).map_err(|e| e.to_string())?;
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
            // Canonical paths keep both Deno's permissions and its import map
            // independent of the process working directory or parent symlinks.
            let directory = tokio::fs::canonicalize(directory)
                .await
                .map_err(|e| e.to_string())?;
            let workspace = Workspace {
                entrypoint: directory.join("bootstrap.ts"),
                import_map: directory.join("imports.json"),
                directory,
                sdk,
                cleanup,
            };
            let directory = &workspace.directory;
            if workspace.sdk.starts_with(directory) {
                return Err("Script SDK must be outside the writable run workspace".into());
            }
            for path in [directory, &workspace.sdk] {
                if path.to_str().is_none_or(|value| value.contains(',')) {
                    return Err(
                        "Script paths must be UTF-8 without commas for scoped Deno permissions"
                            .into(),
                    );
                }
            }
            let sdk_url = url::Url::from_file_path(&workspace.sdk)
                .map_err(|()| "Script SDK needs an absolute file path")?;
            tokio::fs::write(
                &workspace.import_map,
                serde_json::json!({"imports": {"citadel:sdk": sdk_url.as_str()}}).to_string(),
            )
            .await
            .map_err(|e| format!("Could not write script import map: {e}"))?;
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
